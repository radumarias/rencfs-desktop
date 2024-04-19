use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Arc;
use diesel::SqliteConnection;

use tokio::sync::{Mutex, RwLock};
use tonic::{Code, Request, Response, Status};
use tonic_types::{ErrorDetails, StatusExt};
use tracing::info;
use encrypted_fs_desktop_common::vault_handler::{VaultHandler, VaultHandlerError};
use encrypted_fs_desktop_common::vault_service_error::VaultServiceError;

use crate::vault_service::vault_service_server::VaultService;

tonic::include_proto!("encrypted_fs_desktop");

pub struct MyVaultService{
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

    async fn handle_handler_empty_response(response: Result<(), VaultHandlerError>) -> Result<Response<EmptyReply>, Status> {
        match response {
            Ok(_) => Ok(Response::new(EmptyReply {})),
            Err(err) => {
                Err(VaultServiceError::from(err).into())
            }
        }
    }
}

#[tonic::async_trait]
impl VaultService for MyVaultService {
    async fn lock(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        let id = request.into_inner().id;
        info!("Vault {} lock request received", id);

        let mut handlers = self.handlers.lock().await;
        let handler = handlers.entry(id).or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(handler.lock().await).await;
    }

    async fn unlock(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        let id = request.into_inner().id;
        info!("Vault {} unlock request received", id);

        let mut handlers = self.handlers.lock().await;
        let handler = handlers.entry(id).or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(handler.unlock().await).await;
    }

    async fn change_mount_point(&self, request: Request<StringIdRequest>) -> Result<Response<EmptyReply>, Status> {
        let request = request.into_inner();
        let id = request.id;
        info!("Vault {} change mount point request received", id);

        let mut handlers = self.handlers.lock().await;
        let handler = handlers.entry(id).or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(handler.change_mount_point(request.value).await).await;
    }

    async fn change_data_dir(&self, request: Request<StringIdRequest>) -> Result<Response<EmptyReply>, Status> {
        let request = request.into_inner();
        let id = request.id;
        info!("Vault {} change data dir request received", id);

        let mut handlers = self.handlers.lock().await;
        let handler = handlers.entry(id).or_insert_with(|| VaultHandler::new(id, self.db_conn.clone()));

        return MyVaultService::handle_handler_empty_response(handler.change_data_dir(request.value).await).await;
    }
}
