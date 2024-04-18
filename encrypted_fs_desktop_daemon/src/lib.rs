use std::panic;
use tokio::task;
use tonic::transport::Server;
use tracing::{error, info};
use encrypted_fs_desktop_common::persistence::run_migrations;
use crate::vault_service::MyVaultService;
use crate::vault_service::vault_service_server::VaultServiceServer;

mod vault_service;

pub async fn run_in_daemon() {
    // // TODO: take level from configs
    // let _guard = encrypted_fs_desktop_common::log_init("DEBUG", "daemon");

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
            error!("Error: {:?}", err);
            panic!("Error: {:?}", err);
        }
    }
}

async fn daemon_run_async() -> Result<(), Box<dyn std::error::Error>> {
    let conn = encrypted_fs_desktop_common::persistence::establish_connection();
    let mut conn = conn.unwrap_or_else(|_| {
        error!("Error connecting to database");
        panic!("Error connecting to database")
    });
    run_migrations(&mut conn).unwrap_or_else(|_| {
        error!("Cannot run migrations");
        panic!("Cannot run migrations")
    });

    let addr = "[::1]:50051".parse()?;
    let service = MyVaultService::new();

    Server::builder()
        .add_service(VaultServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
