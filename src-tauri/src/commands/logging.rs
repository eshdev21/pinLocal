use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use crate::commands::state::AppState;
use crate::error::AppResult;

#[tauri::command]
pub async fn open_logs_folder(app: tauri::AppHandle) -> AppResult<()> {
    let log_dir = app.path().app_log_dir()?;
    
    app.opener()
        .open_path(log_dir.to_string_lossy(), None::<String>)
        .map_err(|e| crate::error::AppError::TauriError(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn clear_logs(app: tauri::AppHandle) -> AppResult<()> {
    let log_dir = app.path().app_log_dir()?;
    let log_path = log_dir.join("app.log");
    
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(log_path, "")
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;
    
    log::info!("Logs cleared by user.");
    Ok(())
}

#[tauri::command]
pub async fn set_logging_enabled(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    let state_manager = state.state_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        state_manager.update_config(|c| {
            c.logging_enabled = enabled;
        })
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;
    Ok(())
}
