use crate::error::AppResult;
use crate::db::schema;

pub fn ensure_migrated(conn: &mut rusqlite::Connection) -> AppResult<()> {
    let current_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    log::info!("Current database schema version: {}", current_version);

    if current_version < 1 {
        log::info!("Migrating database to version 1...");
        let tx = conn.transaction()?;

        tx.execute(schema::CREATE_BOARDS_TABLE, [])?;
        tx.execute(schema::CREATE_IMAGES_TABLE, [])?;
        tx.execute(schema::CREATE_WORKSPACES_TABLE, [])?;
        tx.execute(schema::CREATE_WORKSPACE_FOLDERS_TABLE, [])?;
        tx.execute(schema::CREATE_APP_STATE_TABLE, [])?;
        tx.execute(schema::CREATE_BACKGROUND_TASKS_TABLE, [])?;
        tx.execute_batch(schema::CREATE_INDEXES)?;

        tx.execute_batch("PRAGMA user_version = 1")?;
        tx.commit()?;
        log::info!("Database migration to version 1 complete.");
    }

    Ok(())
}
