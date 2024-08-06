use std::sync::mpsc::{Sender};
use tonic::{Response, Status};
use tonic::transport::{Channel, Error};
use tracing::{error, instrument};
use rencfs_desktop_common::vault_service_error::VaultServiceError;
use crate::daemon_service::{EmptyReply, HelloRequest, IdRequest, StringIdRequest};
use crate::daemon_service::vault_service_client::VaultServiceClient;
use crate::dashboard::UiReply;
use crate::detail::ServiceReply;
use crate::RT;

pub(super) struct DaemonService {
    id: Option<i32>,
    tx_service: Sender<ServiceReply>,
    tx_parent: Sender<UiReply>,
    client: VaultServiceClient<Channel>,
}

impl DaemonService {
    #[instrument(name = "DaemonService::new", skip(tx_service, tx_parent), err)]
    pub(super) fn new(id: Option<i32>, tx_service: Sender<ServiceReply>, tx_parent: Sender<UiReply>) -> Result<Self, String> {
        let (tx2, tx_p2) = (tx_service.clone(), tx_parent.clone());
        RT.block_on(async {
            Self::create_client(tx2, tx_p2).await
        })
            .map_or_else(|err| {
                Err(format!("failed to connect to daemon: {}", err.to_string()))
            }, |client| Ok(Self { id, tx_service, tx_parent, client }))
    }

    pub(super) fn hello(&mut self, name: &str) {
        let tx = self.tx_service.clone();
        let tx_parent = self.tx_parent.clone();
        let mut client = self.client.clone();
        let name = name.to_string();
        RT.spawn(async move {
            let request = tonic::Request::new(HelloRequest {
                name,
            });
            match client.hello(request).await {
                Ok(response) => {
                    let _ = tx.send(ServiceReply::HelloReply(response.into_inner()))
                        .map_err(|_| {
                            // in case the component is destroyed before the response is received,
                            // we will not be able
                            // to notify service reply because the rx is closed
                            // in that case notify parent with error because it's rx is still open
                            let _ = tx_parent.send(UiReply::VaultUpdated(true));
                        });
                }
                Err(err) => {
                    let vault_service_error: Result<VaultServiceError, _> = err.clone().try_into();
                    match vault_service_error {
                        Ok(err2) => {
                            error!(err2 = %err2);
                            let _ = tx.send(ServiceReply::VaultServiceError(err2.clone()))
                                .map_err(|_| {
                                    // in case the component is destroyed before the response is received,
                                    // we will not be able
                                    // to notify service reply because the rx is closed
                                    // in that case notify parent with error because it's rx is still open
                                    let _ = tx_parent.send(UiReply::Error(err2.to_string()));
                                });
                        }
                        _ => {
                            error!(err = %err);
                            let res = tx.send(ServiceReply::Error(format!("Error: {}", err)));
                            if let Err(err) = res {
                                // in case the component is destroyed before the response is received,
                                // we will not be able
                                // to notify service reply because the rx is closed
                                // in that case notify parent with error because it's rx is still open
                                let _ = tx_parent.send(UiReply::Error(err.to_string()));
                            }
                        }
                    }
                }
            }
        });
    }

    pub(super) fn unlock_vault(&mut self) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        let tx = self.tx_service.clone();
        let tx_parent = self.tx_parent.clone();
        let mut client = self.client.clone();
        RT.spawn(async move {
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
        let mut client = self.client.clone();
        RT.spawn(async move {
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
        let mut client = self.client.clone();
        RT.spawn(async move {
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
        let mut client = self.client.clone();
        RT.spawn(async move {
            let request = tonic::Request::new(StringIdRequest {
                id,
                value,
            });
            Self::handle_empty_response(client.change_data_dir(request).await, ServiceReply::ChangeDataDir, tx, tx_parent);
        });
    }

    #[instrument(skip(f))]
    fn handle_empty_response(result: Result<Response<EmptyReply>, Status>, f: impl FnOnce(EmptyReply) -> ServiceReply,
                             tx: Sender<ServiceReply>, tx_parent: Sender<UiReply>) {
        match result {
            Ok(response) => {
                let _ = tx.send(f(response.into_inner()))
                    .map_err(|_| {
                        // in case the component is destroyed before the response is received,
                        // we will not be able
                        // to notify service reply because the rx is closed
                        // in that case notify parent with error because it's rx is still open
                        let _ = tx_parent.send(UiReply::VaultUpdated(true));
                    });
            }
            Err(err) => {
                let vault_service_error: Result<VaultServiceError, _> = err.clone().try_into();
                match vault_service_error {
                    Ok(err2) => {
                        error!(err2 = %err2);
                        let _ = tx.send(ServiceReply::VaultServiceError(err2.clone()))
                            .map_err(|_| {
                                // in case the component is destroyed before the response is received,
                                // we will not be able
                                // to notify service reply because the rx is closed
                                // in that case notify parent with error because it's rx is still open
                                let _ = tx_parent.send(UiReply::Error(err2.to_string()));
                            });
                    }
                    _ => {
                        error!(err = %err);
                        let res = tx.send(ServiceReply::Error(format!("Error: {}", err)));
                        if let Err(err) = res {
                            // in case the component is destroyed before the response is received,
                            // we will not be able
                            // to notify service reply because the rx is closed
                            // in that case notify parent with error because it's rx is still open
                            let _ = tx_parent.send(UiReply::Error(err.to_string()));
                        }
                    }
                }
            }
        }
    }

    async fn create_client(tx: Sender<ServiceReply>, tx_parent: Sender<UiReply>) -> Result<VaultServiceClient<Channel>, Error> {
        // TODO: https://github.com/radumarias/rencfs-desktop/issues/13
        VaultServiceClient::connect("http://[::1]:50051").await
            .map_err(|err| {
                let _ = tx.send(ServiceReply::Error(format!("{err:?}")))
                    .map_err(|err| {
                        // in case the component is destroyed before the response is received,
                        // we will not be able
                        // to notify service reply because the rx is closed
                        // in that case notify parent with error because it's rx is still open
                        let _ = tx_parent.send(UiReply::Error(err.to_string()));
                    });
                err
            })
    }
}