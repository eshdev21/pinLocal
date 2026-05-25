use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;
use crate::ai::config::{self, *, ModelId};
use crate::ai::model_manager;
use crate::ai::search_orchestrator;
use crate::commands::state::AppState;
use crate::error::{AppResult, AppError};
use serde::Serialize;
// use camino::Utf8PathBuf; (removed)

#[tauri::command]
pub async fn get_ai_config(state: tauri::State<'_, AppState>) -> AppResult<AiConfig> {
    Ok(state.state_manager.config().ai_config)
}

#[tauri::command]
pub async fn set_ai_enabled<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    let state_manager = state.state_manager.clone();
    tauri::async_runtime::spawn_blocking(move || {
        state_manager.update_config(|c| {
            c.ai_config.enabled = enabled;
        })
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    if !enabled {
        crate::ai::sidecar::kill_sidecar(&app);
        state.state_manager.set_engine_status(crate::services::state_manager::EngineStatus::Disabled);
    } else {
        // Explicitly set to Stopped (from Disabled) so the UI reacts immediately
        state.state_manager.set_engine_status(crate::services::state_manager::EngineStatus::Stopped);
    }
    
    Ok(())
}

macro_rules! impl_ai_setter {
    ($name:ident, $field:ident, $type:ty) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, AppState>,
            value: $type,
        ) -> AppResult<()> {
            let state_manager = state.state_manager.clone();
            tauri::async_runtime::spawn_blocking(move || {
                state_manager.update_config(|c| {
                    c.ai_config.$field = value;
                })
            })
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        }
    };
}

impl_ai_setter!(set_ai_model, model, ModelId);
impl_ai_setter!(set_ai_hardware, hardware, HardwareType);
impl_ai_setter!(set_cuda_version, cuda_version, CudaVersion);
impl_ai_setter!(set_python_version, python_version, PythonVersion);
impl_ai_setter!(set_link_mode, link_mode, UvLinkMode);
impl_ai_setter!(set_use_appdata_models, use_appdata_models, bool);
impl_ai_setter!(set_ai_mode, mode, AiMode);
impl_ai_setter!(set_venv_path, venv_path, Option<String>);

#[tauri::command]
pub async fn select_venv_path<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<Option<String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Select Python Virtual Environment Directory (.venv)")
        .pick_folder(move |folder_path| {
            if let Some(path) = folder_path {
                let abs_path = crate::services::path_utils::WorkspacePath::normalize(path.as_path().unwrap());
                tx.send(Some(abs_path)).ok();
            } else {
                tx.send(None).ok();
            }
        });
    rx.recv().map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn setup_siglip<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<()> {
    // Run in background thread to not block the main tauri async runtime
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai::sidecar::kill_sidecar(&app);
        crate::ai::setup::setup_siglip(app)
    }).await.map_err(|e| AppError::Internal(e.to_string()))?.map_err(|e| AppError::AiError(e.to_string()))
}

#[tauri::command]
pub async fn kill_sidecar<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<()> {
    crate::ai::sidecar::kill_sidecar(&app);
    Ok(())
}


#[tauri::command]
pub async fn load_model<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<()> {
    crate::ai::sidecar::kill_sidecar(&app);
    crate::ai::model_manager::ensure_model_loaded(&app)
}

#[derive(Serialize, bon::Builder)]
pub struct AiRuntimeStatus {
    pub python_ready: bool,
    pub model_ready: bool,
    pub is_running: bool,
    pub is_loaded: bool,
}

#[tauri::command]
pub async fn get_ai_runtime_status(
    app: AppHandle<impl Runtime>,
) -> AppResult<AiRuntimeStatus> {
    let state = app.state::<AppState>();
    let status = config::get_python_status(&app);
    
    let app_status = state.state_manager.get_app_status()?;
    
    Ok(AiRuntimeStatus::builder()
        .python_ready(status.venv_ready)
        .model_ready(status.model_ready)
        .is_running(app_status.ai_engine_status == crate::services::state_manager::EngineStatus::Running)
        .is_loaded(app_status.ai_model_status == crate::services::state_manager::ModelStatus::Ready)
        .build())
}

#[tauri::command]
pub async fn get_app_status(state: tauri::State<'_, AppState>) -> AppResult<crate::services::state_manager::AppStatus> {
    state.state_manager.get_app_status()
}

#[tauri::command]
pub async fn generate_embeddings(
    app: AppHandle<impl Runtime>,
    state: tauri::State<'_, AppState>,
) -> AppResult<u32> {
    let app_clone = app.clone();
    let model = state.state_manager.config().ai_config.model.clone();
    // Run in blocking thread to not freeze the main tauri async pool during long indexing
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai::indexing_service::generate_embeddings(app_clone, &model)
    }).await.map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn cancel_indexing(app: AppHandle<impl Runtime>) -> AppResult<()> {
    model_manager::cancel_indexing(&app);
    Ok(())
}

#[tauri::command]
pub async fn reset_embeddings(app: AppHandle<impl Runtime>, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let state_manager = state.state_manager.clone();
    let db = state.get_pool()?;

    tauri::async_runtime::spawn_blocking(move || {
        let active_id = state_manager.config().active_workspace_id
            .ok_or_else(|| AppError::NotFound("No active workspace".to_string()))?;
        let workspaces = state_manager.get_workspaces()?;
        let ws = workspaces.into_iter().find(|w| w.id == active_id)
            .ok_or_else(|| AppError::NotFound("Active workspace not found in config".to_string()))?;
        
        let conn = db.get()?;
        let boards = crate::db::boards::get_boards(&conn, Some(&ws.board_ids))?;
        let roots: Vec<String> = boards.iter().map(|b| crate::services::path_utils::WorkspacePath::normalize(std::path::Path::new(&b.path))).collect();
        
        let ai_cache = crate::db::get_ai_cache_dir(&app)?;

        for folder in roots {
            let folder_id = crate::services::path_utils::WorkspacePath::folder_id(std::path::Path::new(&folder));
            let db_path = ai_cache.join(format!("{}.db", folder_id));
            if db_path.exists() {
                std::fs::remove_file(db_path)?;
            }
        }
        Ok::<(), crate::error::AppError>(())
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;

    state.embedding_cache.clear();
    Ok(())
}

#[tauri::command]
pub async fn ai_search(
    app: AppHandle<impl Runtime>,
    query: String,
    board_id: Option<i32>,
) -> AppResult<Vec<serde_json::Value>> {
    search_orchestrator::search(app, query, board_id).await
}

#[tauri::command]
pub async fn ai_rescore(
    app: AppHandle<impl Runtime>,
    query: String,
    previous_results: Vec<serde_json::Value>,
) -> AppResult<Vec<serde_json::Value>> {
    search_orchestrator::rescore(app, query, previous_results).await
}

// Moved to search_service.rs

#[tauri::command] pub async fn select_python_path() -> AppResult<()> { Ok(()) }
