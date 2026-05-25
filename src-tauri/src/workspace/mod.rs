use crate::db::DbPool;
use crate::error::AppResult;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::{AppHandle, Runtime};
use camino::Utf8PathBuf;

pub mod reconciler_actor;
pub mod thumbnail_actor;
pub mod watcher_actor;

pub struct WorkspaceHandle {
    pub id: String,
    pub name: String,
    pub folders: Vec<Utf8PathBuf>,
    pub db: DbPool,
    pub reconciler_tx: std::sync::mpsc::Sender<()>,
    pub thumb_nudge_tx: std::sync::mpsc::Sender<()>,
    pub actors: Mutex<Vec<std::thread::JoinHandle<()>>>,
    pub cancel: Arc<AtomicBool>,
    pub cancel_ai: Arc<AtomicBool>,
}

impl WorkspaceHandle {
    /// Opens workspace using logical configuration.
    pub fn open<R: Runtime>(
        app: AppHandle<R>,
        ws_config: crate::config::Workspace,
    ) -> AppResult<Arc<Self>> {
        log::info!("Opening workspace: {} ({})", ws_config.name, ws_config.id);

        let id = ws_config.id.clone();
        let folders: Vec<Utf8PathBuf> = ws_config.folder_paths.iter().map(Utf8PathBuf::from).collect();

        // Register FS scope for all workspace folders
        use tauri_plugin_fs::FsExt;
        for folder in &folders {
            app.fs_scope().allow_directory(folder.as_std_path(), true).ok();
        }

        // 1. Open single connection for heal pass
        let conn = crate::db::get_connection(&app)?;

        // Heal pass
        log::info!("Running heal pass for workspace: {}", ws_config.name);
        Self::heal(&conn)?;
        drop(conn);

        // 2. Create pool for normal operation
        let db = crate::db::get_pool(&app)?;

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_ai = Arc::new(AtomicBool::new(false));

        let (thumb_nudge_tx, thumb_handle) = crate::workspace::thumbnail_actor::ThumbnailActor::spawn(
            app.clone(),
            db.clone(),
            cancel.clone(),
        )?;

        let (reconciler_tx, reconciler_handle) =
            crate::workspace::reconciler_actor::ReconcilerActor::spawn(
                app.clone(),
                folders.clone(),
                db.clone(),
                thumb_nudge_tx.clone(),
                cancel.clone(),
            )?;

        let watcher_handle = crate::workspace::watcher_actor::WatcherActor::spawn(
            app.clone(),
            folders.clone(),
            reconciler_tx.clone(),
            cancel.clone(),
        )?;

        let checkpoint_handle =
            Self::spawn_checkpoint_actor(app.clone(), db.clone(), cancel.clone());

        use tap::{Tap, Pipe};

        Arc::new(Self {
            id: id.clone(),
            name: ws_config.name.clone(),
            folders: folders.clone(),
            db: db.clone(),
            reconciler_tx: reconciler_tx.clone(),
            thumb_nudge_tx: thumb_nudge_tx.clone(),
            actors: Mutex::new(Vec::new()),
            cancel: cancel.clone(),
            cancel_ai: cancel_ai.clone(),
        })
        .tap(|h| {
            let mut actors = h.actors.lock();
            actors.push(thumb_handle);
            actors.push(reconciler_handle);
            actors.push(watcher_handle);
            actors.push(checkpoint_handle);
        })
        .tap(|h| {
            h.reconciler_tx.send(()).ok();
            thumb_nudge_tx.send(()).ok();
        })
        .pipe(Ok)
    }

    fn spawn_checkpoint_actor<R: Runtime>(
        _app: AppHandle<R>,
        db: DbPool,
        cancel: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            if let Err(panic_info) =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    loop {
                        // Check every 5 minutes
                        for _ in 0..300 {
                            if cancel.load(Ordering::SeqCst) {
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                        if cancel.load(Ordering::SeqCst) {
                            return;
                        }

                        if let Ok(conn) = db.get() {
                            // PASSIVE checkpoint — never blocks, merges WAL into main DB when possible
                            match conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);") {
                                Ok(_) => log::info!("WAL checkpoint complete."),
                                Err(e) => log::error!("WAL checkpoint failed: {}", e),
                            }
                        }
                    }
                }))
            {
                log::error!("CheckpointActor PANICKED: {:?}", panic_info);
            }
        })
    }

    /// Explicitly shuts down background workers and waits for them to exit.
    pub fn shutdown<R: Runtime>(&self, _app: &AppHandle<R>) {
        log::info!("Explicit shutdown requested for workspace: {}", self.name);
        self.cancel.store(true, Ordering::SeqCst);
        self.cancel_ai.store(true, Ordering::SeqCst);

        // Wake up ThumbnailActor in case it is waiting on rx.recv()
        let _ = self.thumb_nudge_tx.send(());

        let mut actors = self.actors.lock();
        let total = actors.len();
        log::info!("Waiting for {} actors to exit...", total);

        while let Some(handle) = actors.pop() {
            if let Err(e) = handle.join() {
                log::error!("Error joining actor thread: {:?}", e);
            }
        }

        // Final database checkpoint to ensure all WAL data is merged before exit
        if let Ok(conn) = self.db.get() {
            log::info!(
                "Performing final WAL checkpoint for workspace: {}",
                self.name
            );
            match conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
                Ok(_) => log::info!("Final WAL checkpoint complete."),
                Err(e) => log::error!("Final WAL checkpoint failed: {}", e),
            }
        }

        log::info!("All actors for workspace {} have exited.", self.name);
    }

    /// Called by workers to check if they should exit.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    fn heal(db: &Connection) -> AppResult<()> {
        // Reset generating status on startup
        db.execute(
            "UPDATE images SET thumbnail_status='pending', thumb_path=NULL WHERE thumbnail_status='generating'",
            []
        ).ok();

        Ok(())
    }
}

impl Drop for WorkspaceHandle {
    fn drop(&mut self) {
        log::info!(
            "WorkspaceHandle dropped: {}. Signalling cancellation to actors.",
            self.name
        );
        self.cancel.store(true, Ordering::SeqCst);
        self.cancel_ai.store(true, Ordering::SeqCst);
        
        // Wake up ThumbnailActor in case it is waiting on rx.recv()
        let _ = self.thumb_nudge_tx.send(());
    }
}
