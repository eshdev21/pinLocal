use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, Runtime};

// use crate::ai::config::ModelId; (removed unused)
use crate::ai::sidecar::ensure_python_process;
use crate::commands::state::AppState;
use crate::error::{AppError, AppResult};
use crate::ai::indexing_service::PythonResponse;

pub fn cancel_indexing<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    if let Ok(handle) = state.get_handle() {
        handle.cancel_ai.store(true, Ordering::SeqCst);
    }
}

pub fn ensure_model_loaded<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let manager = &state.state_manager;

    // 1. Check if AI is enabled first
    let config = manager.config().ai_config;
    if !config.enabled {
        return Err(AppError::AiError(
            "AI is currently disabled in settings.".into(),
        ));
    }

    // 1. Check if already ready or loading
    let current_status = manager.model_status();
    if current_status == crate::services::state_manager::ModelStatus::Ready {
        return Ok(());
    }
    if current_status == crate::services::state_manager::ModelStatus::Loading {
        return Ok(());
    }

    ensure_python_process(app)?;

    app.emit(
        "ai:log",
        format!(">>> Loading Model: {}...", config.model.label()),
    )
    .ok();
    manager.set_model_status(crate::services::state_manager::ModelStatus::Loading);

    {
        let mut ai_lock = state.ai.lock();

        let device = match config.hardware {
            crate::ai::config::HardwareType::Auto => "auto",
            crate::ai::config::HardwareType::Nvidia => "cuda",
            crate::ai::config::HardwareType::Amd => "dml",
            crate::ai::config::HardwareType::Cpu => "cpu",
        };

        ai_lock.send_command(
            "load_model",
            serde_json::json!({
                "model_id": config.model.hf_model_id(),
                "device": device,
                "max_patches": 256
            }),
        )?;

        let line = ai_lock.recv_response(std::time::Duration::from_secs(120))?;
        let resp: PythonResponse = serde_json::from_str(&line)?;

        match resp {
            PythonResponse::Ok { .. } => {
                // Success
            }
            PythonResponse::Error { message } => {
                manager.set_model_status(crate::services::state_manager::ModelStatus::Error);
                return Err(AppError::AiError(message));
            }
            _ => {
                manager.set_model_status(crate::services::state_manager::ModelStatus::Error);
                return Err(AppError::AiError(
                    "Unexpected response during model load".into(),
                ));
            }
        }
    }

    manager.set_model_status(crate::services::state_manager::ModelStatus::Ready);
    app.emit("ai:log", ">>> AI Model Ready!".to_string()).ok();

    Ok(())
}
