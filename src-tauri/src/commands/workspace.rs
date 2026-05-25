use crate::commands::state::AppState;
use crate::config::Workspace;
use crate::error::AppResult;
use crate::workspace::WorkspaceHandle;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Clone, Serialize)]
pub struct WorkspaceChangedPayload {
    pub id: String,
    pub is_switch: bool,
}

#[derive(Clone, Serialize)]
pub struct ConfigDto {
    pub active: Option<String>,
    pub workspaces: Vec<Workspace>,
    pub ai: crate::ai::config::AiConfig,
    pub logging_enabled: bool,
    pub active_tasks: Vec<crate::services::state_manager::BackgroundTask>,
}

#[tauri::command]
pub async fn get_config(state: tauri::State<'_, AppState>) -> AppResult<ConfigDto> {
    let state_manager = state.state_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let config = state_manager.config();
        let workspaces = state_manager.get_workspaces()?;
        let active_tasks = state_manager.get_active_tasks().unwrap_or_default();

        Ok(ConfigDto {
            active: config.active_workspace_id,
            workspaces,
            ai: config.ai_config,
            logging_enabled: config.logging_enabled,
            active_tasks,
        })
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn set_active_workspace<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    log::info!("Command: set_active_workspace for ID: {}", id);
    let manager = state.state_manager.clone();
    let id_clone = id.clone();

    // 1. Shut down old workspace if present
    let old_handle = state.workspace.lock().take();
    if let Some(handle) = old_handle {
        log::info!("Shutting down previous workspace...");
        let app_inner = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            handle.shutdown(&app_inner);
        })
        .await
        .ok();
    }

    state.embedding_cache.clear();

    // 2. Fetch ws_config, update active state, and open WorkspaceHandle in spawn_blocking
    let app_clone = app.clone();
    let (handle, is_switch) = tauri::async_runtime::spawn_blocking(move || {
        let workspaces = manager.get_workspaces()?;
        let ws_config = workspaces
            .into_iter()
            .find(|w| w.id == id_clone)
            .ok_or_else(|| crate::error::AppError::NotFound("Workspace not found".to_string()))?;

        let is_switch = manager.config().active_workspace_id != Some(id_clone.clone());
        manager.set_active_workspace(Some(id_clone.clone()))?;

        let handle = WorkspaceHandle::open(app_clone, ws_config)?;
        Ok::<(std::sync::Arc<WorkspaceHandle>, bool), crate::error::AppError>((handle, is_switch))
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    // 3. Update active workspace state
    {
        let mut ws = state.workspace.lock();
        *ws = Some(handle);
    }

    app.emit(
        "workspace-changed",
        WorkspaceChangedPayload { id, is_switch },
    )
    .ok();
    Ok(())
}

#[tauri::command]
pub async fn add_workspace(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    name: String,
) -> AppResult<String> {
    log::info!("Command: add_workspace (name: {})", name);
    let state_manager = state.state_manager.clone();
    let ws = Workspace::new(Some(name), vec![], vec![]);
    let id = ws.id.clone();

    let ws_clone = ws.clone();
    tauri::async_runtime::spawn_blocking(move || {
        state_manager.upsert_workspace(&ws_clone.id, &ws_clone.name, &ws_clone.board_ids)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    // Switch to it
    set_active_workspace(app.clone(), state, id.clone()).await?;

    Ok(id)
}

#[tauri::command]
pub async fn rename_workspace<R: Runtime>(
    _app_handle: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: String,
    new_name: String,
) -> AppResult<()> {
    let state_manager = state.state_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let workspaces = state_manager.get_workspaces()?;

        if let Some(mut ws) = workspaces.into_iter().find(|w| w.id == id) {
            ws.name = new_name;
            state_manager.upsert_workspace(&ws.id, &ws.name, &ws.board_ids)?;
            Ok(())
        } else {
            Err(crate::error::AppError::NotFound("Workspace not found".to_string()))
        }
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn remove_workspace<R: Runtime>(
    _app_handle: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    let manager = state.state_manager.clone();
    let mut old_handle = None;

    if manager.config().active_workspace_id == Some(id.clone()) {
        manager.set_active_workspace(None).ok();
        old_handle = state.workspace.lock().take();
    }

    let id_clone = id.clone();
    let manager_clone = manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        manager_clone.remove_workspace(&id_clone)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    if let Some(handle) = old_handle {
        let app_inner = _app_handle.clone();
        tauri::async_runtime::spawn_blocking(move || {
            handle.shutdown(&app_inner);
        })
        .await
        .ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn scan_workspace(state: tauri::State<'_, AppState>) -> AppResult<()> {
    let handle = state.get_handle()?;
    handle.reconciler_tx.send(()).map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WorkspaceStatus {
    pub is_scanning: bool,
    pub total_images: i32,
}

#[tauri::command]
pub async fn get_workspace_status(
    state: tauri::State<'_, AppState>,
) -> AppResult<WorkspaceStatus> {
    let state_manager = state.state_manager.clone();
    let db = match state.get_pool() {
        Ok(pool) => pool,
        Err(_) => return Ok(WorkspaceStatus { is_scanning: false, total_images: 0 }),
    };

    tauri::async_runtime::spawn_blocking(move || {
        let active_id = match state_manager.config().active_workspace_id {
            Some(id) => id,
            None => return Ok(WorkspaceStatus { is_scanning: false, total_images: 0 }),
        };
        
        let workspaces = state_manager.get_workspaces()?;
        let ws = match workspaces.into_iter().find(|w| w.id == active_id) {
            Some(w) => w,
            None => return Ok(WorkspaceStatus { is_scanning: false, total_images: 0 }),
        };
        let board_ids = ws.board_ids;

        let is_scanning = state_manager.is_scanning();
        let conn = db.get()?;

        let (_, total) = crate::db::images::get_images(&conn, None, Some(&board_ids), 1, 1, None, None)?;

        Ok(WorkspaceStatus {
            is_scanning,
            total_images: total as i32,
        })
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageStats {
    pub total_images: i64,
    pub total_size_bytes: i64,
    pub total_boards: i64,
}

#[tauri::command]
pub async fn get_storage_stats(state: tauri::State<'_, AppState>) -> AppResult<StorageStats> {
    let state_manager = state.state_manager.clone();
    let db = state.get_pool()?;

    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.get()?;
        
        let active_id = state_manager.config().active_workspace_id
            .ok_or_else(|| crate::error::AppError::NotFound("No active workspace".to_string()))?;
        let workspaces = state_manager.get_workspaces()?;
        let ws = workspaces.into_iter().find(|w| w.id == active_id)
            .ok_or_else(|| crate::error::AppError::NotFound("Active workspace not found in config".to_string()))?;
        let board_ids = ws.board_ids;

        let (total_images, total_size_bytes) = crate::db::images::get_storage_stats(&conn, &board_ids)?;
        let boards = crate::db::boards::get_boards(&conn, Some(&board_ids))?;
        
        Ok(StorageStats { 
            total_images, 
            total_size_bytes, 
            total_boards: boards.len() as i64 
        })
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn is_scanning(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.state_manager.is_scanning())
}

#[tauri::command]
pub async fn add_folders_to_workspace<R: Runtime>(
    app_handle: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: Option<String>,
    paths: Vec<String>,
) -> AppResult<()> {
    let state_manager = state.state_manager.clone();
    let db = state.get_pool()?;
    
    let active_id = state_manager.config().active_workspace_id;
    let target_id = id.clone().or(active_id.clone())
        .ok_or_else(|| crate::error::AppError::NotFound("No active workspace".to_string()))?;
        
    let target_id_clone = target_id.clone();
    let state_manager_clone = state_manager.clone();
    
    let added_count = tauri::async_runtime::spawn_blocking(move || {
        use std::path::Path;
        let mut added_count = 0;
        let workspaces = state_manager_clone.get_workspaces()?;
        if let Some(mut ws) = workspaces.into_iter().find(|w| w.id == target_id_clone) {
            let conn = db.get()?;
            for path in paths {
                let path_obj = Path::new(&path);
                if !path_obj.is_dir() { continue; }

                let normalized_path = crate::services::path_utils::WorkspacePath::normalize(path_obj);
                let name = path_obj.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled Board");

                let board_id = crate::db::boards::upsert_board(&conn, name, &normalized_path)?;

                if !ws.board_ids.contains(&board_id) {
                    ws.board_ids.push(board_id);
                    added_count += 1;
                }
            }

            if added_count > 0 {
                state_manager_clone.upsert_workspace(&ws.id, &ws.name, &ws.board_ids)?;
            }
        }
        Ok::<i32, crate::error::AppError>(added_count)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    if added_count > 0 {
        if let Ok(handle) = state.get_handle() {
            if handle.id == target_id {
                set_active_workspace(app_handle.clone(), state, target_id).await?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn remove_board_from_workspace<R: Runtime>(
    app_handle: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    id: Option<String>,
    board_id: i32,
) -> AppResult<()> {
    let state_manager = state.state_manager.clone();
    
    let active_id = state_manager.config().active_workspace_id;
    let target_id = id.clone().or(active_id.clone())
        .ok_or_else(|| crate::error::AppError::NotFound("No active workspace".to_string()))?;

    let target_id_clone = target_id.clone();
    let state_manager_clone = state_manager.clone();
    
    let removed = tauri::async_runtime::spawn_blocking(move || {
        let workspaces = state_manager_clone.get_workspaces()?;
        if let Some(mut ws) = workspaces.into_iter().find(|w| w.id == target_id_clone) {
            let initial_len = ws.board_ids.len();
            ws.board_ids.retain(|&bid| bid != board_id);

            if ws.board_ids.len() == initial_len {
                log::warn!("No board was removed from workspace! ID not found: {}", board_id);
                Ok::<bool, crate::error::AppError>(false)
            } else {
                state_manager_clone.upsert_workspace(&ws.id, &ws.name, &ws.board_ids)?;
                Ok(true)
            }
        } else {
            Ok(false)
        }
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    if removed {
        if let Ok(handle) = state.get_handle() {
            if handle.id == target_id {
                set_active_workspace(app_handle, state, target_id).await?;
            }
        }
    }

    Ok(())
}
