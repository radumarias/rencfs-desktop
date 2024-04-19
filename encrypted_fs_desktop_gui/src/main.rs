use std::panic;
use std::panic::catch_unwind;
use std::sync::Mutex;
use diesel::Connection;
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;
use tokio::io::AsyncWriteExt;
use tokio::task;
use tracing::error;

use encrypted_fs_desktop_common::models::NewVault;
use encrypted_fs_desktop_common::persistence::run_migrations;
use static_init::dynamic;
use tokio::runtime::Runtime;
use crate::daemon_service::vault_service_client::VaultServiceClient;
use crate::dashboard::Dashboard;

mod daemon_service {
    tonic::include_proto!("encrypted_fs_desktop");
}

mod dashboard;
mod detail;
mod listview;

pub use listview::ListView;

#[dynamic]
pub static RT: Runtime = Runtime::new().expect("Cannot create tokio runtime");

#[dynamic]
pub static DB_CONN: Mutex<SqliteConnection> = {
    match encrypted_fs_desktop_common::persistence::establish_connection() {
        Ok(db) => { Mutex::new(db) }
        Err(err) => {
            error!("Error connecting to database: {:?}", err);
            panic!("Error connecting to database: {:?}", err);
        }
    }
};

fn main() {
    // TODO: take level from configs
    let _guard = encrypted_fs_desktop_common::log_init("DEBUG", "gui");

    let res = catch_unwind(|| {
        run_main().expect("Error running app");
    });
    match res {
        Ok(_) => println!("Program terminated successfully"),
        Err(err) => {
            error!("Error: {:?}", err);
            panic!("Error: {:?}", err);
        }
    }
}

fn run_main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = encrypted_fs_desktop_common::persistence::establish_connection();
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
        Box::new(|cc| {
            Box::new(Dashboard::new(conn))
        }),
    )
}
