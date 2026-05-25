use rusqlite::{params, Connection};
use std::collections::HashMap;
use crate::error::AppResult;
use crate::config::Workspace;

pub fn upsert_workspace(conn: &mut Connection, id: &str, name: &str, board_ids: &[i32]) -> AppResult<()> {
    let now = chrono::Utc::now().timestamp();

    // Start database transaction
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO workspaces (id, name, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?3) 
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, updated_at=excluded.updated_at",
        params![id, name, now],
    )?;

    tx.execute(
        "DELETE FROM workspace_folders WHERE workspace_id = ?1",
        [id],
    )?;

    {
        let mut stmt = tx.prepare("INSERT INTO workspace_folders (workspace_id, board_id) VALUES (?1, ?2)")?;
        for &board_id in board_ids {
            stmt.execute(params![id, board_id])?;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn get_workspaces(conn: &Connection) -> AppResult<Vec<Workspace>> {
    let mut stmt = conn.prepare(
        "
        SELECT w.id, w.name, f.board_id, b.path
        FROM workspaces w
        LEFT JOIN workspace_folders f ON w.id = f.workspace_id
        LEFT JOIN boards b ON f.board_id = b.id
        ORDER BY w.created_at ASC
    ",
    )?;

    let mut ws_list: Vec<Workspace> = Vec::new();
    let mut ws_index: HashMap<String, usize> = HashMap::new();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    for row in rows {
        let (id, name, board_id, path) = row?;
        if let Some(&idx) = ws_index.get(&id) {
            if let Some(bid) = board_id {
                ws_list[idx].board_ids.push(bid);
            }
            if let Some(p) = path {
                ws_list[idx].folder_paths.push(p);
            }
        } else {
            ws_index.insert(id.clone(), ws_list.len());
            let mut ws = Workspace {
                id,
                name,
                board_ids: Vec::new(),
                folder_paths: Vec::new(),
            };
            if let Some(bid) = board_id {
                ws.board_ids.push(bid);
            }
            if let Some(p) = path {
                ws.folder_paths.push(p);
            }
            ws_list.push(ws);
        }
    }

    Ok(ws_list)
}

pub fn remove_workspace(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
    Ok(())
}

pub fn set_board_ai_sync(conn: &Connection, board_id: i32, enabled: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE boards SET needs_ai_sync = ?1 WHERE id = ?2",
        params![enabled as i32, board_id],
    )?;
    Ok(())
}

pub fn get_pending_ai_boards(conn: &Connection) -> AppResult<Vec<(i32, String)>> {
    let mut stmt = conn.prepare("SELECT id, path FROM boards WHERE needs_ai_sync = 1 AND is_missing = 0")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let mut pending = Vec::new();
    for row in rows {
        pending.push(row?);
    }
    Ok(pending)
}
