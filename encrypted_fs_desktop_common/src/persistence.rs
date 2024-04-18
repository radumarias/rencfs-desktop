use diesel::{Connection, ConnectionResult, SqliteConnection};
use dotenvy::dotenv;
use std::{env, fs};
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

    SqliteConnection::establish(&database_url)
}

pub fn run_migrations(conn: &mut SqliteConnection) -> diesel::migration::Result<Vec<MigrationVersion<'_>>> {
    info!("Running migrations");
    conn.run_pending_migrations(MIGRATIONS)
}
