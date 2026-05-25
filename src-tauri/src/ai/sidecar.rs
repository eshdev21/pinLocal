use crate::ai::config::{python_env_dir, AiMode};
use crate::commands::state::AppState;
use anyhow::Context;
use crossbeam_channel::unbounded;
use std::io::BufReader;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub fn kill_sidecar<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let manager = &state.state_manager;
    let mut ai_state = state.ai.lock();

    if let Some(mut child) = ai_state.process.take() {
        let _ = child.kill();
    }
    ai_state.stdin.take();
    ai_state.responses = None;
    #[cfg(windows)]
    ai_state.job.take();
    ai_state.config.take();

    manager.set_engine_status(crate::services::state_manager::EngineStatus::Stopped);
}

pub fn ensure_python_process<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let manager = &state.state_manager;

    // 1. Guard against disabled feature
    let current_config = manager.config().ai_config;
    if !current_config.enabled {
        manager.set_engine_status(crate::services::state_manager::EngineStatus::Disabled);
        anyhow::bail!("AI feature is disabled in settings.");
    }

    {
        let mut ai_state = state.ai.lock();

        // 2. Guard against multiple simultaneous initialization attempts (Race Condition Fix)
        let current_status = manager.engine_status();
        if current_status == crate::services::state_manager::EngineStatus::Starting {
            return Ok(());
        }

        let is_alive = ai_state
            .process
            .as_mut()
            .map(|c| c.try_wait().map(|s| s.is_none()).unwrap_or(false))
            .unwrap_or(false);
        let config_matches = ai_state.config.as_ref() == Some(&current_config);

        if is_alive && config_matches {
            manager.set_engine_status(crate::services::state_manager::EngineStatus::Running);
            return Ok(());
        } else {
            if let Some(mut child) = ai_state.process.take() {
                let _ = child.kill();
            }
            ai_state.stdin.take();
            ai_state.config.take();

            manager.set_engine_status(crate::services::state_manager::EngineStatus::Starting);
        }
    }

    let config = current_config;

    let (python_exe, script_dir) = match config.mode {
        AiMode::Manual => {
            let venv_path = config
                .venv_path
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Manual mode enabled but no venv path provided"))?;
            let p = PathBuf::from(venv_path);
            let exe = if cfg!(windows) {
                p.join("Scripts").join("python.exe")
            } else {
                p.join("bin").join("python")
            };
            if !exe.exists() {
                anyhow::bail!("Python executable not found at {}", exe.display());
            }
            let script_dir = app
                .path()
                .resolve("python", tauri::path::BaseDirectory::Resource)
                .context("Failed to resolve Python script directory")?;
            (exe, script_dir)
        }
        AiMode::Auto => {
            let venv_dir = python_env_dir(app);
            let exe = if cfg!(windows) {
                venv_dir.join("Scripts").join("python.exe")
            } else {
                venv_dir.join("bin").join("python")
            };
            if !exe.exists() {
                anyhow::bail!(
                    "Python environment not initialized. Please click 'Initialize' in Settings."
                );
            }
            let script_dir = app
                .path()
                .resolve("python", tauri::path::BaseDirectory::Resource)
                .context("Failed to resolve Python script directory")?;
            (exe, script_dir)
        }
    };

    let script_path = script_dir.join("interface.py");
    if !script_path.exists() {
        anyhow::bail!("Sidecar script not found at {}", script_path.display());
    }

    let mut command = Command::new(&python_exe);
    command
        .arg("-u")
        .arg(&script_path)
        .arg("--mode")
        .arg("manual")
        .env("HF_HUB_DISABLE_PROGRESS_BARS", "0")
        .env("PYTHONUNBUFFERED", "1")
        .env("TERM", "xterm-256color")
        .current_dir(&script_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if config.mode == crate::ai::config::AiMode::Auto && config.use_appdata_models {
        let models_dir = app
            .path()
            .app_data_dir()
            .context("Failed to get app data directory")?
            .join("models");
        std::fs::create_dir_all(&models_dir).ok();
        command.env("HF_HOME", &models_dir);
        command.env("HUGGINGFACE_HUB_CACHE", &models_dir);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            manager.set_engine_status(crate::services::state_manager::EngineStatus::Error);
            anyhow::bail!("Failed to spawn sidecar: {}", e);
        }
    };

    #[cfg(windows)]
    let mut job_handle = None;

    #[cfg(windows)]
    {
        use win32job::Job;
        if let Ok(job) = Job::create() {
            if let Ok(mut info) = job.query_extended_limit_info() {
                info.limit_kill_on_job_close();
                if job.set_extended_limit_info(&info).is_ok() {
                    let _ = job.assign_process(child.as_raw_handle() as isize);
                    job_handle = Some(job);
                }
            }
        }
    }

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stderr"))?;

    let (resp_tx, resp_rx) = unbounded::<String>();
    let (ready_tx, ready_rx) = unbounded::<()>();

    // STDOUT Reader
    spawn_stream_reader(app.clone(), stdout, Some(resp_tx.clone()), Some(ready_tx.clone()));

    // STDERR Reader
    spawn_stream_reader(app.clone(), stderr, None, None);

    match ready_rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(_) => {
            let pid = child.id();
            let mut ai_state = state.ai.lock();
            ai_state.process = Some(child);
            ai_state.stdin = Some(stdin);
            ai_state.responses = Some(resp_rx);
            ai_state.config = Some(config.clone());
            #[cfg(windows)]
            {
                ai_state.job = job_handle;
            }

            manager.set_pid(pid);
            manager.set_engine_status(crate::services::state_manager::EngineStatus::Running);
            app.emit("ai:log", "Sidecar engine ready.").ok();
            app.emit("python:ready", true).ok();
            Ok(())
        }
        Err(_) => {
            let _ = child.kill();
            manager.set_engine_status(crate::services::state_manager::EngineStatus::Error);
            anyhow::bail!("Sidecar failed to initialize within 30 seconds")
        }
    }
}

fn spawn_stream_reader<R: Runtime, T: std::io::Read + Send + 'static>(
    app: AppHandle<R>,
    reader: T,
    resp_tx: Option<crossbeam_channel::Sender<String>>,
    ready_tx: Option<crossbeam_channel::Sender<()>>,
) {
    std::thread::spawn(move || {
        use std::io::Read;
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        let mut byte_buf = [0u8; 1024];

        while let Ok(n) = reader.read(&mut byte_buf) {
            if n == 0 {
                break;
            }
            for &b in &byte_buf[..n] {
                if b == b'\n' || b == b'\r' {
                    if !buffer.is_empty() {
                        let line = String::from_utf8_lossy(&buffer).to_string();
                        let trimmed = line.trim();
                        if trimmed == "READY" {
                            if let Some(tx) = &ready_tx {
                                tx.send(()).ok();
                            }
                        } else if trimmed.starts_with('{') {
                            if let Some(tx) = &resp_tx {
                                tx.send(line.clone()).ok();
                            }
                        } else if trimmed.starts_with("[LOG]") {
                            let msg = trimmed.trim_start_matches("[LOG]").trim();
                            app.emit("ai:log", msg.to_string()).ok();
                        } else if trimmed.to_lowercase().contains("error") {
                            app.emit("ai:log", format!("CRITICAL: {}", trimmed)).ok();
                        }
                        buffer.clear();
                    }
                } else {
                    buffer.push(b);
                }
            }
        }
    });
}
