use crate::ai::config::ModelId;
// use crate::ai::sidecar::ensure_python_process; (removed)
use crate::ai::embeddings_store;
use crate::ai::model_manager::ensure_model_loaded;
use crate::commands::state::AppState;
use crate::services::path_utils::WorkspacePath;
use rusqlite;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
// use camino::Utf8PathBuf; (removed unused)
use std::path::Path;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Serialize, Clone)]
pub struct IndexProgress {
    pub done: u32,
    pub total: u32,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PythonResponse {
    Ok {
        results: Option<Vec<SearchResult>>,
        added: Option<u32>,
        removed: Option<u32>,
        done: Option<u32>,
        message: Option<String>,
    },
    Progress {
        done: u32,
        total: u32,
    },
    Error {
        message: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SearchResult {
    pub path: String,
    pub score: f64,
    pub ftype: Option<String>,
}

// Moved to engine.rs

// Moved to embeddings_db.rs

fn fetch_workspace_boards(state: &AppState) -> AppResult<Vec<(i32, String)>> {
    let conn = state.get_conn()?;
    let board_ids = state.get_board_ids()?;

    if board_ids.is_empty() {
        return Ok(vec![]);
    }

    let placeholders: Vec<String> = board_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, path FROM boards WHERE id IN ({}) AND is_missing = 0",
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<i32> = board_ids;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
    })?;

    Ok(rows.flatten().collect())
}

pub fn fetch_board_inventory(state: &AppState, board_id: i32) -> AppResult<Vec<(String, i64)>> {
    let conn = state.get_conn()?;
    let mut stmt =
        conn.prepare("SELECT path, mtime FROM images WHERE board_id = ?1 AND is_missing = 0")?;

    let rows = stmt.query_map([board_id], |row| {
        let img_path = row.get::<_, String>(0)?;
        let normalized_img = WorkspacePath::normalize(std::path::Path::new(&img_path));
        Ok((normalized_img, row.get::<_, i64>(1)?))
    })?;

    Ok(rows.flatten().collect())
}

pub fn generate_embeddings<R: Runtime>(app: AppHandle<R>, _model: &ModelId) -> AppResult<u32> {
    let state = app.state::<AppState>();
    let handle = state.get_handle()?;
    handle.cancel_ai.store(false, Ordering::SeqCst);

    // 0. Ensure AI is ready.
    ensure_model_loaded(&app)?;

    let state = app.state::<AppState>();
    let manager = &state.state_manager;
    let ai_cache_dir = crate::db::get_ai_cache_dir(&app)?;

    // 1. Collect all active boards in the workspace, including empty ones.
    let boards = fetch_workspace_boards(&state)?;

    if boards.is_empty() {
        log::info!(">>> [AI] No boards found to index.");
        app.emit("ai:index-complete", 0).ok();
        return Ok(0);
    }

    let mut total_indexed = 0;

    // 2. Process all boards
    for (board_id, board_path_str) in boards {
        let board_images = fetch_board_inventory(&state, board_id)?;
        let board_path = Path::new(&board_path_str);
        let folder_id = WorkspacePath::folder_id(board_path);
        let ai_db_path = ai_cache_dir.join(format!("{}.db", folder_id));

        manager
            .update_task(
                "ai-index",
                "indexing",
                "running",
                Some(&format!("Indexing board: {}", board_path_str)),
                0,
                0,
            )
            .ok();

        if let Err(e) = embeddings_store::sync_inventory(&app, &board_path_str, board_images, Vec::new(), true) {
            manager
                .finish_task("ai-index", "failed", Some("AI inventory sync failed"))
                .ok();
            return Err(e);
        }

        {
            let mut ai_lock = state.ai.lock();
            let normalized_db = WorkspacePath::normalize(&ai_db_path);
            let normalized_root = WorkspacePath::normalize(Path::new(&board_path_str));

            ai_lock.send_command(
                "index",
                serde_json::json!({
                    "db_path": normalized_db,
                    "workspace_root": normalized_root
                }),
            )?;

            loop {
                let line = ai_lock.recv_response(std::time::Duration::from_secs(300))?;
                let res: PythonResponse = serde_json::from_str(&line)?;

                match res {
                    PythonResponse::Ok { done, .. } => {
                        total_indexed += done.unwrap_or(0);
                        // Success! Clear the dirty flag
                        manager.set_board_ai_sync(board_id, false).ok();
                        break;
                    }
                    PythonResponse::Progress { done, total } => {
                        manager
                            .update_task(
                                "ai-index",
                                "indexing",
                                "running",
                                Some("Indexing images..."),
                                done as i64,
                                total as i64,
                            )
                            .ok();
                    }
                    PythonResponse::Error { message } => {
                        manager
                            .finish_task("ai-index", "error", Some(&message))
                            .ok();
                        return Err(AppError::AiError(message));
                    }
                    _ => {}
                }

                if handle.cancel_ai.load(Ordering::SeqCst) {
                    crate::ai::sidecar::kill_sidecar(&app);
                    manager.finish_task("ai-index", "cancelled", None).ok();
                    break;
                }
            }
        }
    }

    manager.finish_task("ai-index", "completed", None).ok();

    // Invalidate entire cache as many boards might have been updated
    state.embedding_cache.clear();

    log::info!(
        ">>> [AI] Global Indexing Finished. Total new embeddings: {}",
        total_indexed
    );
    app.emit("ai:index-complete", total_indexed).ok();
    Ok(total_indexed)
}

// Moved to embeddings_db.rs

// Moved to embeddings_db.rs

// Moved to embeddings_db.rs

// Moved to engine.rs

// Moved to worker.rs
