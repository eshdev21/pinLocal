use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, Runtime};

use crate::commands::state::AppState;
use crate::error::AppResult;
use crate::ai::indexing_service::fetch_board_inventory;
use crate::ai::embeddings_store;

struct WorkerGuard(Arc<std::sync::atomic::AtomicBool>);
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
        log::info!("AI maintenance worker stopped.");
    }
}

pub fn spawn_background_worker<R: Runtime>(app: AppHandle<R>) -> AppResult<()> {
    let state = app.state::<crate::commands::state::AppState>();
    if !state.state_manager.claim_ai_worker_slot() {
        return Ok(());
    }

    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let _guard = WorkerGuard(state.state_manager.transient.is_ai_worker_running.clone());

        log::info!("AI maintenance worker started.");

        loop {
            if state.state_manager.is_pulse_cancelled() {
                log::info!("AI maintenance worker received cancellation signal.");
                break;
            }

            if state.state_manager.get_app_status().is_err() {
                break;
            }

            if state.get_handle().is_err() {
                std::thread::sleep(std::time::Duration::from_secs(15));
                continue;
            }

            if !state.state_manager.config().ai_config.enabled {
                std::thread::sleep(std::time::Duration::from_secs(15));
                continue;
            }

            let board_ids = state.get_board_ids().unwrap_or_default();

            let dirty_boards: Vec<(i32, String)> = state
                .state_manager
                .get_pending_ai_boards()
                .unwrap_or_default()
                .into_iter()
                .filter(|(board_id, _)| board_ids.contains(board_id))
                .collect();
            if dirty_boards.is_empty() {
                std::thread::sleep(std::time::Duration::from_secs(15));
                continue;
            }

            for (board_id, board_path) in dirty_boards {
                log::info!("AI maintenance: reconciling board {}", board_path);

                let board_images = match fetch_board_inventory(&state, board_id) {
                    Ok(images) => images,
                    Err(e) => {
                        log::error!(
                            "AI maintenance: failed to fetch images for board {}: {}",
                            board_path,
                            e
                        );
                        continue;
                    }
                };

                if let Err(e) = embeddings_store::sync_inventory(&app, &board_path, board_images, Vec::new(), true) {
                    log::error!(
                        "AI maintenance: inventory sync failed for {}: {}",
                        board_path,
                        e
                    );
                    continue;
                }

                state.state_manager.set_board_ai_sync(board_id, false).ok();
            }

            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });

    Ok(())
}
