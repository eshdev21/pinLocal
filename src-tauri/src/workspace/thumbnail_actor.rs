use std::sync::mpsc::{channel, Sender};
use rayon::prelude::*;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use crate::error::AppResult;
use crate::services::thumbnail_service::generate_thumbnail;
use crate::db::DbPool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use camino::{Utf8Path, Utf8PathBuf};


pub struct ThumbnailActor;

#[derive(serde::Serialize, Clone)]
struct ThumbnailUpdate {
    id: i32,
    thumb_path: String, // resolved absolute path
    width: u32,
    height: u32,
}

impl ThumbnailActor {
    pub fn spawn<R: Runtime>(
        app: AppHandle<R>, 
        db: DbPool,
        cancel: Arc<AtomicBool>,
    ) -> AppResult<(Sender<()>, std::thread::JoinHandle<()>)> {
        let (tx, rx) = channel::<()>();
        let actor_tx = tx.clone();
        
        let handle = std::thread::spawn(move || {
            if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                loop {
                    // Wait for a nudge to process thumbnails. 
                    if rx.recv().is_err() {
                        break;
                    }
                    
                    if cancel.load(Ordering::SeqCst) {
                        log::info!("ThumbnailActor: Workspace cancelled, exiting thread.");
                        break;
                    }
                    
                    if let Err(e) = Self::process_batch(&app, &db, &cancel) {
                        log::error!("Thumbnail processing failed: {}", e);
                    }
                }
            })) {
                log::error!("ThumbnailActor PANICKED: {:?}", panic_info);
            }
        });

        Ok((actor_tx, handle))
    }

    fn process_batch<R: Runtime>(
        app: &AppHandle<R>, 
        db: &DbPool,
        cancel: &Arc<AtomicBool>,
    ) -> AppResult<()> {
        log::info!("ThumbnailActor: Starting process_batch...");
        let mut all_ready_updates = Vec::new();
        let mut total_session_processed = 0;

        let mut total_in_db = 0;
        let mut done_in_db = 0;
        let mut first_iteration = true;

        loop {
            let state = app.state::<crate::commands::state::AppState>();
            let manager = &state.state_manager;

            if cancel.load(Ordering::SeqCst) {
                return Ok(());
            }

            // 0. Get app_local_data_dir
            let app_local_data_dir = match app.path().app_local_data_dir() {
                Ok(dir) => match Utf8PathBuf::from_path_buf(dir) {
                    Ok(path) => path,
                    Err(e) => {
                        log::error!("Non-UTF8 app local data dir: {:?}", e);
                        break;
                    }
                },
                Err(e) => {
                    log::error!("Failed to get app local data dir: {}", e);
                    break;
                }
            };

            // 0.1 Query total count ONLY on first iteration of this nudge
            if first_iteration {
                let conn = db.get()?;
                done_in_db = conn.query_row("SELECT COUNT(*) FROM images WHERE thumbnail_status = 'ready'", [], |row| row.get::<_, i64>(0))?;
                let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM images WHERE thumbnail_status IN ('pending', 'generating')", [], |row| row.get(0))?;
                total_in_db = done_in_db + remaining;
                first_iteration = false;

                if total_in_db > done_in_db {
                    log::info!("ThumbnailActor: Progress starting: {}/{}", done_in_db, total_in_db);
                    manager.update_task("thumb-gen", "thumbnails", "running", Some("Generating thumbnails..."), done_in_db, total_in_db).ok();
                }
            } else if total_in_db > 0 {
                // Just update task with current count
                manager.update_task("thumb-gen", "thumbnails", "running", Some("Generating thumbnails..."), done_in_db + total_session_processed as i64, total_in_db).ok();
            }

            let batch: Vec<(i32, String, String)> = {
                let conn = db.get()?;
                let mut stmt = conn.prepare("
                    SELECT i.id, i.path, b.path 
                    FROM images i 
                    JOIN boards b ON i.board_id = b.id 
                    WHERE i.thumbnail_status = 'pending' 
                    LIMIT 50
                ")?;
                let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
                let mut batch_results = Vec::new();
                for r in rows {
                    batch_results.push(r?);
                }
                batch_results
            };

            if batch.is_empty() {
                if total_session_processed > 0 {
                    manager.finish_task("thumb-gen", "completed", None).ok();
                }
                break;
            }
            total_session_processed += batch.len();

            // Mark generating
            {
                let mut conn = db.get()?;
                let tx = conn.transaction()?;
                {
                    let mut stmt = tx.prepare_cached("UPDATE images SET thumbnail_status = 'generating' WHERE id = ?1")?;
                    for (id, _, _) in &batch {
                        stmt.execute([id])?;
                    }
                }
                tx.commit()?;
            }

            if cancel.load(Ordering::SeqCst) {
                Self::reset_generating(db);
                return Ok(());
            }

            use crate::services::path_utils::WorkspacePath;

            // Process in parallel
            let results: Vec<Result<ThumbnailUpdate, i32>> = batch.into_par_iter().map(|(id, img_abs_path_str, board_abs_path_str)| {
                let src = Utf8Path::new(&img_abs_path_str);
                let board = Utf8Path::new(&board_abs_path_str);
                
                let board_id = WorkspacePath::folder_id(board.as_std_path());
                let dest_root = app_local_data_dir.join("cache").join("thumbnails").join(&board_id);
                
                // Robust path stripping for Windows
                let rel = if let Ok(stripped) = src.strip_prefix(board) {
                    stripped
                } else {
                    Utf8Path::new(src.file_name().unwrap_or_default())
                };
                
                let dest_file = dest_root.join(rel).with_extension("webp");
                
                if !dest_file.starts_with(&dest_root) {
                    log::error!("Security: Prevented write outside of cache: {:?}", dest_file);
                    return Err(id);
                }
                
                if let Some(dest_parent) = dest_file.parent() {
                    std::fs::create_dir_all(dest_parent).ok();
                } else {
                    log::error!("Failed to get parent for thumb path: {:?}", dest_file);
                    return Err(id);
                }

                match generate_thumbnail(src.as_std_path(), dest_file.as_std_path(), 400) {
                    Ok((w, h)) => {
                        let rel_thumb_path = format!(
                            "cache/thumbnails/{}/{}",
                            board_id,
                            rel.with_extension("webp").as_str().replace("\\", "/")
                        );
                        Ok(ThumbnailUpdate {
                            id,
                            thumb_path: rel_thumb_path,
                            width: w,
                            height: h,
                        })
                    }
                    Err(id_err) => {
                        let err_msg = id_err.to_string();
                        if err_msg.contains("os error 2") || err_msg.contains("not found") {
                             log::warn!("Source image missing, skipping: {:?}", src);
                        } else {
                             log::error!("Failed to generate thumbnail for {:?}: {}", src, id_err);
                        }
                        Err(id)
                    },
                }
            }).collect();

            // Apply results to DB
            {
                let mut conn = db.get()?;
                let tx = conn.transaction()?;
                {
                    let mut stmt_ready = tx.prepare_cached("UPDATE images SET thumbnail_status='ready', thumb_path=?1, width=?2, height=?3 WHERE id=?4")?;
                    let mut stmt_failed = tx.prepare_cached("UPDATE images SET thumbnail_status='failed' WHERE id=?1")?;

                    for res in results {
                        match res {
                            Ok(r) => {
                                stmt_ready.execute(rusqlite::params![r.thumb_path, r.width, r.height, r.id])?;
                                all_ready_updates.push(r);
                            }
                            Err(id) => {
                                stmt_failed.execute([id])?;
                            }
                        }
                    }
                }
                tx.commit()?;
            }

            if !all_ready_updates.is_empty() {
                app.emit(
                    "thumbnails:batch-ready",
                    serde_json::json!({ "updates": all_ready_updates.clone() }),
                ).ok();
                all_ready_updates.clear();
            }
        }

        Ok(())
    }

    fn reset_generating(db: &DbPool) {
        if let Ok(conn) = db.get() {
            conn.execute("UPDATE images SET thumbnail_status='pending' WHERE thumbnail_status='generating'", []).ok();
        }
    }
}
