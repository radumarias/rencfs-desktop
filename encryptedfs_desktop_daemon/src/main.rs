extern crate daemonize;
extern crate directories;

use std::backtrace::Backtrace;
use std::thread;
use std::fs::OpenOptions;
use std::panic::catch_unwind;
use std::sync::Arc;

use daemonize::Daemonize;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use tokio::task;
use tonic::transport::Server;
use tracing::{error, info, instrument};

use encryptedfs_desktop_common::persistence::run_migrations;
use encryptedfs_desktop_common::storage::{get_data_dir, get_logs_dir};

use crate::vault_service::MyVaultService;
use crate::vault_service::vault_service_server::VaultServiceServer;

mod vault_service;

pub(crate) static DEVMODE: Lazy<bool> = Lazy::new(|| dotenv().is_ok());

#[tokio::main]
async fn main() {
    if *DEVMODE {
        // TODO: take level from configs
        let _guard = encryptedfs_desktop_common::log_init("DEBUG", "daemon");

        // in dev mode we don't want to daemonize so we can see logs in console and have debug
        run_in_daemon().await;
    } else {
        daemonize();
    }
}

#[instrument]
fn daemonize() {
    let logs_dir = get_logs_dir();
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let username = whoami::username();

    let stdout = OpenOptions::new().write(true).append(true).open(logs_dir.join("daemon.out")).unwrap();
    let stderr = OpenOptions::new().write(true).append(true).open(logs_dir.join("daemon.err")).unwrap();

    let daemonize = Daemonize::new()
        // .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        // .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory(get_data_dir()) // for default behaviour.
        .user(username.as_str())
        // .group("gnome") // Group name
        .group(gid)        // or group id.
        // .umask(0o600)    // Set umask, `0o027` by default.
        .stdout(stdout)
        .stderr(stderr)
        .privileged_action(move || {
            println!("Privileged action, my uid is: {}, my gid is: {}", uid, gid);

            // TODO: take level from configs
            let _guard = encryptedfs_desktop_common::log_init("DEBUG", "daemon");

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
            error!(err = %e)
        }
    }
}

#[instrument]
pub async fn run_in_daemon() {
    info!("Starting daemon");

    let res = task::spawn_blocking(|| {
        catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                daemon_run_async().await.expect("Error running daemon");
            });
        })
    }).await;
    match res {
        Ok(Ok(_)) => println!("Program terminated successfully"),
        Ok(Err(err)) => {
            error!("panic {err:#?}");
            error!(backtrace = %Backtrace::force_capture());
            panic!("{err:#?}");
        }
        Err(err) => {
            error!(err = %err, "panic");
            error!(backtrace = %Backtrace::force_capture());
            panic!("{err}");
        }
    }
}

#[instrument]
async fn daemon_run_async() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = encryptedfs_desktop_common::persistence::establish_connection().unwrap_or_else(|_| {
        error!("Error connecting to database");
        panic!("Error connecting to database")
    });

    run_migrations(&mut conn).unwrap_or_else(|_| {
        error!("Cannot run migrations");
        panic!("Cannot run migrations")
    });
    let db_conn = Arc::new(Mutex::new(conn));

    let addr = "[::1]:50051".parse()?;
    let service = MyVaultService::new(db_conn);

    Server::builder()
        .add_service(VaultServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
