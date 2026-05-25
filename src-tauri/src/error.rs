use miette::Diagnostic;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),

    #[error("Database pool error: {0}")]
    DatabaseError(#[from] r2d2::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Tauri error: {0}")]
    TauriError(String),

    #[error("Workspace error: {0}")]
    #[diagnostic(code(pinlocal::workspace::error))]
    #[help("Check if the workspace folders still exist and are accessible.")]
    WorkspaceError(String),

    #[error("AI error: {0}")]
    #[diagnostic(code(pinlocal::ai::error))]
    #[help("Ensure the AI sidecar is running and the Python environment is valid.")]
    AiError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::TauriError(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
