use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use parking_lot::Mutex;
use rusqlite::params;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use crate::error::AppResult;
use crate::services::path_utils::WorkspacePath;
use crate::db::DbPool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use camino::{Utf8Path, Utf8PathBuf};
use log::info;

pub struct ReconcilerActor;

#[derive(Debug, bon::Builder)]
struct FoundImage {
    path: Utf8PathBuf,
    board_abs_path: String,
    mtime: i64,
    size: i64,
    filename: String,
}

struct ImageDbMetadata {
    id: i32,
    mtime: i64,
    size: i64,
    _thumb_status: String,
    thumb_path: Option<String>,
}

struct ScanFinalizeParams<'a> {
    board_ids: &'a HashMap<String, i32>,
    folders: &'a [Utf8PathBuf],
    thumb_nudge_tx: &'a Sender<()>,
    new_pending: usize,
    paths_to_delete: Vec<String>,
}

impl ReconcilerActor {
    pub fn spawn<R: Runtime>(
        app: AppHandle<R>, 
        folders: Vec<Utf8PathBuf>,
        db: DbPool,
        thumb_nudge_tx: Sender<()>,
        cancel: Arc<AtomicBool>,
    ) -> AppResult<(Sender<()>, std::thread::JoinHandle<()>)> {
        let (tx, rx) = channel::<()>();
        let actor_tx = tx.clone();
        
        let handle = std::thread::spawn(move || {
            if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let state = app.state::<crate::commands::state::AppState>();
                let manager = &state.state_manager;

                loop {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }

                    if let Ok(()) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        while rx.try_recv().is_ok() {}

                        log::info!("Starting multi-folder reconciliation...");
                        manager.set_scanning(true);
                        manager.update_task("workspace-scan", "scan", "running", Some("Scanning library..."), 0, 0).ok();
                        
                        if let Err(e) = Self::reconcile(&app, &folders, &db, &thumb_nudge_tx, &cancel) {
                            log::error!("Reconcile failed: {}", e);
                            manager.finish_task("workspace-scan", "failed", Some("Reconciliation failed")).ok();
                        } else {
                            manager.finish_task("workspace-scan", "completed", None).ok();
                        }
                        manager.set_scanning(false);
                    }
                }
            })) {
                log::error!("ReconcilerActor PANICKED: {:?}", panic_info);
            }
        });

        Ok((actor_tx, handle))
    }

    fn reconcile<R: Runtime>(
        app: &AppHandle<R>, 
        folders: &[Utf8PathBuf],
        db: &DbPool,
        thumb_nudge_tx: &Sender<()>,
        cancel: &Arc<AtomicBool>,
    ) -> AppResult<()> {
        let state = app.state::<crate::commands::state::AppState>();
        let manager = &state.state_manager;

        // Phase 1: Discover and Upsert Boards
        let found_boards = Self::discover_boards(folders);
        let board_ids = Self::upsert_boards(app, db, &found_boards)?;

        // Phase 2: Parallel Image Walk
        let found_images = Self::walk_images(folders, cancel, manager)?;

        // Phase 3: Diff and Upsert Images
        let (new_pending, existing_images) = Self::diff_and_upsert_images(db, &board_ids, found_images)?;

        // Phase 4: Cleanup Missing Boards and Images
        let paths_to_delete = Self::cleanup_missing(db, folders, &found_boards, existing_images)?;

        // Phase 5: Finalize (Stats, Cache, Notifications)
        Self::finalize::<R>(app, db, manager, ScanFinalizeParams {
            board_ids: &board_ids,
            folders,
            thumb_nudge_tx,
            new_pending,
            paths_to_delete,
        })?;

        Ok(())
    }

    fn discover_boards(folders: &[Utf8PathBuf]) -> HashMap<String, String> {
        let mut found_boards = HashMap::new();
        for root in folders {
            if root.exists() {
                let name = Self::board_name(root);
                let path_key = WorkspacePath::normalize(root.as_std_path());
                found_boards.insert(path_key, name);
            }
        }
        found_boards
    }

    fn upsert_boards<R: Runtime>(
        app: &AppHandle<R>,
        db: &DbPool,
        found_boards: &HashMap<String, String>,
    ) -> AppResult<HashMap<String, i32>> {
        let mut board_ids = HashMap::new();
        let mut conn = db.get()?;
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().timestamp();

        for (path_key, name) in found_boards {
            tx.execute(
                "INSERT INTO boards (name, path, created_at, updated_at, is_missing) 
                 VALUES (?1, ?2, ?3, ?3, 0) 
                 ON CONFLICT(path) DO UPDATE SET name=excluded.name, updated_at=excluded.updated_at, is_missing=0",
                params![name, path_key, now],
            )?;
            let id: i32 = tx.query_row(
                "SELECT id FROM boards WHERE path = ?1",
                params![path_key],
                |row| row.get(0),
            )?;
            board_ids.insert(path_key.clone(), id);
        }
        tx.commit()?;
        log::info!("Phase 1: Boards upserted. Signalling UI.");
        app.emit("scan:boards-ready", ()).ok();
        Ok(board_ids)
    }

    fn walk_images(
        folders: &[Utf8PathBuf],
        cancel: &Arc<AtomicBool>,
        manager: &crate::services::state_manager::StateManager<tauri::Wry>,
    ) -> AppResult<Vec<FoundImage>> {
        let found_images = Arc::new(Mutex::new(Vec::new()));

        for (idx, root) in folders.iter().enumerate() {
            if !root.exists() {
                log::warn!("Source folder missing: {:?}", root);
                continue;
            }

            manager.update_task("workspace-scan", "scan", "running", Some(&format!("Scanning folder {} of {}...", idx + 1, folders.len())), (idx + 1) as i64, folders.len() as i64).ok();

            let mut overrides = ignore::overrides::OverrideBuilder::new(root);
            overrides.add("!node_modules/").ok();
            overrides.add("!.git/").ok();
            let override_obj = overrides.build().unwrap_or(ignore::overrides::Override::empty());

            let walker = ignore::WalkBuilder::new(root)
                .hidden(true)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .ignore(true)
                .overrides(override_obj)
                .threads(rayon::current_num_threads())
                .build_parallel();

            let root_clone = root.clone();
            walker.run(|| {
                let found_images = found_images.clone();
                let root = root_clone.clone();
                let cancel = cancel.clone();
                Box::new(move |result| {
                    if cancel.load(Ordering::SeqCst) {
                        return ignore::WalkState::Quit;
                    }

                    if let Ok(entry) = result {
                        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                            let path_std = entry.path();
                            if WorkspacePath::is_image(path_std) {
                                if let Ok(path) = Utf8PathBuf::from_path_buf(path_std.to_path_buf()) {
                                    let board_path = &root;
                                    let board_path_key = WorkspacePath::normalize(board_path.as_std_path());

                                    if let Ok(meta) = entry.metadata() {
                                        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH).duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                                        let size = meta.len() as i64;
                                        let filename = path.file_name().map(|n| n.to_string()).unwrap_or_else(|| "unknown".to_string());

                                        found_images.lock().push(FoundImage::builder()
                                            .path(path)
                                            .board_abs_path(board_path_key)
                                            .mtime(mtime)
                                            .size(size)
                                            .filename(filename)
                                            .build());
                                    }
                                }
                            }
                        }
                    }
                    ignore::WalkState::Continue
                })
            });
        }

        Ok(Arc::try_unwrap(found_images).unwrap().into_inner())
    }

    fn diff_and_upsert_images(
        db: &DbPool,
        board_ids: &HashMap<String, i32>,
        found_images: Vec<FoundImage>,
    ) -> AppResult<(usize, HashMap<String, ImageDbMetadata>)> {
        let mut new_pending = 0;
        let mut conn = db.get()?;
        let tx = conn.transaction()?;

        // Fetch existing images for these boards
        let mut existing_images = HashMap::new();
        for board_path in board_ids.keys() {
            let root_prefix = format!("{}%", board_path);
            let mut stmt = tx.prepare_cached(
                "SELECT id, path, mtime, size_bytes, thumbnail_status, thumb_path 
                 FROM images 
                 WHERE path LIKE ?1"
            )?;

            let rows = stmt.query_map(params![root_prefix], |row| {
                let db_path: String = row.get(1)?;
                let normalized_db_path = WorkspacePath::normalize(std::path::Path::new(&db_path));
                Ok((
                    normalized_db_path,
                    (
                        row.get(0)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ),
                ))
            })?;

            for row in rows.flatten() {
                existing_images.insert(row.0, ImageDbMetadata {
                    id: row.1.0,
                    mtime: row.1.1,
                    size: row.1.2,
                    _thumb_status: row.1.3,
                    thumb_path: row.1.4,
                });
            }
        }

        let now = chrono::Utc::now().timestamp();
        let mut stmt_update_modified = tx.prepare_cached("UPDATE images SET mtime=?1, size_bytes=?2, board_id=?3, thumbnail_status='pending', thumb_path=NULL, is_missing=0 WHERE id=?4")?;
        let mut stmt_update_present = tx.prepare_cached("UPDATE images SET is_missing=0 WHERE id=?1")?;
        let mut stmt_insert = tx.prepare_cached("INSERT INTO images (filename, path, board_id, size_bytes, mtime, created_at, thumbnail_status, is_missing) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0)")?;

        for img in found_images {
            let abs_path = WorkspacePath::normalize(img.path.as_std_path());
            let board_id = *board_ids.get(&img.board_abs_path).unwrap_or(&0);

            if let Some(metadata) = existing_images.remove(&abs_path) {
                if metadata.mtime != img.mtime || metadata.size != img.size {
                    stmt_update_modified.execute(params![img.mtime, img.size, board_id, metadata.id])?;
                    new_pending += 1;
                } else {
                    stmt_update_present.execute(params![metadata.id])?;
                }
            } else {
                stmt_insert.execute(params![img.filename, abs_path, board_id, img.size, img.mtime, now])?;
                new_pending += 1;
            }
        }
        
        drop(stmt_update_modified);
        drop(stmt_update_present);
        drop(stmt_insert);

        tx.commit()?;
        Ok((new_pending, existing_images))
    }

    fn cleanup_missing(
        db: &DbPool,
        folders: &[Utf8PathBuf],
        found_boards: &HashMap<String, String>,
        remaining_images: HashMap<String, ImageDbMetadata>,
    ) -> AppResult<Vec<String>> {
        let mut conn = db.get()?;
        let tx = conn.transaction()?;
        let mut paths_to_delete = Vec::new();

        // 1. Cleanup images whose boards are still present but file is gone
        let mut stmt_delete_img = tx.prepare_cached("DELETE FROM images WHERE id = ?1")?;
        let mut stmt_mark_img_missing = tx.prepare_cached("UPDATE images SET is_missing = 1 WHERE id = ?1")?;

        for (path, metadata) in remaining_images {
            let mut board_still_present = false;
            for board_path in found_boards.keys() {
                if path.starts_with(board_path) {
                    board_still_present = true;
                    break;
                }
            }

            if board_still_present {
                stmt_delete_img.execute(params![metadata.id])?;
                if let Some(tp) = metadata.thumb_path {
                    paths_to_delete.push(tp);
                }
            } else {
                stmt_mark_img_missing.execute(params![metadata.id])?;
            }
        }
        
        drop(stmt_delete_img);
        drop(stmt_mark_img_missing);

        // 2. Cleanup boards that are missing from disk
        let mut stmt_get_boards = tx.prepare_cached("SELECT id, path FROM boards WHERE path LIKE ?1")?;
        let mut stmt_mark_board_missing = tx.prepare_cached("UPDATE boards SET is_missing = 1 WHERE id = ?1")?;

        for root in folders {
            let root_prefix = format!("{}%", root);
            let db_boards: Vec<(i32, String)> = stmt_get_boards.query_map(params![root_prefix], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
            for (id, path) in db_boards {
                if !found_boards.contains_key(&path) {
                    stmt_mark_board_missing.execute(params![id])?;
                }
            }
        }
        
        drop(stmt_get_boards);
        drop(stmt_mark_board_missing);

        tx.commit()?;
        Ok(paths_to_delete)
    }

    fn finalize<R: Runtime>(
        app: &AppHandle<R>,
        db: &DbPool,
        manager: &crate::services::state_manager::StateManager<tauri::Wry>,
        params: ScanFinalizeParams,
    ) -> AppResult<()> {
        let mut conn = db.get()?;
        let tx = conn.transaction()?;

        // Update image counts and covers
        tx.execute("UPDATE boards SET image_count = (SELECT COUNT(*) FROM images WHERE board_id = boards.id AND is_missing = 0)", [])?;
        tx.execute("UPDATE boards SET cover_image = (
                SELECT thumb_path FROM images 
                WHERE board_id = boards.id AND thumb_path IS NOT NULL AND width > 0 AND is_missing = 0
                ORDER BY mtime DESC LIMIT 1
            )", [])?;
        tx.commit()?;

        // Mark boards for AI sync
        for board_id in params.board_ids.values() {
            manager.set_board_ai_sync(*board_id, true).ok();
        }

        // Delete thumbnail files from disk
        let local_data_dir = app.path().app_local_data_dir().ok();
        for abs_path_str in params.paths_to_delete {
            let abs_path = if let Some(ref base) = local_data_dir {
                base.join(&abs_path_str)
            } else {
                std::path::Path::new(&abs_path_str).to_path_buf()
            };
            if abs_path.exists() {
                let _ = std::fs::remove_file(abs_path);
            }
        }

        info!("Reconciliation complete. Multi-folder images synced.");
        manager.update_task("workspace-scan", "scan", "running", Some("Scan complete"), params.folders.len() as i64, params.folders.len() as i64).ok();
        app.emit("scan:complete", ()).ok();
        
        if params.new_pending > 0 {
            params.thumb_nudge_tx.send(()).ok();
        }

        Ok(())
    }

    fn board_name(root: &Utf8Path) -> String {
        root.file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Source Folder".to_string())
    }
}
