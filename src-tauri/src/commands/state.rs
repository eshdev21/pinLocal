use std::process::{Child, ChildStdin};
use std::sync::Arc;
use parking_lot::Mutex;
use anyhow::Context;
use crossbeam_channel::Receiver;
use crate::workspace::WorkspaceHandle;
use crate::db::DbPool;
use crate::error::{AppResult, AppError};

pub struct AiSidecarState {
    pub process: Option<Child>,
    pub stdin: Option<ChildStdin>,
    pub responses: Option<Receiver<String>>,
    pub config: Option<crate::ai::config::AiConfig>,
    #[cfg(windows)]
    pub job: Option<win32job::Job>,
}

impl AiSidecarState {
    pub fn clear_responses(&mut self) {
        if let Some(rx) = self.responses.as_mut() {
            while rx.try_recv().is_ok() {}
        }
    }

    pub fn send_command(&mut self, action: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        let stdin = self.stdin.as_mut().ok_or_else(|| anyhow::anyhow!("AI Engine not running (stdin missing)"))?;
        let mut req = payload;
        req["action"] = serde_json::json!(action);
        
        use std::io::Write;
        writeln!(stdin, "{}", req).context("Failed to write to AI sidecar stdin")?;
        stdin.flush().context("Failed to flush AI sidecar stdin")?;
        Ok(())
    }

    pub fn recv_response(&mut self, timeout: std::time::Duration) -> AppResult<String> {
        let rx = self.responses.as_mut().ok_or_else(|| AppError::Internal("AI Engine response channel missing".to_string()))?;
        rx.recv_timeout(timeout).map_err(|_| AppError::Internal("AI Engine response timeout".to_string()))
    }

    pub fn encode_text(&mut self, query: &str) -> AppResult<Vec<f32>> {
        self.clear_responses();
        self.send_command("encode_text", serde_json::json!({
            "query_text": query,
            "workspace_root": "" 
        })).map_err(|e| AppError::Internal(e.to_string()))?;
        
        let line = self.recv_response(std::time::Duration::from_secs(30))?;
        let resp: serde_json::Value = serde_json::from_str(&line)?;
        
        if resp["status"] != "ok" {
            return Err(AppError::AiError(resp["message"].as_str().unwrap_or("Failed to encode text").to_string()));
        }
        
        let embedding = resp["embedding"]
            .as_array()
            .ok_or_else(|| AppError::AiError("Invalid embedding format from sidecar".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect::<Vec<f32>>();
            
        Ok(embedding)
    }
}

pub struct AppState {
    pub workspace: Mutex<Option<Arc<WorkspaceHandle>>>,
    pub ai: Mutex<AiSidecarState>,
    pub state_manager: crate::services::state_manager::StateManager<tauri::Wry>,
    pub embedding_cache: crate::ai::vector_search::EmbeddingCache,
}

impl AppState {
    pub fn get_handle(&self) -> AppResult<Arc<WorkspaceHandle>> {
        self.workspace.lock().as_ref().cloned().ok_or_else(|| AppError::NotFound("No active workspace".to_string()))
    }

    pub fn get_pool(&self) -> AppResult<DbPool> {
        Ok(self.get_handle()?.db.clone())
    }

    pub fn get_conn(&self) -> AppResult<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>> {
        Ok(self.get_pool()?.get()?)
    }

    pub fn get_query_vector(&self, query: &str) -> AppResult<Vec<f32>> {
        let mut ai_lock = self.ai.try_lock().ok_or_else(|| {
            AppError::AiError("AI engine is currently busy. Please wait a moment.".to_string())
        })?;
        ai_lock.encode_text(query)
    }

    pub fn get_roots(&self) -> AppResult<Vec<String>> {
        let handle = self.get_handle()?;
        Ok(handle
            .folders
            .iter()
            .map(|p| crate::services::path_utils::WorkspacePath::normalize(p.as_std_path()))
            .collect())
    }

    pub fn get_board_ids(&self) -> AppResult<Vec<i32>> {
        let manager = &self.state_manager;
        let active_id = manager.config().active_workspace_id
            .ok_or_else(|| AppError::NotFound("No active workspace".to_string()))?;
        
        let workspaces = manager.get_workspaces()?;
        let ws = workspaces.into_iter().find(|w| w.id == active_id)
            .ok_or_else(|| AppError::NotFound("Active workspace not found in config".to_string()))?;
            
        Ok(ws.board_ids)
    }
}
