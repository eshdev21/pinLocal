pub mod commands;
pub mod db;
pub mod ai;
pub mod error;
pub mod services;
pub mod workspace;
pub mod config;

use parking_lot::Mutex;
use tauri::Manager;
use tauri_plugin_fs::FsExt;
use crate::workspace::WorkspaceHandle;
use log::{info, error};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_log::Builder::new()
            .targets([
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: Some("app".into()) }),
            ])
            .level(log::LevelFilter::Info)
            .max_file_size(10 * 1024 * 1024)
            .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
            .build())
        .setup(|app| {
            info!(">>> SESSION STARTED [v{}] [{}-{}] <<<", 
                app.package_info().version, 
                std::env::consts::OS, 
                std::env::consts::ARCH
            );

            // 1. Initialize DB and StateManager first
            info!("Initializing Database Pool...");
            let db_pool = crate::db::get_pool(app.handle()).map_err(|e| {
                error!("CRITICAL: Failed to initialize database pool: {}", e);
                e
            })?;
            
            info!("Initializing State Manager...");
            let state_manager = crate::services::state_manager::StateManager::new(app.handle().clone(), db_pool);
            
            // 1.2 Heal ghost tasks from previous session
            state_manager.heal_tasks().ok();

            // 2. Register initial AppState (without workspace handle yet)
            let state = crate::commands::state::AppState {
                workspace: Mutex::new(None),
                ai: Mutex::new(crate::commands::state::AiSidecarState {
                    process: None,
                    stdin: None,
                    responses: None,
                    config: None,
                    #[cfg(windows)]
                    job: None,
                }),
                state_manager: state_manager.clone(),
                embedding_cache: crate::ai::vector_search::EmbeddingCache::new(),
            };
            app.manage(state);

            // 3. Restore Active Workspace
            restore_active_workspace(app, &state_manager);
            
            // 4. Start the AI maintenance worker.
            // This keeps the AI DB inventory clean, but does not generate embeddings.
            crate::ai::sync_worker::spawn_background_worker(app.handle().clone()).ok();
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace::get_config,
            commands::workspace::set_active_workspace,
            commands::workspace::add_workspace,
            commands::workspace::rename_workspace,
            commands::workspace::remove_workspace,
            commands::workspace::add_folders_to_workspace,
            commands::workspace::remove_board_from_workspace,
            commands::workspace::scan_workspace,
            commands::boards::get_boards,
            commands::boards::create_board,
            commands::boards::delete_board,
            commands::boards::cleanup_orphaned_boards,
            commands::images::get_images,
            commands::images::get_image,
            commands::images::delete_image,
            commands::images::open_in_explorer,
            commands::images::import_images,
            commands::workspace::get_storage_stats,
            commands::workspace::is_scanning,
            commands::workspace::get_workspace_status,
            commands::ai::get_ai_config,
            commands::ai::set_ai_enabled,
            commands::ai::set_ai_model,
            commands::ai::set_ai_hardware,
            commands::ai::set_cuda_version,
            commands::ai::set_python_version,
            commands::ai::set_link_mode,
            commands::ai::set_use_appdata_models,
            commands::ai::set_ai_mode,
            commands::ai::set_venv_path,
            commands::ai::select_venv_path,
            commands::ai::setup_siglip,
            commands::ai::kill_sidecar,
            commands::ai::load_model,
            commands::ai::get_ai_runtime_status,
            commands::ai::get_app_status,
            commands::ai::generate_embeddings,
            commands::ai::cancel_indexing,
            commands::ai::reset_embeddings,
            commands::ai::select_python_path,
            commands::ai::ai_search,
            commands::ai::ai_rescore,
            commands::logging::open_logs_folder,
            commands::logging::clear_logs,
            commands::logging::set_logging_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Restores the last active workspace on application startup.
fn restore_active_workspace(app: &mut tauri::App, state_manager: &crate::services::state_manager::StateManager<tauri::Wry>) {
    info!("Fetching active workspace ID from database...");
    let active_id = state_manager.config().active_workspace_id;
    
    if let Some(id) = active_id {
        info!("Found active workspace ID: {}. Attempting to open...", id);
        let workspaces = state_manager.get_workspaces().unwrap_or_default();

        if let Some(ws_config) = workspaces.into_iter().find(|w| w.id == id) {
            info!("Workspace config found for {}. Registering FS scopes...", ws_config.name);
            for folder in &ws_config.folder_paths {
                app.handle().fs_scope().allow_directory(folder, true).ok();
            }

            if let Ok(handle) = WorkspaceHandle::open(app.handle().clone(), ws_config) {
                info!("Workspace handle opened successfully.");
                let state = app.state::<crate::commands::state::AppState>();
                state.embedding_cache.clear();
                let mut ws = state.workspace.lock();
                *ws = Some(handle);
            } else {
                error!("Failed to open workspace handle for ID: {}", id);
            }
        } else {
            log::warn!("Active workspace ID {} found in state, but no workspace config exists!", id);
        }
    } else {
        log::info!("No active workspace found in database (Fresh Start).");
    }
}
