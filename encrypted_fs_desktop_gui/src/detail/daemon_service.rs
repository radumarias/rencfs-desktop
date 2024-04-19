use std::sync::mpsc::{Sender};
use tonic::{Response, Status};
use tonic::transport::Channel;
use tracing::error;
use encrypted_fs_desktop_common::vault_service_error::VaultServiceError;
use crate::daemon_service::{EmptyReply, IdRequest, StringIdRequest};
use crate::daemon_service::vault_service_client::VaultServiceClient;
use crate::dashboard::UiReply;
use crate::detail::ServiceReply;
use crate::RT;

pub(super) struct DaemonService {
    id: Option<i32>,
    tx_service: Sender<ServiceReply>,
    tx_parent: Sender<UiReply>,
}

impl DaemonService {
    pub(super) fn new(id: Option<i32>, tx_service: Sender<ServiceReply>, tx_parent: Sender<UiReply>) -> Self {
        Self { id, tx_service, tx_parent }
    }

    pub(super) fn unlock_vault(&mut self) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        let tx = self.tx_service.clone();
        let tx_parent = self.tx_parent.clone();
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(IdRequest {
                id,
            });
            Self::handle_empty_response(client.unlock(request).await, ServiceReply::UnlockVaultReply, tx, tx_parent);
        });
    }

    pub(super) fn lock_vault(&mut self) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        let tx = self.tx_service.clone();
        let tx_parent = self.tx_parent.clone();
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(IdRequest {
                id,
            });
            Self::handle_empty_response(client.lock(request).await, ServiceReply::LockVaultReply, tx, tx_parent);
        });
    }

    pub(super) fn change_mount_point(&mut self, value: String) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        let tx = self.tx_service.clone();
        let tx_parent = self.tx_parent.clone();
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(StringIdRequest {
                id,
                value,
            });
            Self::handle_empty_response(client.change_mount_point(request).await, ServiceReply::ChangeMountPoint, tx, tx_parent);
        });
    }

    pub(super) fn change_data_dir(&mut self, value: String) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        let tx = self.tx_service.clone();
        let tx_parent = self.tx_parent.clone();
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(StringIdRequest {
                id,
                value,
            });
            Self::handle_empty_response(client.change_data_dir(request).await, ServiceReply::ChangeDataDir, tx, tx_parent);
        });
    }

    fn handle_empty_response(result: Result<Response<EmptyReply>, Status>, f: impl FnOnce(EmptyReply) -> ServiceReply,
                             tx: Sender<ServiceReply>, tx_parent: Sender<UiReply>) {
        match result {
            Ok(response) => {
                if let Err(_) = tx.send(f(response.into_inner())) {
                    // in case the component is destroyed before the response is received we will not be able to notify service reply because the rx is closed
                    // in that case notify parent for update because it's rx is still open
                    let _ = tx_parent.send(UiReply::VaultUpdated(true));
                }
            }
            Err(err) => {
                let vault_service_error: Result<VaultServiceError, _> = err.clone().try_into();
                match vault_service_error {
                    Ok(err2) => {
                        error!("Error: {}", err2);
                        if let Err(_) = tx.send(ServiceReply::VaultServiceError(err2.clone())) {
                            // in case the component is destroyed before the response is received we will not be able to notify service reply because the rx is closed
                            // in that case notify parent for update because it's rx is still open
                            let _ = tx_parent.send(UiReply::Error(err2.to_string()));
                        }
                    }
                    _ => {
                        error!("Error: {}", err);
                        let res = tx.send(ServiceReply::Error(format!("Error: {}", err)));
                        if let Err(err) = res {
                            // in case the component is destroyed before the response is received we will not be able to notify service reply because the rx is closed
                            // in that case notify parent for update because it's rx is still open
                            let _ = tx_parent.send(UiReply::Error(err.to_string()));
                        }
                    }
                }
            }
        }
    }

    async fn create_client(tx: Sender<ServiceReply>) -> Result<VaultServiceClient<Channel>, ()> {
        // TODO: resolve port dynamically
        let client = VaultServiceClient::connect("http://[::1]:50051").await;
        if !client.is_err() {
            return Ok(client.unwrap());
        }
        let err = client.unwrap_err();
        error!("Error: {:?}", err);
        tx.send(ServiceReply::Error(format!("Error: {:?}", err))).unwrap();
        Err(())
    }
}