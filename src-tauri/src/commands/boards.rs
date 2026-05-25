use std::fs;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use crate::db;
use crate::commands::state::AppState;
use crate::error::{AppResult, AppError};

#[derive(Serialize, Deserialize, Clone, Debug, bon::Builder)]
pub struct Board {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub cover_image: Option<String>,
    pub image_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_missing: bool,
}

#[tauri::command]
pub async fn get_boards<R: Runtime>(_app: AppHandle<R>, state: tauri::State<'_, AppState>) -> AppResult<Vec<Board>> {
    let db = state.get_pool()?;
    let state_manager = state.state_manager.clone();
    
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.get()?;
        
        let active_id = state_manager.config().active_workspace_id
            .ok_or_else(|| AppError::NotFound("No active workspace".to_string()))?;
        let workspaces = state_manager.get_workspaces()?;
        let ws = workspaces.into_iter().find(|w| w.id == active_id)
            .ok_or_else(|| AppError::NotFound("Active workspace not found in config".to_string()))?;
        let board_ids = ws.board_ids;

        let boards = db::boards::get_boards(&conn, Some(&board_ids))?;
        Ok(boards)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn create_board(_state: tauri::State<'_, AppState>, _name: String) -> AppResult<()> {
    // In the new architecture, boards are added by adding source folders to the workspace.
    // Manual board creation is disabled for now to maintain source folder integrity.
    Err(AppError::WorkspaceError("Please use 'Add Folder' to create new boards".to_string()))
}


#[tauri::command]
pub async fn delete_board(state: tauri::State<'_, AppState>, board_id: i32) -> AppResult<()> {
    log::info!("Command: delete_board (id: {})", board_id);
    let manager = state.state_manager.clone();
    let db = state.get_pool()?;
    let workspace_handle = state.workspace.lock().clone();
    
    manager.update_task("board-delete", "delete", "running", Some("Deleting board..."), 0, 0).ok();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.get()?;
        let abs_path_str = crate::db::boards::get_board_path(&conn, board_id)?;
        let abs_path = std::path::PathBuf::from(abs_path_str);

        log::info!("Moving source folder to Recycle Bin: {:?}", abs_path);
        if abs_path.exists() {
            trash::delete(&abs_path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        }
        Ok::<(), crate::error::AppError>(())
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    // Trigger reconcile to clean up DB
    if let Some(handle) = workspace_handle {
        handle.reconciler_tx.send(()).ok();
    }
    manager.finish_task("board-delete", "completed", None).ok();

    Ok(())
}

#[tauri::command]
pub async fn cleanup_orphaned_boards<R: Runtime>(app: AppHandle<R>, state: tauri::State<'_, AppState>) -> AppResult<usize> {
    log::info!("Command: cleanup_orphaned_boards");
    let manager = state.state_manager.clone();
    let db = state.get_pool()?;
    
    tauri::async_runtime::spawn_blocking(move || {
        let workspaces = manager.get_workspaces()?;
        let all_board_ids: std::collections::HashSet<i32> = workspaces.iter()
            .flat_map(|w| w.board_ids.clone())
            .collect();

        let conn = db.get()?;

        // Fetch all boards from DB
        let mut stmt = conn.prepare("SELECT id, path FROM boards")?;
        let board_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut orphaned_ids = Vec::new();
        let mut orphaned_paths = Vec::new();
        for (id, path) in board_rows.flatten() {
            if !all_board_ids.contains(&id) {
                orphaned_ids.push(id);
                orphaned_paths.push(path);
            }
        }

        let count = orphaned_ids.len();
        if count > 0 {
            log::info!("Cleaning up {} orphaned boards and their cache", count);
            
            let ai_cache_dir = crate::db::get_ai_cache_dir(&app)?;
            let thumb_cache_dir = crate::db::get_thumb_cache_dir(&app)?;
            
            use crate::services::path_utils::WorkspacePath;
            use std::path::Path;

            for path in &orphaned_paths {
                let folder_id = WorkspacePath::folder_id(Path::new(path));
                
                // AI Cache
                let ai_db = ai_cache_dir.join(format!("{}.db", folder_id));
                if ai_db.exists() {
                    log::info!("Deleting AI cache: {:?}", ai_db);
                    let _ = fs::remove_file(ai_db);
                }
                
                // Thumbnails Cache
                let thumb_dir = thumb_cache_dir.join(folder_id);
                if thumb_dir.exists() {
                    log::info!("Deleting thumbnails cache: {:?}", thumb_dir);
                    let _ = fs::remove_dir_all(thumb_dir);
                }
            }

            // 2. Delete from DB using chunked statements
            const CHUNK_SIZE: usize = 999; 
            for chunk in orphaned_ids.chunks(CHUNK_SIZE) {
                let placeholders: String = vec!["?"; chunk.len()].join(",");
                let sql = format!("DELETE FROM boards WHERE id IN ({})", placeholders);
                let params: Vec<rusqlite::types::Value> = chunk.iter().map(|&id| id.into()).collect();
                conn.execute(&sql, rusqlite::params_from_iter(params))?;
            }
        }

        Ok(count)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}
