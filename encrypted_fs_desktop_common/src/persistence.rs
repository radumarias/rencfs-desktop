use diesel::{Connection, ConnectionError, ConnectionResult, SqliteConnection};
use dotenvy::dotenv;
use std::{env, fs};
use diesel::connection::SimpleConnection;
use diesel::migration::MigrationVersion;
use diesel_migrations::MigrationHarness;
use directories::ProjectDirs;
use tracing::info;
use crate::app_details::{APPLICATION, ORGANIZATION, QUALIFIER};
use crate::{get_project_dirs, MIGRATIONS};

const DATABASE_FILE_NAME: &str = "encrypted_fs_desktop.db";

pub fn establish_connection() -> ConnectionResult<SqliteConnection> {
    let mut database_url: String;
    let env = dotenv();
    if env.is_ok() {
        info!("Loaded .env file");
        database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    } else {
        database_url = get_project_dirs().config_dir().join(DATABASE_FILE_NAME).to_str().unwrap().to_string()
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

pub fn run_migrations(conn: &mut SqliteConnection) -> diesel::migration::Result<Vec<MigrationVersion<'_>>> {
    info!("Running migrations");
    conn.run_pending_migrations(MIGRATIONS)
}
