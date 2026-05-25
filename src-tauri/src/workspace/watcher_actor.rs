use camino::Utf8PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tauri::{AppHandle, Runtime, Manager};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use crate::error::AppResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct WatcherActor;

impl WatcherActor {
    pub fn spawn<R: Runtime>(
        app: AppHandle<R>, 
        folders: Vec<Utf8PathBuf>, 
        reconciler_tx: Sender<()>,
        cancel: Arc<AtomicBool>,
    ) -> AppResult<std::thread::JoinHandle<()>> {
        let handle = std::thread::spawn(move || {
            if let Err(panic_info) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let state = app.state::<crate::commands::state::AppState>();
                let manager = &state.state_manager;
                let (tx, rx) = std::sync::mpsc::channel();
                
                let mut debouncer = match new_debouncer(
                    Duration::from_millis(2000), 
                    move |res: DebounceEventResult| { tx.send(res).ok(); }
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Watcher initialization failed: {:?}", e);
                        return;
                    }
                };

                let watcher = debouncer.watcher();
                for root in &folders {
                    log::info!("Watcher: Starting for {:?}", root);
                    if let Err(e) = watcher.watch(root.as_std_path(), RecursiveMode::Recursive) {
                        log::error!("Watcher failed for {:?}: {:?}", root, e);
                    }
                }

                while !cancel.load(Ordering::SeqCst) {
                    // Check for events every 200ms to stay responsive to cancel signal
                    if let Ok(result) = rx.recv_timeout(Duration::from_millis(200)) {
                        match result {
                            Ok(events) => {
                                if events.iter().any(|e| !Self::should_ignore(e.path.as_path())) {
                                    log::info!("Watcher: Change detected. Triggering reconciliation.");
                                    manager.update_task("fs-watcher", "watcher", "running", Some("File system change detected..."), 0, 0).ok();
                                    
                                    reconciler_tx.send(()).ok();
                                    manager.finish_task("fs-watcher", "completed", None).ok();
                                }
                            }
                            Err(e) => {
                                log::error!("Watcher error: {:?}", e);
                            }
                        }
                    }
                }
                log::info!("Watcher: Stopped monitoring all folders.");
            })) {
                log::error!("WatcherActor PANICKED: {:?}", panic_info);
            }
        });

        Ok(handle)
    }

    fn should_ignore(path: &std::path::Path) -> bool {
        let path_str = crate::services::path_utils::WorkspacePath::normalize(path).to_lowercase();
        
        // Ignore common junk folders
        if path_str.contains(".git") || path_str.contains("node_modules") || path_str.contains(".ds_store") {
            return true;
        }
        
        // Ignore hidden files
        if path.file_name().map(|n| n.to_string_lossy().starts_with('.')).unwrap_or(false) {
            return true;
        }

        false
    }
}
