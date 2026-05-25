use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use crate::error::{AppResult, AppError};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub mod schema;
pub mod boards;
pub mod images;
pub mod workspaces;
pub mod migrations;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn get_pool<R: Runtime>(app: &AppHandle<R>) -> AppResult<DbPool> {
    let db_path = get_db_path(app)?;
    log::info!("Initializing Database Pool at: {:?}", db_path);

    ensure_db_initialized(&db_path)?;
    
    let manager = SqliteConnectionManager::file(&db_path)
        .with_init(|conn| setup_pragmas(conn));

    Ok(Pool::builder()
        .max_size(4)
        .build(manager)?)
}

pub fn get_connection<R: Runtime>(app: &AppHandle<R>) -> AppResult<rusqlite::Connection> {
    let db_path = get_db_path(app)?;
    log::info!("Opening single connection at: {:?}", db_path);

    let conn = rusqlite::Connection::open(&db_path)?;
    setup_pragmas(&conn)?;
    
    Ok(conn)
}

fn ensure_db_initialized(db_path: &std::path::Path) -> AppResult<()> {
    let mut conn = rusqlite::Connection::open(db_path)?;
    setup_pragmas(&conn)?;
    ensure_migrated(&mut conn)?;
    Ok(())
}

fn get_db_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let app_data = app.path().app_local_data_dir().map_err(|e| AppError::Internal(e.to_string()))?;
    let db_dir = app_data.join("data");
    std::fs::create_dir_all(&db_dir).map_err(AppError::IoError)?;
    Ok(db_dir.join("app.db"))
}

pub fn get_ai_cache_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let app_data = app.path().app_local_data_dir().map_err(|e| AppError::Internal(e.to_string()))?;
    let ai_dir = app_data.join("cache").join("ai");
    std::fs::create_dir_all(&ai_dir).map_err(AppError::IoError)?;
    Ok(ai_dir)
}

pub fn get_thumb_cache_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let app_data = app.path().app_local_data_dir().map_err(|e| AppError::Internal(e.to_string()))?;
    let thumb_dir = app_data.join("cache").join("thumbnails");
    std::fs::create_dir_all(&thumb_dir).map_err(AppError::IoError)?;
    Ok(thumb_dir)
}

pub fn setup_pragmas(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;
        PRAGMA busy_timeout=5000;
        PRAGMA cache_size=-64000;
        PRAGMA temp_store=MEMORY;
        PRAGMA mmap_size=30000000000;
    ")
}

fn ensure_migrated(conn: &mut rusqlite::Connection) -> AppResult<()> {
    migrations::ensure_migrated(conn)
}
