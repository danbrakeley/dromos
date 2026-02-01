use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::Result;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let migrations = Migrations::new(vec![
        M::up(include_str!("../../migrations/001_initial.sql")),
        M::up(include_str!("../../migrations/002_add_title.sql")),
    ]);

    migrations.to_latest(conn)?;
    Ok(())
}
