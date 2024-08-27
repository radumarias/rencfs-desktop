use std::collections::HashMap;
use std::sync::Arc;

use diesel::SqliteConnection;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::{info, instrument};

use rencfs_desktop_common::vault_handler::{VaultHandler, VaultHandlerError};
use rencfs_desktop_common::vault_service_error::VaultServiceError;

use crate::vault_service::vault_service_server::VaultService;

tonic::include_proto!("rencfs_desktop");

pub struct MyVaultService {
    handlers: Arc<Mutex<HashMap<u32, VaultHandler>>>,
    db_conn: Arc<Mutex<SqliteConnection>>,
}

impl MyVaultService {
    pub fn new(db_conn: Arc<Mutex<SqliteConnection>>) -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            db_conn,
        }
    }

    async fn handle_handler_empty_response(
        response: Result<(), VaultHandlerError>,
    ) -> Result<Response<EmptyReply>, Status> {
        match response {
            Ok(_) => Ok(Response::new(EmptyReply {})),
            Err(err) => Err(VaultServiceError::from(err).into()),
        }
    }
}

#[tonic::async_trait]
impl VaultService for MyVaultService {
    async fn hello(&self, request: Request<HelloRequest>) -> Result<Response<HelloReply>, Status> {
        Ok(Response::new(HelloReply {
            message: format!("Hello, {}!", request.into_inner().name),
        }))
    }

    #[instrument(skip(self), err)]
    async fn lock(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        let id = request.into_inner().id;
        info!(id, "Vault lock request received");

        let mut handlers = self.handlers.lock().await;
        let handler = handlers
            .entry(id)
            .or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(handler.lock(None).await).await;
    }

    #[instrument(skip(self), err)]
    async fn unlock(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        let id = request.into_inner().id;
        info!(id, "Vault unlock request received");

        let mut handlers = self.handlers.lock().await;
        let handler = handlers
            .entry(id)
            .or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(handler.unlock().await).await;
    }

    #[instrument(skip(self), err)]
    async fn change_mount_point(
        &self,
        request: Request<StringIdRequest>,
    ) -> Result<Response<EmptyReply>, Status> {
        let request = request.into_inner();
        let id = request.id;
        info!(id, "Vault change mount point request received");

        let mut handlers = self.handlers.lock().await;
        let handler = handlers
            .entry(id)
            .or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(
            handler.change_mount_point(request.value).await,
        )
        .await;
    }

    #[instrument(skip(self), err)]
    async fn change_data_dir(
        &self,
        request: Request<StringIdRequest>,
    ) -> Result<Response<EmptyReply>, Status> {
        let request = request.into_inner();
        let id = request.id;
        info!(id, "Vault change data dir request received");

        let mut handlers = self.handlers.lock().await;
        let handler = handlers
            .entry(id)
            .or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(
            handler.change_data_dir(request.value).await,
        )
        .await;
    }
}
