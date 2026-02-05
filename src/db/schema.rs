use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::Result;

/// Data revision number. Increment this to wipe all data on next startup.
/// When incrementing, also collapse all migrations into 001_initial.sql.
pub const DATA_REVISION: u32 = 1;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let migrations = Migrations::new(vec![M::up(include_str!(
        "../../migrations/001_initial.sql"
    ))]);

    migrations.to_latest(conn)?;
    Ok(())
}

/// Get the stored data revision from dromos_meta table.
/// Returns None if table doesn't exist or key not found.
pub fn get_stored_data_revision(conn: &Connection) -> Option<u32> {
    conn.query_row(
        "SELECT value FROM dromos_meta WHERE key = 'data_revision'",
        [],
        |row| {
            let value: String = row.get(0)?;
            Ok(value.parse::<u32>().ok())
        },
    )
    .ok()
    .flatten()
}

/// Store the data revision in dromos_meta table.
pub fn set_data_revision(conn: &Connection, revision: u32) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO dromos_meta (key, value) VALUES ('data_revision', ?1)",
        [revision.to_string()],
    )?;
    Ok(())
}

/// Check if the database has any user tables (nodes, edges).
/// Used to detect legacy databases without dromos_meta.
pub fn has_existing_data(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='nodes')",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|exists| exists == 1)
    .unwrap_or(false)
}
