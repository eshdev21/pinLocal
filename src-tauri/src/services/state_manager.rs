use crate::ai::config::AiConfig;
use crate::db::DbPool;
use crate::error::AppResult;
use arc_swap::ArcSwap;
use delegate::delegate;
use getset::Getters;
use itertools::Itertools;
use parking_lot::RwLock;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Serialize, Deserialize, Clone, Debug, bon::Builder)]
pub struct BackgroundTask {
    #[builder(into)]
    pub id: String,
    #[builder(into)]
    pub task_type: String,
    #[builder(into)]
    pub status: String,
    #[builder(into)]
    pub message: Option<String>,
    pub progress: i64,
    pub total: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Getters, bon::Builder)]
#[getset(get = "pub")]
pub struct AppConfig {
    #[builder(into)]
    pub active_workspace_id: Option<String>,
    pub ai_config: AiConfig,
    pub logging_enabled: bool,
    pub setup_completed: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_workspace_id: None,
            ai_config: AiConfig::default(),
            logging_enabled: true,
            setup_completed: false,
        }
    }
}



pub use crate::services::ai_lifecycle::{AiStateMachine, EngineStatus, ModelStatus};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppStatus {
    pub active_workspace_id: Option<String>,
    pub ai_config: AiConfig,
    pub logging_enabled: bool,
    pub setup_completed: bool,
    pub active_tasks: Vec<BackgroundTask>,
    pub is_scanning: bool,
    pub ai_engine_status: EngineStatus,
    pub ai_model_status: ModelStatus,
    pub pid: u32,
}

struct PulseLoopGuard {
    cancel: Arc<AtomicBool>,
}

impl Drop for PulseLoopGuard {
    fn drop(&mut self) {
        log::info!("PulseLoopGuard dropping, signaling pulse and worker loops to stop.");
        self.cancel.store(true, Ordering::SeqCst);
    }
}

pub struct TransientState {
    pub ai: AiStateMachine,
    pub is_scanning: Arc<AtomicBool>,
    pub is_ai_worker_running: Arc<AtomicBool>,
    pub cancel_pulse: Arc<AtomicBool>,
    pub active_tasks: Arc<RwLock<HashMap<String, BackgroundTask>>>,
    _pulse_loop_guard: PulseLoopGuard,
}

impl TransientState {
    pub fn new(cancel_pulse: Arc<AtomicBool>, initial_engine_status: EngineStatus) -> Self {
        Self {
            ai: AiStateMachine::new(initial_engine_status),
            is_scanning: Arc::new(AtomicBool::new(false)),
            is_ai_worker_running: Arc::new(AtomicBool::new(false)),
            cancel_pulse: cancel_pulse.clone(),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            _pulse_loop_guard: PulseLoopGuard {
                cancel: cancel_pulse,
            },
        }
    }

    pub fn store_scanning(&self) -> bool {
        !self.is_scanning.swap(true, Ordering::SeqCst)
    }

    pub fn store_not_scanning(&self) -> bool {
        self.is_scanning.swap(false, Ordering::SeqCst)
    }

    pub fn store_engine_status(&self, status: EngineStatus) -> bool {
        self.ai.transition_engine(status)
    }

    pub fn store_model_status(&self, status: ModelStatus) -> bool {
        self.ai.transition_model(status)
    }

    pub fn is_scanning(&self) -> bool {
        self.is_scanning.load(Ordering::SeqCst)
    }

    pub fn engine_status(&self) -> EngineStatus {
        self.ai.engine()
    }

    pub fn model_status(&self) -> ModelStatus {
        self.ai.model()
    }

    pub fn pid(&self) -> u32 {
        self.ai.pid()
    }

    pub fn store_pid(&self, pid: u32) {
        self.ai.set_pid(pid);
    }

    pub fn is_ai_worker_running(&self) -> bool {
        self.is_ai_worker_running.load(Ordering::SeqCst)
    }

    pub fn claim_ai_worker_slot(&self) -> bool {
        !self.is_ai_worker_running.swap(true, Ordering::SeqCst)
    }

    pub fn is_pulse_cancelled(&self) -> bool {
        self.cancel_pulse.load(Ordering::SeqCst)
    }
}

pub struct StateManager<R: Runtime> {
    app: AppHandle<R>,
    db: DbPool,
    pub transient: Arc<TransientState>,
    dirty: Arc<AtomicBool>,
    config_cache: Arc<ArcSwap<AppConfig>>,
    cancel_pulse: Arc<AtomicBool>,
}

impl<R: Runtime> Clone for StateManager<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            db: self.db.clone(),
            transient: self.transient.clone(),
            dirty: self.dirty.clone(),
            config_cache: self.config_cache.clone(),
            cancel_pulse: self.cancel_pulse.clone(),
        }
    }
}

impl<R: Runtime> StateManager<R> {
    pub fn new(app: AppHandle<R>, db: DbPool) -> Self {
        let dirty = Arc::new(AtomicBool::new(true)); // Start dirty to emit initial pulse
        let cancel_pulse = Arc::new(AtomicBool::new(false));
        let initial_config = {
            let conn = db.get().ok();
            conn.and_then(|c| {
                c.query_row(
                    "SELECT value FROM app_state WHERE key = 'config:v1'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .ok()
                .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
            })
            .unwrap_or_default()
        };

        let engine_status = if initial_config.ai_config.enabled {
            EngineStatus::Stopped
        } else {
            EngineStatus::Disabled
        };
        let transient = Arc::new(TransientState::new(cancel_pulse.clone(), engine_status));
        let config_cache = Arc::new(ArcSwap::new(Arc::new(initial_config)));

        let manager = Self {
            app: app.clone(),
            db: db.clone(),
            transient: transient.clone(),
            dirty: dirty.clone(),
            config_cache: config_cache.clone(),
            cancel_pulse: cancel_pulse.clone(),
        };

        // Start the Pulse Loop (Heartbeat)
        let pulse_manager = manager.clone();
        std::thread::spawn(move || loop {
            if pulse_manager.cancel_pulse.load(Ordering::SeqCst) {
                log::info!("StateManager Pulse Loop exiting.");
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(150));

            if pulse_manager.dirty.swap(false, Ordering::SeqCst) {
                if let Ok(status) = pulse_manager.get_app_status() {
                    pulse_manager.app.emit("app:sync", status).ok();
                }
            }
        });

        manager
    }

    /// Fetches the global configuration (Lock-Free).
    pub fn config(&self) -> AppConfig {
        (**self.config_cache.load()).clone()
    }

    /// Updates the global configuration using RCU (Read-Copy-Update).
    pub fn update_config<F>(&self, f: F) -> AppResult<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        // 1. Prepare the new config by copying the old one
        let mut new_config = self.config();
        f(&mut new_config);

        let val_str = serde_json::to_string(&new_config)?;

        // 2. Persist to Database
        let conn = self.db.get()?;
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES ('config:v1', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![val_str],
        )?;

        // 3. Atomically swap the pointer in memory
        self.config_cache.store(Arc::new(new_config));

        self.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn get_app_status(&self) -> AppResult<AppStatus> {
        let config = self.config();
        let tasks = self.get_active_tasks().unwrap_or_default();

        Ok(AppStatus {
            active_workspace_id: config.active_workspace_id,
            ai_config: config.ai_config,
            logging_enabled: config.logging_enabled,
            setup_completed: config.setup_completed,
            active_tasks: tasks,
            is_scanning: self.is_scanning(),
            ai_engine_status: self.engine_status(),
            ai_model_status: self.model_status(),
            pid: self.pid(),
        })
    }

    pub fn update_task(
        &self,
        id: &str,
        task_type: &str,
        status: &str,
        message: Option<&str>,
        progress: i64,
        total: i64,
    ) -> AppResult<()> {
        let task = BackgroundTask::builder()
            .id(id)
            .task_type(task_type)
            .status(status)
            .maybe_message(message.map(String::from))
            .progress(progress)
            .total(total)
            .updated_at(chrono::Utc::now().timestamp())
            .build();

        self.transient
            .active_tasks
            .write()
            .insert(id.to_string(), task);

        self.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn finish_task(&self, id: &str, status: &str, message: Option<&str>) -> AppResult<()> {
        let removed_task = self.transient.active_tasks.write().remove(id);

        let display_message = message.map(|s| s.to_string()).or_else(|| {
            removed_task.map(|t| {
                use heck::ToTitleCase;
                format!("{} completed", t.task_type.to_title_case())
            })
        });

        self.app
            .emit(
                "app:task-finished",
                json!({ "id": id, "status": status, "message": display_message }),
            )
            .ok();
        self.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn heal_tasks(&self) -> AppResult<()> {
        self.transient.active_tasks.write().clear();
        self.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn get_active_tasks(&self) -> AppResult<Vec<BackgroundTask>> {
        let tasks = self.transient.active_tasks.read();
        Ok(tasks
            .values()
            .cloned()
            .sorted_by(|a, b| b.updated_at.cmp(&a.updated_at))
            .collect())
    }

    // --- Workspace Management ---

    pub fn upsert_workspace(&self, id: &str, name: &str, board_ids: &[i32]) -> AppResult<()> {
        let mut conn = self.db.get()?;
        crate::db::workspaces::upsert_workspace(&mut conn, id, name, board_ids)?;

        self.app
            .emit(
                "app:workspace-updated",
                json!({ "id": id, "name": name, "board_ids": board_ids }),
            )
            .ok();
        Ok(())
    }

    pub fn get_workspaces(&self) -> AppResult<Vec<crate::config::Workspace>> {
        let conn = self.db.get()?;
        crate::db::workspaces::get_workspaces(&conn)
    }

    pub fn remove_workspace(&self, id: &str) -> AppResult<()> {
        let conn = self.db.get()?;
        crate::db::workspaces::remove_workspace(&conn, id)?;

        self.app
            .emit("app:workspace-removed", json!({ "id": id }))
            .ok();
        Ok(())
    }

    // --- Cancellation API ---

    pub fn stop_pulse(&self) {
        self.cancel_pulse.store(true, Ordering::SeqCst);
    }

    // --- Type-safe Helpers ---

    pub fn set_scanning(&self, scanning: bool) {
        let changed = if scanning {
            self.transient.store_scanning()
        } else {
            self.transient.store_not_scanning()
        };
        if changed {
            log::info!(
                "Scanning: {}",
                if scanning { "Started" } else { "Finished" }
            );
            self.dirty.store(true, Ordering::SeqCst);
        }
    }

    pub fn set_engine_status(&self, status: EngineStatus) {
        if self.transient.store_engine_status(status) {
            self.dirty.store(true, Ordering::SeqCst);
        }
    }

    pub fn set_model_status(&self, status: ModelStatus) {
        if self.transient.store_model_status(status) {
            self.dirty.store(true, Ordering::SeqCst);
        }
    }

    pub fn set_pid(&self, pid: u32) {
        self.transient.store_pid(pid);
        self.dirty.store(true, Ordering::SeqCst);
    }

    delegate! {
        to self.transient {
            pub fn is_scanning(&self) -> bool;
            pub fn engine_status(&self) -> EngineStatus;
            pub fn model_status(&self) -> ModelStatus;
            pub fn pid(&self) -> u32;
            pub fn is_ai_worker_running(&self) -> bool;
            pub fn claim_ai_worker_slot(&self) -> bool;
            pub fn is_pulse_cancelled(&self) -> bool;
        }
    }

    pub fn set_board_ai_sync(&self, board_id: i32, enabled: bool) -> AppResult<()> {
        let conn = self.db.get()?;
        crate::db::workspaces::set_board_ai_sync(&conn, board_id, enabled)?;
        self.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn get_pending_ai_boards(&self) -> AppResult<Vec<(i32, String)>> {
        let conn = self.db.get()?;
        crate::db::workspaces::get_pending_ai_boards(&conn)
    }

    pub fn set_active_workspace(&self, id: Option<String>) -> AppResult<()> {
        self.update_config(|c| {
            c.active_workspace_id = id;
        })
    }
}
