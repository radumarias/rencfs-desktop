use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};
use tonic::{Code, Request, Response, Status};
use tonic_types::{ErrorDetails, StatusExt};
use tracing::info;
use encrypted_fs_desktop_common::vault_handler::VaultHandler;
use encrypted_fs_desktop_common::vault_service_error::VaultServiceError;

use crate::vault_service::vault_service_server::VaultService;

tonic::include_proto!("encrypted_fs_desktop");

pub struct MyVaultService {
    handlers: Arc<RwLock<HashMap<u32, VaultHandler>>>,
}

impl MyVaultService {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl VaultService for MyVaultService {
    async fn lock(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        let id = request.into_inner().id;
        info!("Vault {} lock request received", id);

        let mut handlers = self.handlers.write().await;
        let handler = handlers.entry(id).or_insert_with(|| VaultHandler::new(id));
        match handler.lock().await {
            Ok(_) => Ok(Response::new(EmptyReply {})),
            Err(err) => {
                Err(VaultServiceError::from(err).into())
            }
        }
    }

    async fn unlock(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        let id = request.into_inner().id;
        info!("Vault {} unlock request received", id);

        let mut handlers = self.handlers.write().await;
        let handler = handlers.entry(id).or_insert_with(|| VaultHandler::new(id));
        match handler.unlock().await {
            Ok(_) => Ok(Response::new(EmptyReply {})),
            Err(err) => {
                Err(VaultServiceError::from(err).into())
            }
        }
    }

    async fn change_mount_point(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        todo!()
    }

    async fn change_data_dir(&self, request: Request<IdRequest>) -> Result<Response<EmptyReply>, Status> {
        todo!()
    }
}
