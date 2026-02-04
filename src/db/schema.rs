use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::Result;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let migrations = Migrations::new(vec![
        M::up(include_str!("../../migrations/001_initial.sql")),
        M::up(include_str!("../../migrations/002_add_title.sql")),
        M::up(include_str!("../../migrations/003_nes_header_fields.sql")),
        M::up(include_str!("../../migrations/004_metadata_fields.sql")),
        M::up(include_str!("../../migrations/005_source_file_header.sql")),
    ]);

    migrations.to_latest(conn)?;
    Ok(())
}
