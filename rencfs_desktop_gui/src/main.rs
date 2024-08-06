use std::backtrace::Backtrace;
use std::panic;
use std::panic::catch_unwind;
use std::str::FromStr;
use std::sync::Mutex;
use diesel::prelude::*;
use dotenvy::dotenv;
use tracing::{error, instrument, Level};

use rencfs_desktop_common::persistence::run_migrations;
use static_init::dynamic;
use tokio::runtime::Runtime;
use crate::dashboard::Dashboard;

mod daemon_service {
    tonic::include_proto!("rencfs_desktop");
}

mod dashboard;
mod detail;
mod listview;

pub mod util;

pub use listview::ListView;

#[dynamic]
pub(crate) static RT: Runtime = Runtime::new().expect("Cannot create tokio runtime");

#[dynamic]
pub static DB_CONN: Mutex<SqliteConnection> = {
    let path = dotenv();
    match path {
        Ok(path) => println!("Loaded env file from {:?}", path),
        Err(err) => eprintln!("Error loading env file: {:?}", err),
    }
    match rencfs_desktop_common::persistence::establish_connection() {
        Ok(db) => { Mutex::new(db) }
        Err(err) => {
            error!(err = %err, "Error connecting to database");
            panic!("Error connecting to database: {:?}", err);
        }
    }
};

#[instrument]
fn main() {
    let path = dotenv();
    match path {
        Ok(path) => println!("Loaded env file from {:?}", path),
        Err(err) => eprintln!("Error loading env file: {:?}", err),
    }

    // TODO: take level from configs
    let log_level = Level::from_str("DEBUG").unwrap();
    let _log_guard = rencfs_desktop_common::log_init(log_level, "gui");

    let res = catch_unwind(|| {
        run_main().expect("Error running app");
    });
    match res {
        Ok(_) => {}
        Err(err) => {
            error!("panic {err:#?}");
            error!(backtrace = %Backtrace::force_capture());
            panic!("{err:#?}");
        }
    }
}

#[instrument]
fn run_main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = rencfs_desktop_common::persistence::establish_connection();
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
            Ok(Box::new(Dashboard::new(conn)))
        }),
    )
}
