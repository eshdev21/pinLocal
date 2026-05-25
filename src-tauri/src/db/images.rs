use rusqlite::Connection;
use crate::commands::images::Image;
use crate::error::AppResult;

pub fn get_images(
    conn: &Connection, 
    board_id: Option<i32>, 
    allowed_board_ids: Option<&[i32]>,
    page: u32, 
    page_size: u32,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> AppResult<(Vec<Image>, u32)> {
    let offset = (page - 1) * page_size;
    let order_col = match sort_by.as_deref() {
        Some("name") => "i.filename",
        Some("size") => "i.size_bytes",
        Some("date") => "i.mtime",
        _ => "i.mtime",
    };
    let order_dir = match sort_order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let mut where_clauses = Vec::new();
    let mut params: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(bid) = board_id {
        where_clauses.push("i.board_id = ?".to_string());
        params.push(bid.into());
    } else if let Some(ids) = allowed_board_ids {
        if ids.is_empty() { return Ok((Vec::new(), 0)); }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        where_clauses.push(format!("i.board_id IN ({})", placeholders.join(",")));
        for id in ids {
            params.push((*id).into());
        }
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT i.id, i.filename, i.path, i.board_id, b.name as board_name, i.thumb_path, i.thumbnail_status, i.width, i.height, i.size_bytes, i.mtime, i.created_at, i.is_missing
         FROM images i JOIN boards b ON i.board_id = b.id 
         {} ORDER BY {} {} LIMIT ? OFFSET ?",
         where_sql,
         order_col,
         order_dir
    );

    let mut stmt = conn.prepare(&sql)?;
    
    // Total count query
    let count_sql = format!("SELECT COUNT(*) FROM images i {}", where_sql);
    let mut count_stmt = conn.prepare(&count_sql)?;
    let total: u32 = count_stmt.query_row(rusqlite::params_from_iter(params.clone()), |row| row.get(0))?;

    // Images query
    let mut images_params = params;
    images_params.push(page_size.into());
    images_params.push(offset.into());
    let images = stmt.query_map(rusqlite::params_from_iter(images_params), map_image_row)?.collect::<Result<Vec<_>, _>>()?;

    Ok((images, total))
}

pub fn map_image_row(row: &rusqlite::Row) -> rusqlite::Result<Image> {
    Ok(Image {
        id: row.get(0)?,
        filename: row.get(1)?,
        path: row.get(2)?,
        board_id: row.get(3)?,
        board_name: row.get(4)?,
        thumb_path: row.get(5)?,
        thumbnail_status: row.get(6)?,
        width: row.get(7)?,
        height: row.get(8)?,
        size_bytes: row.get(9)?,
        mtime: row.get(10)?,
        created_at: row.get(11)?,
        is_missing: row.get::<_, i32>(12)? != 0,
    })
}

pub fn get_image_paths(conn: &Connection, image_id: i32) -> AppResult<(String, Option<String>)> {
    let res = conn.query_row(
        "SELECT path, thumb_path FROM images WHERE id = ?1",
        [image_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    )?;
    Ok(res)
}

pub fn get_storage_stats(conn: &Connection, board_ids: &[i32]) -> AppResult<(i64, i64)> {
    let mut sql = "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM images WHERE is_missing = 0".to_string();
    if !board_ids.is_empty() {
        let placeholders: Vec<String> = board_ids.iter().map(|_| "?".to_string()).collect();
        sql.push_str(&format!(" AND board_id IN ({})", placeholders.join(",")));
    }
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<i32> = board_ids.to_vec();
    let stats = stmt.query_row(rusqlite::params_from_iter(params), |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    Ok(stats)
}
