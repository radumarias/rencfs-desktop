use std::{fs, panic};
use std::future::Future;
use std::panic::UnwindSafe;
use std::str::FromStr;

use diesel::{Connection, IntoSql};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use directories::ProjectDirs;
use tracing::{error, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, Layer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::app_details::{APPLICATION, ORGANIZATION, QUALIFIER};

pub mod schema;
pub mod models;
pub mod dao;
pub mod app_details;
pub mod persistence;
pub mod vault_service_error;
pub mod vault_handler;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn log_init(level: &str, prefix: &str) -> WorkerGuard {
    let mut guard: Option<WorkerGuard> = None;

    if dotenvy::dotenv().is_ok() {
        // for dev mode print to stdout
        let (writer, guard2) = tracing_appender::non_blocking(std::io::stdout());
        tracing_subscriber::fmt()
            .with_writer(writer)
            .with_max_level(Level::from_str(level).unwrap())
            .init();
        guard = Some(guard2);
    } else {
        // for prod mode print to file
        let logs_path = get_project_dirs().data_local_dir().join("logs");

        let file_appender = tracing_appender::rolling::daily(logs_path.to_str().unwrap(), format!("{}.log", prefix));
        let (file_writer, guard2) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_writer(file_writer)
            .with_max_level(Level::from_str(level).unwrap())
            .init();
        guard = Some(guard2);
    }

    guard.take().unwrap()
}

pub async fn execute_catch_unwind<F: FnOnce() -> R + UnwindSafe, R>(f: F) {
    let res = panic::catch_unwind(f);
    match res {
        Ok(_) => {}
        Err(err) => {
            error!("Error: {:?}", err);
            panic!("Error: {:?}", err);
        }
    }
}

pub fn get_project_dirs() -> ProjectDirs {
    let proj_dirs = if let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) {
        proj_dirs
    } else {
        error!("Cannot get project directories");
        panic!("Cannot get project directories");
    };
    fs::create_dir_all(proj_dirs.config_dir()).expect("Cannot create config directory");
    fs::create_dir_all(proj_dirs.data_local_dir()).expect("Cannot create data directory");
    fs::create_dir_all(proj_dirs.data_local_dir().join("logs")).expect("Cannot create logs directory");

    proj_dirs
}