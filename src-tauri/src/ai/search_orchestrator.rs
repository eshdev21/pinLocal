use tauri::{AppHandle, Runtime, Manager};
use crate::commands::state::AppState;
use crate::error::AppResult;
use camino::Utf8PathBuf;
use crate::ai::indexing_service::SearchResult;
use crate::services::path_utils::WorkspacePath;
use std::path::Path;

pub async fn search<R: Runtime>(
    app: AppHandle<R>,
    query: String,
    board_id: Option<i32>,
) -> AppResult<Vec<serde_json::Value>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        crate::ai::model_manager::ensure_model_loaded(&app)?;

        let ai_cache_dir = crate::db::get_ai_cache_dir(&app)?;

        // 1. Find relevant AI DBs
        let ai_dbs = {
            let conn = state.get_conn()?;
            let roots = state.get_roots()?;
            let mut sql = "SELECT id, path FROM boards WHERE is_missing = 0 AND (?1 IS NULL OR id = ?1)".to_string();
            if !roots.is_empty() {
                let conditions: Vec<_> = roots.iter().map(|_| "path LIKE ?").collect();
                sql.push_str(&format!(" AND ({})", conditions.join(" OR ")));
            }
            let mut stmt = conn.prepare(&sql)?;
            let mut params: Vec<rusqlite::types::Value> = vec![board_id.into()];
            for r in &roots { params.push(format!("{}%", r).into()); }
            let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?)))?;
                
            let mut paths = Vec::new();
            for row in rows.flatten() {
                let (_, board_path_str) = row;
                let folder_id = WorkspacePath::folder_id(Path::new(&board_path_str));
                let db_path = ai_cache_dir.join(format!("{}.db", folder_id));
                if db_path.exists() { 
                    if let Ok(utf8_path) = Utf8PathBuf::from_path_buf(db_path) {
                        paths.push(utf8_path); 
                    }
                }
            }
            paths
        };

        if ai_dbs.is_empty() { return Ok(vec![]); }

        // 2. Get Query Vector from Python (Only once!)
        let query_vec = state.get_query_vector(&query)?;

        // 3. Search All Boards in Parallel (Native Rust)
        let top_results = crate::ai::vector_search::search_all_boards(
            ai_dbs, 
            &query_vec, 
            200, 
            &state.embedding_cache
        );

        if top_results.is_empty() { return Ok(vec![]); }

        // 4. Batch resolve to image metadata
        let conn = state.get_conn()?;
        let local_data_dir = app.path().app_local_data_dir().ok().map(|p| p.to_string_lossy().to_string());
        resolve_ai_results_to_images(&conn, top_results, board_id, &local_data_dir)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

pub async fn rescore<R: Runtime>(
    app: AppHandle<R>,
    query: String,
    previous_results: Vec<serde_json::Value>,
) -> AppResult<Vec<serde_json::Value>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        crate::ai::model_manager::ensure_model_loaded(&app)?;
        let ai_cache_dir = crate::db::get_ai_cache_dir(&app)?;

        // 1. Get Query Vector
        let query_vec = state.get_query_vector(&query)?;

        // 2. Group paths by board to find their DBs
        let mut board_to_paths: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        {
            let conn = state.get_conn()?;
            let paths: Vec<String> = previous_results
                .iter()
                .filter_map(|res| {
                    res.get("path")
                        .or_else(|| res.get("image").and_then(|i| i.get("path")))
                        .and_then(|p| p.as_str())
                })
                .map(|s| s.to_string())
                .collect();

            if !paths.is_empty() {
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
                    board_to_paths.entry(board_path).or_default().push(img_path);
                }
            }
        }

        // 3. Rescore in native Rust
        let mut all_rescored = Vec::new();
        for (board_path_str, paths) in board_to_paths {
            let folder_id = WorkspacePath::folder_id(Path::new(&board_path_str));
            let db_path = ai_cache_dir.join(format!("{}.db", folder_id));
            if !db_path.exists() { continue; }

            if let Ok(utf8_path) = Utf8PathBuf::from_path_buf(db_path) {
                let rescored = crate::ai::vector_search::rescore(&utf8_path, &query_vec, &paths, &state.embedding_cache);
                all_rescored.extend(rescored);
            }
        }

        all_rescored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // 4. Batch resolve
        let conn = state.get_conn()?;
        let local_data_dir = app.path().app_local_data_dir().ok().map(|p| p.to_string_lossy().to_string());
        resolve_ai_results_to_images(&conn, all_rescored, None, &local_data_dir)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}

pub fn resolve_ai_results_to_images(
    conn: &rusqlite::Connection,
    results: Vec<SearchResult>,
    board_id: Option<i32>,
    local_data_dir: &Option<String>,
) -> AppResult<Vec<serde_json::Value>> {
    if results.is_empty() { return Ok(vec![]); }

    // Batch resolve paths using IN clause for speed
    let placeholders: Vec<String> = (1..=results.len()).map(|i| format!("?{}", i)).collect();
    let sql = format!(
        r#"SELECT i.id, i.filename, i.path, i.board_id, b.name as board_name, 
                  i.thumb_path, i.thumbnail_status, i.width, i.height, 
                  i.size_bytes, i.mtime, i.created_at, i.is_missing
           FROM images i JOIN boards b ON i.board_id = b.id
           WHERE i.path IN ({}) AND (?{} IS NULL OR i.board_id = ?{})"#,
        placeholders.join(","),
        results.len() + 1,
        results.len() + 1
    );

    let mut stmt = conn.prepare(&sql)?;
    
    let mut params: Vec<rusqlite::types::Value> = results
        .iter()
        .map(|r| WorkspacePath::normalize(Path::new(&r.path)).into())
        .collect();
    params.push(board_id.into());

    let rows = stmt.query_map(rusqlite::params_from_iter(params), crate::db::images::map_image_row)?;

    // Map back to images and maintain search result order/scores
    let mut image_map = std::collections::HashMap::new();
    for img in rows.flatten() {
        image_map.insert(img.path.clone(), img);
    }

    let mut final_results = Vec::new();
    for result in results {
        let normalized = WorkspacePath::normalize(Path::new(&result.path));
        if let Some(img) = image_map.get(&normalized) {
            final_results.push(serde_json::json!({
                "image": img.clone().clean(local_data_dir),
                "score": result.score
            }));
        }
    }

    Ok(final_results)
}
