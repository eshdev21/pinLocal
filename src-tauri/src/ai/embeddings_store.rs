use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use camino::Utf8PathBuf;
use tauri::{AppHandle, Manager, Runtime};

use crate::error::AppResult;
use crate::services::path_utils::WorkspacePath;
use crate::commands::state::AppState;

pub const EMBEDDINGS_DB_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS embeddings (
    filepath TEXT PRIMARY KEY,
    modified_at REAL NOT NULL,
    embedding BLOB,
    model_version TEXT,
    file_type TEXT DEFAULT 'image'
);
"#;

pub fn ai_db_path<R: Runtime>(
    app: &AppHandle<R>,
    board_path_str: &str,
) -> AppResult<std::path::PathBuf> {
    let ai_cache_dir = crate::db::get_ai_cache_dir(app)?;
    let folder_id = WorkspacePath::folder_id(Path::new(board_path_str));
    let ai_db_path = ai_cache_dir.join(format!("{}.db", folder_id));
    if let Some(parent) = ai_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(ai_db_path)
}

pub fn open_ai_db(db_path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(EMBEDDINGS_DB_SCHEMA)?;
    Ok(conn)
}

pub fn reconcile_inventory_db(
    conn: &mut Connection,
    added: Vec<(String, i64)>,
    removed: Vec<String>,
    full: bool,
) -> AppResult<(u32, u32)> {
    let tx = conn.transaction()?;

    let current_inventory: HashMap<String, i64> = added.into_iter().collect();
    let current_paths: HashSet<&str> = current_inventory.keys().map(|k| k.as_str()).collect();

    let mut existing_stmt = tx.prepare("SELECT filepath, modified_at FROM embeddings")?;
    let existing_rows = existing_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut existing = HashMap::new();
    for row in existing_rows {
        let (path, modified_at) = row?;
        existing.insert(path, modified_at as i64);
    }
    drop(existing_stmt);

    let mut removed_count = 0u32;
    let removed_paths: HashSet<String> = if full {
        existing
            .keys()
            .filter(|path| !current_paths.contains(path.as_str()))
            .cloned()
            .collect()
    } else {
        removed.into_iter().collect()
    };

    for path in removed_paths {
        removed_count +=
            tx.execute("DELETE FROM embeddings WHERE filepath = ?1", params![path])? as u32;
    }

    let mut added_count = 0u32;
    for (path, modified_at) in current_inventory {
        match existing.get(&path).copied() {
            None => {
                tx.execute(
                    "INSERT INTO embeddings (filepath, modified_at, embedding) VALUES (?1, ?2, NULL)",
                    params![path, modified_at as f64],
                )?;
                added_count += 1;
            }
            Some(existing_modified_at) if existing_modified_at < modified_at => {
                tx.execute(
                    "UPDATE embeddings SET modified_at = ?1, embedding = NULL WHERE filepath = ?2",
                    params![modified_at as f64, path],
                )?;
            }
            _ => {}
        }
    }

    tx.commit()?;
    Ok((added_count, removed_count))
}

pub fn sync_inventory<R: Runtime>(
    app: &AppHandle<R>,
    board_path_str: &str,
    added: Vec<(String, i64)>,
    removed: Vec<String>,
    full: bool,
) -> AppResult<()> {
    if added.is_empty() && removed.is_empty() && !full {
        return Ok(());
    }

    let state = app.state::<AppState>();
    let ai_db_path = ai_db_path(app, board_path_str)?;
    let mut conn = open_ai_db(&ai_db_path)?;
    let (added_count, removed_count) = reconcile_inventory_db(&mut conn, added, removed, full)?;

    log::info!(
        ">>> [AI] Inventory synced for board {}: added {}, removed {}",
        board_path_str,
        added_count,
        removed_count
    );

    state
        .embedding_cache
        .invalidate(&Utf8PathBuf::from_path_buf(ai_db_path).unwrap());
    Ok(())
}

pub fn delete_embeddings<R: Runtime>(app: &AppHandle<R>, paths: Vec<String>) -> AppResult<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let state = app.state::<AppState>();

    let mut board_groups: HashMap<String, Vec<String>> = HashMap::new();
    {
        let conn = state.get_conn()?;
        let placeholders: Vec<String> = (1..=paths.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT i.path, b.path FROM images i JOIN boards b ON i.board_id = b.id WHERE i.path IN ({})",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<rusqlite::types::Value> = paths.into_iter().map(|p| p.into()).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows.flatten() {
            let (img_path, board_path) = row;
            board_groups.entry(board_path).or_default().push(img_path);
        }
    }

    for (board_path_str, folder_paths) in board_groups {
        let db_path = ai_db_path(app, &board_path_str)?;
        if !db_path.exists() {
            continue;
        }

        let mut conn = open_ai_db(&db_path)?;
        let tx = conn.transaction()?;
        for path in folder_paths {
            tx.execute("DELETE FROM embeddings WHERE filepath = ?1", params![path])?;
        }
        tx.commit()?;

        // Invalidate cache for this board
        state
            .embedding_cache
            .invalidate(&Utf8PathBuf::from_path_buf(db_path).unwrap());
    }

    Ok(())
}

pub fn cleanup_embeddings<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let state = app.state::<AppState>();

    let boards: Vec<(String, String)> = {
        let conn = state.get_conn()?;
        let board_ids = state.get_board_ids()?;
        if board_ids.is_empty() {
            return Ok(());
        }

        let placeholders: Vec<String> = board_ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT name, path FROM boards WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<i32> = board_ids;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.flatten().collect()
    };

    for (board_name, board_path_str) in boards {
        let db_path = ai_db_path(app, &board_path_str)?;
        if !db_path.exists() {
            continue;
        }

        log::info!(">>> [AI] Cleaning up Board: {}", board_name);
        let mut conn = open_ai_db(&db_path)?;
        let root = Path::new(&board_path_str);

        let existing_paths: Vec<String> = {
            let mut stmt = conn.prepare("SELECT filepath FROM embeddings")?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .filter_map(Result::ok)
                .collect();
            rows
        };

        let mut paths_to_delete = Vec::new();
        for path in existing_paths {
            let full_path = {
                let candidate = Path::new(&path);
                if candidate.is_absolute() {
                    candidate.to_path_buf()
                } else {
                    root.join(candidate)
                }
            };
            if !full_path.exists() {
                paths_to_delete.push(path);
            }
        }

        if !paths_to_delete.is_empty() {
            let tx = conn.transaction()?;
            for path in paths_to_delete {
                tx.execute("DELETE FROM embeddings WHERE filepath = ?1", params![path])?;
            }
            tx.commit()?;
        }

        // Invalidate cache for this board
        state
            .embedding_cache
            .invalidate(&Utf8PathBuf::from_path_buf(db_path).unwrap());
    }

    Ok(())
}

pub fn reset_all_stores<R: Runtime>(app: &AppHandle<R>, state: &AppState) -> AppResult<()> {
    let roots = state.get_roots()?;
    let ai_cache = crate::db::get_ai_cache_dir(app)?;

    for folder in roots {
        let folder_id = WorkspacePath::folder_id(Path::new(&folder));
        let db_path = ai_cache.join(format!("{}.db", folder_id));
        if db_path.exists() {
            std::fs::remove_file(db_path)?;
        }
    }
    state.embedding_cache.clear();
    Ok(())
}

