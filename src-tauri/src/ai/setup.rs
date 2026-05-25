use crate::ai::config::{get_python_status, python_env_dir, PythonVersion};
use crate::ai::sidecar::kill_sidecar;
use crate::commands::state::AppState;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use anyhow::Context;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub fn setup_siglip<R: Runtime>(app: AppHandle<R>) -> anyhow::Result<()> {
    // 1. Kill any running sidecar
    kill_sidecar(&app);

    let state = app.state::<AppState>();
    let manager = &state.state_manager;
    let config = manager.config().ai_config;

    app.emit("ai:log", "Starting AI environment setup...").ok();

    // 2. Check for 'uv'
    if Command::new("uv").arg("--version").output().is_err() {
        anyhow::bail!("The 'uv' tool was not found on your system. Please install it from https://astral.sh/uv to continue.");
    }

    // 4. Run setup.py via uv
    let venv_dir = python_env_dir(&app);
    let script_dir = app
        .path()
        .resolve("python", tauri::path::BaseDirectory::Resource)
        .context("Failed to resolve Python script directory")?;
    let setup_script = script_dir.join("setup.py");

    if !setup_script.exists() {
        anyhow::bail!("Setup script not found at {}", setup_script.display());
    }

    app.emit(
        "ai:log",
        format!(
            "Creating/Updating virtual environment in {}...",
            venv_dir.display()
        ),
    )
    .ok();

    let mut command = Command::new("uv");
    command.arg("run");

    if config.python_version != PythonVersion::Auto {
        command.arg("--python").arg(config.python_version.as_str());
    }

    command
        .arg(&setup_script)
        .arg("--venv")
        .arg(&venv_dir)
        .arg("--hardware")
        .arg(match config.hardware {
            crate::ai::config::HardwareType::Auto => "auto",
            crate::ai::config::HardwareType::Nvidia => "nvidia",
            crate::ai::config::HardwareType::Amd => "amd",
            crate::ai::config::HardwareType::Cpu => "cpu",
        })
        .arg("--cuda")
        .arg(config.cuda_version.as_tag())
        .arg("--link-mode")
        .arg(config.link_mode.as_arg())
        .current_dir(&script_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .context("Failed to start setup process")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let app_logs = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            app_logs.emit("ai:log", line).ok();
        }
    });

    let app_errs = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            app_errs.emit("ai:log", line).ok();
        }
    });

    let status = child
        .wait()
        .context("Setup process failed during execution")?;
    if !status.success() {
        anyhow::bail!("AI environment setup failed. Check the logs above for details.");
    }

    app.emit("ai:log", "Setup completed successfully.".to_string())
        .ok();

    // 5. Verify venv health
    let status = get_python_status(&app);
    if status.venv_ready {
        app.emit("python:ready", status).ok();
    } else {
        anyhow::bail!("Venv health check failed after setup.");
    }

    Ok(())
}
