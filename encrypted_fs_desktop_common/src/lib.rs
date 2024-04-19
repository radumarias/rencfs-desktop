use std::{fs, panic};
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::str::FromStr;

use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use directories::ProjectDirs;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use tracing::{error, Level};
use tracing_appender::non_blocking::WorkerGuard;

use crate::app_details::{APPLICATION, ORGANIZATION, QUALIFIER};

pub mod schema;
pub mod models;
pub mod dao;
pub mod app_details;
pub mod persistence;
pub mod vault_service_error;
pub mod vault_handler;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub(crate) static DEVMODE: Lazy<bool> = Lazy::new(|| dotenv().is_ok());

pub fn log_init(level: &str, prefix: &str) -> WorkerGuard {
    if *DEVMODE {
        // for dev mode print to stdout
        let (writer, guard) = tracing_appender::non_blocking(std::io::stdout());
        tracing_subscriber::fmt()
            .with_writer(writer)
            .with_max_level(Level::from_str(level).unwrap())
            .init();
        guard
    } else {
        // for prod mode print to file
        let file_appender = tracing_appender::rolling::daily(get_logs_dir().to_str().unwrap(), format!("{}.log", prefix));
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_writer(file_writer)
            .with_max_level(Level::from_str(level).unwrap())
            .init();
        guard
    }
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

fn get_project_dirs() -> ProjectDirs {
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

pub fn get_config_dir() -> PathBuf {
    if *DEVMODE {
        get_dev_data_dir()
    } else {
        get_project_dirs().config_local_dir().to_path_buf()
    }
}

pub fn get_data_dir() -> PathBuf {
    if *DEVMODE {
        get_dev_data_dir()
    } else {
        get_project_dirs().data_local_dir().to_path_buf()
    }
}

pub fn get_logs_dir() -> PathBuf {
    if *DEVMODE {
        get_dev_data_dir()
    } else {
        get_project_dirs().data_local_dir().to_path_buf()
    }
}

fn get_dev_data_dir() -> PathBuf {
    let path = PathBuf::from_str(&format!("/tmp/{}", APPLICATION.replace(" ", "-").to_lowercase())).unwrap();
    fs::create_dir_all(&path).expect("Cannot create config directory");
    path
}