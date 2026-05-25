use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use crate::commands::state::AppState;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelId {
    Siglip2So400m, // google/siglip2-so400m-patch16-naflex (1152d)
    Siglip2Base,   // google/siglip2-base-patch16-naflex (768d)
}

impl ModelId {
    pub fn label(&self) -> &str {
        match self {
            ModelId::Siglip2So400m => "SigLIP 2 SO400M (Highest Accuracy)",
            ModelId::Siglip2Base => "SigLIP 2 Base (Faster / Lighter)",
        }
    }

    pub fn hf_model_id(&self) -> &str {
        match self {
            ModelId::Siglip2So400m => "google/siglip2-so400m-patch16-naflex",
            ModelId::Siglip2Base => "google/siglip2-base-patch16-naflex",
        }
    }

    pub fn dimension(&self) -> i32 {
        match self {
            ModelId::Siglip2So400m => 1152,
            ModelId::Siglip2Base => 768,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum HardwareType {
    #[default]
    #[serde(alias = "auto")]
    Auto,
    #[serde(alias = "nvidia")]
    Nvidia,
    #[serde(alias = "amd")]
    Amd,
    #[serde(alias = "cpu")]
    Cpu,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum CudaVersion {
    #[serde(alias = "v11_8")]
    V11_8,
    #[serde(alias = "v12_1")]
    V12_1,
    #[default]
    #[serde(alias = "v12_4")]
    V12_4,
    #[serde(alias = "v12_6")]
    V12_6,
    #[serde(alias = "v13_0")]
    V13_0,
}

impl CudaVersion {
    pub fn as_str(&self) -> &str {
        match self {
            CudaVersion::V11_8 => "11.8",
            CudaVersion::V12_1 => "12.1",
            CudaVersion::V12_4 => "12.4",
            CudaVersion::V12_6 => "12.6",
            CudaVersion::V13_0 => "13.0",
        }
    }
    
    pub fn as_tag(&self) -> &str {
        match self {
            CudaVersion::V11_8 => "cu118",
            CudaVersion::V12_1 => "cu121",
            CudaVersion::V12_4 => "cu124",
            CudaVersion::V12_6 => "cu126",
            CudaVersion::V13_0 => "cu130",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum PythonVersion {
    #[default]
    #[serde(alias = "auto")]
    Auto,
    #[serde(alias = "v3_10")]
    V3_10,
    #[serde(alias = "v3_11")]
    V3_11,
    #[serde(alias = "v3_12")]
    V3_12,
}

impl PythonVersion {
    pub fn as_str(&self) -> &str {
        match self {
            PythonVersion::Auto => "auto",
            PythonVersion::V3_10 => "3.10",
            PythonVersion::V3_11 => "3.11",
            PythonVersion::V3_12 => "3.12",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum UvLinkMode {
    #[default]
    #[serde(alias = "copy")]
    Copy,
    #[serde(alias = "hardlink")]
    Hardlink,
    #[serde(alias = "symlink")]
    Symlink,
}

impl UvLinkMode {
    pub fn as_arg(&self) -> &str {
        match self {
            UvLinkMode::Copy => "copy",
            UvLinkMode::Hardlink => "hardlink",
            UvLinkMode::Symlink => "symlink",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AiMode {
    #[serde(alias = "auto")]
    Auto,
    #[serde(alias = "manual")]
    Manual,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bon::Builder)]
pub struct AiConfig {
    pub enabled: bool,
    pub mode: AiMode,
    pub venv_path: Option<String>,
    pub model: ModelId,
    pub hardware: HardwareType,
    pub cuda_version: CudaVersion,
    pub python_version: PythonVersion,
    pub link_mode: UvLinkMode,
    pub use_appdata_models: bool,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AiMode::Auto,
            venv_path: None,
            model: ModelId::Siglip2So400m,
            hardware: HardwareType::Auto,
            cuda_version: CudaVersion::V12_4,
            python_version: PythonVersion::Auto,
            link_mode: UvLinkMode::Copy,
            use_appdata_models: true,
        }
    }
}

#[derive(Serialize, Clone, Debug, bon::Builder)]
pub struct PythonStatus {
    pub venv_ready: bool,
    pub model_ready: bool,
    pub python_path: Option<String>,
}

/// Returns AppData/Roaming/pinlocal/python_env/
pub fn python_env_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("app data dir")
        .join("python_env");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn get_python_status<R: Runtime>(app: &AppHandle<R>) -> PythonStatus {
    let state = app.state::<AppState>();
    let manager = &state.state_manager;
    let config = manager.config().ai_config;
    
    let python_exe = match config.mode {
        AiMode::Manual => {
            if let Some(path) = config.venv_path {
                let p = PathBuf::from(path);
                if cfg!(windows) { p.join("Scripts").join("python.exe") } else { p.join("bin").join("python") }
            } else {
                PathBuf::new()
            }
        },
        AiMode::Auto => {
            let venv_dir = python_env_dir(app);
            if cfg!(windows) { venv_dir.join("Scripts").join("python.exe") } else { venv_dir.join("bin").join("python") }
        }
    };

    let venv_ready = python_exe.exists();
    
    PythonStatus {
        venv_ready,
        model_ready: venv_ready,
        python_path: if venv_ready {
            Some(python_exe.to_string_lossy().into())
        } else {
            None
        },
    }
}
