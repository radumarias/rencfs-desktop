extern crate daemonize;
extern crate directories;

use static_init::dynamic;
use std::{fs, panic, thread};
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::{Arc};
use daemonize::Daemonize;
use diesel::SqliteConnection;
use directories::ProjectDirs;
use dotenvy::dotenv;
use libc::gid_t;
use tokio::sync::Mutex;
use tokio::task;
use tonic::transport::Server;

use encrypted_fs_desktop_common::persistence::run_migrations;
use tracing::{error, info};
use encrypted_fs_desktop_common::app_details::{APPLICATION, ORGANIZATION, QUALIFIER};
use encrypted_fs_desktop_common::{execute_catch_unwind, get_project_dirs};
use crate::vault_service::MyVaultService;
use crate::vault_service::vault_service_server::VaultServiceServer;

mod vault_service;

#[tokio::main]
async fn main() {
    if dotenv().is_ok() {
        // TODO: take level from configs
        let _guard = encrypted_fs_desktop_common::log_init("DEBUG", "daemon");

        // in dev mode we don't want to daemonize so we can see logs in console and have debug
        run_in_daemon().await;
    } else {
        daemonize();
    }
}

fn daemonize() {
    let data_dir = get_project_dirs().data_local_dir().to_path_buf();
    let logs_dir = data_dir.join("logs");
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let username = whoami::username();

    let stdout = OpenOptions::new().write(true).append(true).open(logs_dir.join("daemon.out")).unwrap();
    let stderr = OpenOptions::new().write(true).append(true).open(logs_dir.join("daemon.err")).unwrap();

    let daemonize = Daemonize::new()
        // .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        // .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory(data_dir.clone()) // for default behaviour.
        .user(username.as_str())
        // .group("gnome") // Group name
        .group(gid)        // or group id.
        // .umask(0o600)    // Set umask, `0o027` by default.
        .stdout(stdout)
        .stderr(stderr)
        .privileged_action(move || {
            println!("Privileged action, my uid is: {}, my gid is: {}", uid, gid);

            // TODO: take level from configs
            let _guard = encrypted_fs_desktop_common::log_init("DEBUG", "daemon");

            let handle = thread::spawn(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    run_in_daemon().await;
                });
            });
            handle.join().unwrap();

            "Executed before drop privileges"
        });

    match daemonize.start() {
        Ok(_) => {}
        Err(e) => {
            error!("Error, {}", e)
        }
    }
}

pub async fn run_in_daemon() {
    info!("Starting daemon");

    let res = task::spawn_blocking(|| {
        panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                daemon_run_async().await.expect("Error running daemon");
            });
        })
    }).await;
    match res {
        Ok(Ok(_)) => println!("Program terminated successfully"),
        Ok(Err(err)) => {
            error!("Error: {:?}", err);
            panic!("Error: {:?}", err);
        }
        Err(err) => {
            error!("Error: {}", err);
            panic!("Error: {}", err);
        }
    }
}

async fn daemon_run_async() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = encrypted_fs_desktop_common::persistence::establish_connection().unwrap_or_else(|_| {
        error!("Error connecting to database");
        panic!("Error connecting to database")
    });

    unsafe {
        run_migrations(&mut conn).unwrap_or_else(|_| {
            error!("Cannot run migrations");
            panic!("Cannot run migrations")
        });
    }
    let db_conn = Arc::new(Mutex::new(conn));

    let addr = "[::1]:50051".parse()?;
    let service = MyVaultService::new(db_conn.clone());

    Server::builder()
        .add_service(VaultServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
