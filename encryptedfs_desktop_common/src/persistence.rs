use std::env;

use diesel::{Connection, ConnectionError, ConnectionResult, SqliteConnection};
use diesel::connection::SimpleConnection;
use diesel::migration::MigrationVersion;
use diesel_migrations::MigrationHarness;
use tracing::{info, instrument};

use crate::{DEVMODE, MIGRATIONS};
use crate::storage::get_config_dir;

pub fn establish_connection() -> ConnectionResult<SqliteConnection> {
    let database_url: String;
    if *DEVMODE {
        database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    } else {
        database_url = get_config_dir().join("encryptedfs_desktop.db").to_str().unwrap().to_string()
    }

    let mut conn = SqliteConnection::establish(&database_url)?;
    conn.batch_execute("
            PRAGMA journal_mode = WAL;          -- better write-concurrency
            PRAGMA synchronous = NORMAL;        -- fsync only in critical moments
            PRAGMA wal_autocheckpoint = 1000;   -- write WAL changes back every 1000 pages, for an in average 1MB WAL file. May affect readers if number is increased
            PRAGMA wal_checkpoint(TRUNCATE);    -- free some space by truncating possibly massive WAL files from the last run.
            PRAGMA busy_timeout = 250;          -- sleep if the database is busy
            PRAGMA foreign_keys = ON;           -- enforce foreign keys
        ").map_err(ConnectionError::CouldntSetupConfiguration)?;

    Ok(conn)
}

#[instrument(skip(conn))]
pub fn run_migrations(conn: &mut SqliteConnection) -> diesel::migration::Result<Vec<MigrationVersion<'_>>> {
    info!("Running migrations");
    conn.run_pending_migrations(MIGRATIONS)
}
