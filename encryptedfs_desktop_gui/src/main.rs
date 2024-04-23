use std::backtrace::Backtrace;
use std::panic;
use std::panic::catch_unwind;
use std::sync::Mutex;
use diesel::prelude::*;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use tracing::{error, instrument};

use encryptedfs_desktop_common::persistence::run_migrations;
use static_init::dynamic;
use tokio::runtime::Runtime;
use crate::dashboard::Dashboard;

mod daemon_service {
    tonic::include_proto!("encryptedfs_desktop");
}

mod dashboard;
mod detail;
mod listview;

pub mod util;

pub use listview::ListView;

#[dynamic]
pub(crate) static RT: Runtime = Runtime::new().expect("Cannot create tokio runtime");

pub(crate) static DEVMODE: Lazy<bool> = Lazy::new(|| dotenv().is_ok());

#[dynamic]
pub static DB_CONN: Mutex<SqliteConnection> = {
    match encryptedfs_desktop_common::persistence::establish_connection() {
        Ok(db) => { Mutex::new(db) }
        Err(err) => {
            error!(err = %err, "Error connecting to database");
            panic!("Error connecting to database: {:?}", err);
        }
    }
};

#[instrument]
fn main() {
    // TODO: take level from configs
    let _guard = encryptedfs_desktop_common::log_init("DEBUG", "gui");

    let res = catch_unwind(|| {
        run_main().expect("Error running app");
    });
    match res {
        Ok(_) => println!("Program terminated successfully"),
        Err(err) => {
            error!("panic {err:#?}");
            error!(backtrace = %Backtrace::force_capture());
            panic!("{err:#?}");
        }
    }
}

#[instrument]
fn run_main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = encryptedfs_desktop_common::persistence::establish_connection();
    let mut conn = conn.unwrap_or_else(|_| {
        error!("Error connecting to database");
        panic!("Error connecting to database")
    });
    run_migrations(&mut conn).unwrap_or_else(|_| {
        error!("Cannot run migrations");
        panic!("Cannot run migrations")
    });

    start_ui(conn).expect("Error starting UI");

    Ok(())
}

pub fn start_ui(conn: SqliteConnection) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0]) // wide enough for the drag-drop overlay text
            .with_drag_and_drop(true),
        centered: true,
        // decorated: false,
        follow_system_theme: true,
        ..Default::default()
    };
    eframe::run_native(
        "EncryptedFS",
        options,
        Box::new(|_cc| {
            Box::new(Dashboard::new(conn))
        }),
    )
}
