use std::path::Path;
use std::fs;
use tauri::{AppHandle, Runtime, Manager};
use serde::{Deserialize, Serialize};
use crate::db;
use crate::commands::state::AppState;
use crate::error::{AppResult, AppError};

#[derive(Serialize, Deserialize, Clone, Debug, bon::Builder)]
pub struct Image {
    pub id: i32,
    #[builder(into)]
    pub filename: String,
    #[builder(into)]
    pub path: String,
    pub board_id: i32,
    #[builder(into)]
    pub board_name: String,
    #[builder(into)]
    pub thumb_path: Option<String>,
    #[builder(into)]
    pub thumbnail_status: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: i64,
    pub mtime: i64,
    pub created_at: i64,
    pub is_missing: bool,
}

impl Image {
    /// Cleans up thumbnail path if not ready.
    pub fn clean(mut self, local_data_dir: &Option<String>) -> Self {
        if self.thumbnail_status != "ready" {
            self.thumb_path = None;
        } else if let (Some(ref mut path), Some(ref base)) = (&mut self.thumb_path, local_data_dir) {
            // Convert relative path to absolute path
            if !path.is_empty() && !path.starts_with('/') && !path.contains(':') && !path.starts_with('\\') {
                *path = format!("{}/{}", base.replace("\\", "/"), path);
            }
        }
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, bon::Builder)]
pub struct PaginatedImages {
    pub images: Vec<Image>,
    pub total: u32,
}

#[tauri::command]
pub async fn get_images<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    board_id: Option<i32>,
    page: u32,
    page_size: u32,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> AppResult<PaginatedImages> {
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

        let (images, total) = db::images::get_images(&conn, board_id, Some(&board_ids), page, page_size, sort_by, sort_order)?;
        
        let local_data_dir = app.path().app_local_data_dir().ok().map(|p| p.to_string_lossy().to_string());
        let cleaned_images = images.into_iter().map(|img| img.clean(&local_data_dir)).collect();
        Ok(PaginatedImages::builder()
            .images(cleaned_images)
            .total(total)
            .build())
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn get_image<R: Runtime>(app: AppHandle<R>, state: tauri::State<'_, AppState>, id: i32) -> AppResult<Image> {
    let db = state.get_pool()?;
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.get()?;
        let mut stmt = conn.prepare(
            "SELECT i.id, i.filename, i.path, i.board_id, b.name as board_name, i.thumb_path, i.thumbnail_status, i.width, i.height, i.size_bytes, i.mtime, i.created_at, i.is_missing
             FROM images i JOIN boards b ON i.board_id = b.id 
             WHERE i.id = ?1"
        )?;
                                    
        let img = stmt.query_row([id], db::images::map_image_row)?;
        let local_data_dir = app.path().app_local_data_dir().ok().map(|p| p.to_string_lossy().to_string());
        Ok(img.clean(&local_data_dir))
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_image(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    image_id: i32,
) -> AppResult<()> {
    log::info!("Command: delete_image (id: {})", image_id);
    let db = state.get_pool()?;
    
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.get()?;
        let (abs_path_str, thumb_path_opt) = crate::db::images::get_image_paths(&conn, image_id)?;

        // 1. Delete physical image file by moving it to Trash
        let abs_path = std::path::PathBuf::from(abs_path_str.clone());
        if abs_path.exists() {
            trash::delete(&abs_path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        }

        // 2. Delete thumbnail file from cache if it exists
        if let Some(ref thumb_path_str) = thumb_path_opt {
            let local_data_dir = app.path().app_local_data_dir().ok();
            let thumb_path = if let Some(ref base) = local_data_dir {
                base.join(thumb_path_str)
            } else {
                Path::new(thumb_path_str).to_path_buf()
            };
            if thumb_path.exists() {
                fs::remove_file(thumb_path).ok();
            }
        }

        // 3. Clear embedding from the AI vector store immediately
        if let Err(e) = crate::ai::embeddings_store::delete_embeddings(&app, vec![abs_path_str]) {
            log::error!("Failed to delete embedding for image {}: {}", image_id, e);
        }

        // 4. Delete from DB
        conn.execute("DELETE FROM images WHERE id = ?1", [image_id])?;
        Ok::<(), crate::error::AppError>(())
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    Ok(())
}

#[tauri::command]
pub async fn import_images(state: tauri::State<'_, AppState>, board_id: i32, file_paths: Vec<String>) -> AppResult<serde_json::Value> {
    log::info!("Command: import_images (board: {}, count: {})", board_id, file_paths.len());
    let db = state.get_pool()?;

    // Wrap copying loop in spawn_blocking
    let imported = tauri::async_runtime::spawn_blocking(move || {
        let conn = db.get()?;
        let board_path_str = crate::db::boards::get_board_path(&conn, board_id)?;
        let board_path = std::path::PathBuf::from(board_path_str);
        
        let mut count = 0;
        for src_path_str in file_paths {
            let src_path = Path::new(&src_path_str);
            if !src_path.exists() { continue; }

            let filename = src_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            fs::create_dir_all(&board_path).ok();
            
            let target_path = board_path.join(filename);
            if let Err(e) = fs::copy(src_path, &target_path) {
                log::error!("Failed to copy file: {}", e);
            } else {
                count += 1;
            }
        }
        Ok::<i32, crate::error::AppError>(count)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    Ok(serde_json::json!({ "imported": imported }))
}


#[tauri::command]
pub async fn open_in_explorer(state: tauri::State<'_, AppState>, path: String) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use crate::services::path_utils::WorkspacePath;

        let normalized_path = WorkspacePath::normalize(Path::new(&path));
        let roots = state.get_roots()?;
        let is_allowed = roots.iter().any(|root| {
            let normalized_root = WorkspacePath::normalize(Path::new(root));
            normalized_path == normalized_root || normalized_path.starts_with(&format!("{}/", normalized_root))
        });

        if !is_allowed {
            return Err(crate::error::AppError::WorkspaceError("Access denied: File is outside of workspace roots".to_string()));
        }

        Command::new("explorer").arg("/select,").arg(normalized_path.replace("/", "\\")).spawn()?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    { Err(crate::error::AppError::WorkspaceError("Only supported on Windows".to_string())) }
}
