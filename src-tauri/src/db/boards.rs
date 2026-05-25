use rusqlite::Connection;
use crate::commands::boards::Board;
use crate::error::AppResult;

pub fn get_boards(conn: &Connection, allowed_board_ids: Option<&[i32]>) -> AppResult<Vec<Board>> {
    let sql = if let Some(ids) = allowed_board_ids {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        format!("SELECT id, name, path, cover_image, image_count, created_at, updated_at, is_missing FROM boards WHERE id IN ({}) ORDER BY name ASC", placeholders.join(","))
    } else {
        "SELECT id, name, path, cover_image, image_count, created_at, updated_at, is_missing FROM boards ORDER BY name ASC".to_string()
    };

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<i32> = allowed_board_ids.map(|ids| ids.to_vec()).unwrap_or_default();
    
    let boards = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(Board {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            cover_image: row.get(3)?,
            image_count: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            is_missing: row.get::<_, i32>(7)? != 0,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(boards)
}

pub fn upsert_board(conn: &Connection, name: &str, path: &str) -> AppResult<i32> {
    let now = chrono::Utc::now().timestamp();
    let id: i32 = conn.query_row(
        "INSERT INTO boards (name, path, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?3) 
         ON CONFLICT(path) DO UPDATE SET name=excluded.name, updated_at=excluded.updated_at
         RETURNING id",
        rusqlite::params![name, path, now],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn get_board_path(conn: &Connection, board_id: i32) -> AppResult<String> {
    let path: String = conn.query_row(
        "SELECT path FROM boards WHERE id = ?1",
        [board_id],
        |row| row.get(0),
    )?;
    Ok(path)
}
