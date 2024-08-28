#[cfg(target_os = "linux")]
extern crate daemonize;
extern crate directories;

use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::panic::catch_unwind;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;

#[cfg(target_os = "linux")]
use daemonize::Daemonize;
use dotenvy::dotenv;
use rencfs_desktop_common::is_debug;
use tokio::sync::Mutex;
use tokio::task;
use tonic::transport::Server;
use tracing::{error, info, instrument, Level};

use rencfs_desktop_common::directories::{get_data_dir, get_logs_dir};
use rencfs_desktop_common::persistence::run_migrations;

use crate::vault_service::vault_service_server::VaultServiceServer;
use crate::vault_service::MyVaultService;

mod vault_service;

#[tokio::main]
async fn main() {
    let path = dotenv();
    match path {
        Ok(path) => println!("Loaded env file from {:?}", path),
        Err(err) => eprintln!("Error loading env file: {:?}", err),
    }

    if is_debug() {
        // TODO: take level from configs
        let log_level = Level::from_str("DEBUG").unwrap();
        let _log_guard = rencfs_desktop_common::log_init(log_level, "daemon");

        // in dev mode, we don't want to daemonize, so we can see logs in the console and have debug
        run_in_daemon().await;
    } else {
        // todo: daemonize after we can do this on windows also
        // #[cfg(target_os = "linux")]
        // daemonize();
        run_in_daemon().await;
    }
}

#[cfg(target_os = "linux")]
#[instrument]
fn daemonize() {
    let logs_dir = get_logs_dir();
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let username = whoami::username();

    OpenOptions::new()
        .append(true)
        .create(true)
        .open(logs_dir.join("daemon.out"))
        .unwrap();
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(logs_dir.join("daemon.err"))
        .unwrap();

    let stdout = OpenOptions::new()
        .write(true)
        .append(true)
        .open(logs_dir.join("daemon.out"))
        .unwrap();
    let stderr = OpenOptions::new()
        .write(true)
        .append(true)
        .open(logs_dir.join("daemon.err"))
        .unwrap();

    let daemonize = Daemonize::new()
        // .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        // .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory(get_data_dir()) // for default behaviour.
        .user(username.as_str())
        // .group("gnome") // Group name
        .group(gid) // or group id.
        // .umask(0o600)    // Set umask, `0o027` by default.
        .stdout(stdout)
        .stderr(stderr)
        .privileged_action(move || {
            println!("Privileged action, my uid is: {}, my gid is: {}", uid, gid);

            // TODO: take level from configs
            let log_level = Level::from_str("DEBUG").unwrap();
            let _log_guard = rencfs_desktop_common::log_init(log_level, "daemon");

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
    })
    .await;
    match res {
        Ok(Ok(_)) => {}
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
    let mut conn =
        rencfs_desktop_common::persistence::establish_connection().unwrap_or_else(|_| {
            error!("Error connecting to database");
            panic!("Error connecting to database")
        });

    run_migrations(&mut conn).unwrap_or_else(|_| {
        error!("Cannot run migrations");
        panic!("Cannot run migrations")
    });
    let db_conn = Arc::new(Mutex::new(conn));

    info!("Starting server");
    let addr = "[::1]:50051".parse()?;
    let service = MyVaultService::new(db_conn);
    let service = VaultServiceServer::new(service);

    info!("Listening on {}", addr);
    Server::builder()
        // GrpcWeb is over http1 so we must enable it.
        .add_service(service)
        .serve(addr)
        .await?;

    Ok(())
}
