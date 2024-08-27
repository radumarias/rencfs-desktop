use std::panic;
use std::panic::UnwindSafe;

use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use tracing::{error, instrument, Level};
use tracing_appender::non_blocking::WorkerGuard;

pub mod app_details;
pub mod dao;
pub mod directories;
pub mod models;
pub mod persistence;
pub mod schema;
pub mod vault_handler;
pub mod vault_service_error;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn log_init(level: Level, prefix: &str) -> WorkerGuard {
    if is_debug() {
        // for dev mode print to stdout
        let (writer, guard) = tracing_appender::non_blocking(std::io::stdout());
        tracing_subscriber::fmt()
            .pretty()
            .with_writer(writer)
            .with_max_level(level)
            .init();
        guard
    } else {
        // for prod mode print to file
        let file_appender = tracing_appender::rolling::daily(
            directories::get_logs_dir().to_str().unwrap(),
            format!("{}.log", prefix),
        );
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_writer(file_writer)
            .with_max_level(level)
            .init();
        guard
    }
}

#[instrument(skip(f))]
pub async fn execute_catch_unwind<F: FnOnce() -> R + UnwindSafe, R>(f: F) {
    let res = panic::catch_unwind(f);
    match res {
        Ok(_) => {}
        Err(err) => {
            error!(?err);
            panic!("{err:#?}");
        }
    }
}

#[allow(unreachable_code)]
pub fn is_debug() -> bool {
    #[cfg(debug_assertions)]
    {
        return true;
    }
    return false;
}
