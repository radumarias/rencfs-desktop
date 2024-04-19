use std::convert::Infallible;
use std::{fs, sync};
use std::sync::mpsc::Sender;
use std::time::Duration;
use sync::mpsc::Receiver;
use diesel::{ExpressionMethods, QueryResult};
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error::DatabaseError;
use dotenvy::dotenv;

use eframe::{egui, Frame};
use eframe::egui::Context;
use egui::{Button, ecolor, Widget};
use encrypted_fs_desktop_common::dao::VaultDao;
use encrypted_fs_desktop_common::models::NewVault;
use eframe::emath::Align2;
use eframe::epaint::FontId;
use egui_notify::{Toast, Toasts};
use tonic::{Response, Status};
use tonic::transport::Channel;
use tracing::{debug, error};
use encrypted_fs_desktop_common::schema::vaults::name;
use encrypted_fs_desktop_common::vault_service_error::{VaultServiceError};

use crate::daemon_service::vault_service_client::VaultServiceClient;
use crate::dashboard::{Item, UiReply};
use crate::{DB_CONN, RT};
use crate::daemon_service::{EmptyReply, IdRequest, StringIdRequest};

enum ServiceReply {
    UnlockVaultReply(EmptyReply),
    LockVaultReply(EmptyReply),
    ChangeMountPoint(EmptyReply),
    ChangeDataDir(EmptyReply),
    VaultServiceError(VaultServiceError),
    Error(String),
}

pub struct ViewGroupDetail {
    pub(crate) id: Option<i32>,
    pub(crate) name: String,
    pub(crate) mount_point: Option<String>,
    pub(crate) data_dir: Option<String>,
    pub(crate) locked: bool,

    confirmation_delete_pending: bool,

    tx_service: Sender<ServiceReply>,
    rx_service: Receiver<ServiceReply>,
    tx_parent: Sender<UiReply>,

    toasts: Toasts,
}

impl eframe::App for ViewGroupDetail {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let customize_toast_duration = |t: &mut Toast, seconds: u64| {
            let duration = Some(Duration::from_secs(seconds));
            t.set_closable(false)
                .set_duration(duration)
                .set_show_progress_bar(false);
        };
        let customize_toast = |t: &mut Toast| {
            customize_toast_duration(t, 5);
        };
        if let Ok(reply) = self.rx_service.try_recv() {
            match reply {
                ServiceReply::UnlockVaultReply(_) => {
                    self.locked = false;
                    customize_toast(self.toasts.success("vault unlocked"));
                    self.tx_parent.send(UiReply::VaultUpdated).unwrap();
                }
                ServiceReply::LockVaultReply(_) => {
                    self.locked = true;
                    customize_toast(self.toasts.success("vault locked"));
                    self.tx_parent.send(UiReply::VaultUpdated).unwrap();
                }
                ServiceReply::ChangeMountPoint(_) => {
                    customize_toast(self.toasts.success("mount point changed"));
                    self.tx_parent.send(UiReply::VaultUpdated).unwrap();
                }
                ServiceReply::ChangeDataDir(_) => {
                    customize_toast(self.toasts.success("data dir changed"));
                    self.tx_parent.send(UiReply::VaultUpdated).unwrap();
                }
                ServiceReply::VaultServiceError(err) => customize_toast(self.toasts.error(err.to_string())),
                ServiceReply::Error(s) => customize_toast(self.toasts.error(s.clone())),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label("Vault Detail");
                ui.separator();
                if self.id.is_some() {
                    if Button::new(if self.locked { "Unlock vault" } else { "Lock vault" })
                        .fill(if self.locked { ecolor::Color32::DARK_GRAY } else { ecolor::Color32::DARK_GREEN })
                        .ui(ui).on_hover_ui(|ui| {
                        ui.label(if self.locked { "Unlock the vault" } else { "Lock the vault" });
                    }).clicked() {
                        if self.locked {
                            self.service_unlock_vault(self.tx_service.clone());
                            customize_toast_duration(self.toasts.info("please wait, it takes up to 10 seconds to unlock the vault, you will be notified"), 10)
                        } else {
                            self.service_lock_vault(self.tx_service.clone());
                        }
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Name");
                    if ui.text_edit_singleline(&mut self.name).lost_focus() {
                        self.ui_on_name_lost_focus();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Mount point");
                    ui.push_id(1000, |ui| {
                        egui::ScrollArea::horizontal().
                            max_width(400.0).show(ui, |ui| {
                            if let Some(picked_path) = &self.mount_point {
                                ui.horizontal(|ui| {
                                    ui.monospace(picked_path);
                                });
                            }
                        });
                    });
                    if ui.button("...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            if fs::read_dir(path.clone()).unwrap().count() > 0 {
                                customize_toast(self.toasts.error("mount point must be empty"));
                            } else {
                                self.mount_point = Some(path.display().to_string());
                                self.service_change_mount_point(path.display().to_string(), self.tx_service.clone());
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Data dir");
                    ui.push_id(1001, |ui| {
                        egui::ScrollArea::horizontal().
                            max_width(400.0).show(ui, |ui| {
                            if let Some(picked_path) = &self.data_dir {
                                ui.horizontal(|ui| {
                                    ui.monospace(picked_path);
                                });
                            }
                        });
                    });
                    if ui.button("...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            // TODO move dotenv() to global variable
                            // if dotenv().is_err() {
                            if fs::read_dir(path.clone()).unwrap().count() > 0 {
                                customize_toast(self.toasts.error("data dir must be empty"));
                            } else {
                                self.data_dir = Some(path.display().to_string());
                                self.service_change_data_dir(path.display().to_string(), self.tx_service.clone());
                            }
                            // }
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if self.id.is_none() {
                        if ui.button("Save").clicked() {
                            self.name = self.name.trim().to_string();

                            let mut msg = None;
                            let mut err = None;
                            if self.name.is_empty() {
                                err = Some("invalid name".into());
                            } else if self.mount_point.is_none() {
                                err = Some("invalid mount point".into());
                            } else if self.data_dir.is_none() {
                                err = Some("invalid data dir".into());
                            } else {
                                match self.db_save() {
                                    Ok(_) => {
                                        self.tx_parent.send(UiReply::VaultInserted).unwrap();
                                        msg = Some(format!("vault {} saved", self.name));
                                    }
                                    Err(DatabaseError((UniqueViolation), _)) => {
                                        err = Some(format!("another vault named {} exists", self.name));
                                    }
                                    Err(err2) => {
                                        err = Some(format!("failed to save: {:?}", err2));
                                    }
                                }
                            }
                            if msg.is_some() {
                                customize_toast(self.toasts.success(msg.unwrap()))
                            }
                            if err.is_some() {
                                customize_toast(self.toasts.error(err.unwrap()))
                            }
                        }
                    }

                    if self.id.is_some() {
                        let mut button = Button::new(if !self.confirmation_delete_pending { "Delete" } else { "Confirm DELETE" });
                        if self.confirmation_delete_pending {
                            button = button.fill(ecolor::Color32::DARK_RED)
                        }
                        if button.ui(ui).on_hover_ui(|ui| {
                            ui.label("Delete vault");
                        }).clicked() {
                            if !self.confirmation_delete_pending {
                                // ask for confirmation
                                self.confirmation_delete_pending = true;
                                customize_toast(self.toasts.error("click again to confirm delete"))
                            } else {
                                // confirmed, delete
                                self.confirmation_delete_pending = false;
                                // TODO move to service
                                if let Err(err) = self.db_delete() {
                                    customize_toast(self.toasts.error(format!("failed to delete: {:?}", err)))
                                } else {
                                    self.tx_parent.send(UiReply::VaultDeleted).unwrap();
                                    customize_toast(self.toasts.success("vault deleted"))
                                }
                            }
                        }
                        if self.confirmation_delete_pending {
                            if ui.button("Cancel").clicked() {
                                self.confirmation_delete_pending = false;
                            }
                        }
                    } else {
                        if ui.button("Cancel").clicked() {
                            self.tx_parent.send(UiReply::GoBack).unwrap();
                        }
                    }
                });
            });
        });

        self.toasts.show(ctx);
    }
}

impl ViewGroupDetail {
    pub fn new(tx_parent: Sender<UiReply>) -> Self {
        let (tx_service, rx_service) = sync::mpsc::channel::<ServiceReply>();

        ViewGroupDetail {
            id: None,
            name: "".to_string(),
            mount_point: None,
            data_dir: None,
            locked: true,
            confirmation_delete_pending: false,
            tx_service,
            rx_service,
            tx_parent,
            toasts: Toasts::default(),
        }
    }

    async fn create_client(tx: Sender<ServiceReply>) -> Result<VaultServiceClient<Channel>, ()> {
        // TODO: resolve port dynamically
        let mut client = VaultServiceClient::connect("http://[::1]:50051").await;
        if !client.is_err() {
            return Ok(client.unwrap());
        }
        let err = client.unwrap_err();
        error!("Error: {:?}", err);
        tx.send(ServiceReply::Error(format!("Error: {:?}", err))).unwrap();
        Err(())
    }

    pub fn new_by_item(item: Item, tx_parent: Sender<UiReply>) -> Self {
        let (tx_service, rx_service) = sync::mpsc::channel::<ServiceReply>();

        ViewGroupDetail {
            id: Some(item.id),
            name: item.name,
            mount_point: Some(item.mount_point),
            data_dir: Some(item.data_dir),
            locked: item.locked,
            confirmation_delete_pending: false,
            tx_service,
            rx_service,
            tx_parent,
            toasts: Toasts::default(),
        }
    }

    fn service_unlock_vault(&mut self, tx: Sender<ServiceReply>) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(IdRequest {
                id,
            });
            Self::handle_empty_response(client.unlock(request).await, ServiceReply::UnlockVaultReply, tx.clone());
        });
    }

    fn service_lock_vault(&mut self, tx: Sender<ServiceReply>) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(IdRequest {
                id,
            });
            Self::handle_empty_response(client.lock(request).await, ServiceReply::LockVaultReply, tx.clone());
        });
    }

    fn service_change_mount_point(&mut self, value: String, tx: Sender<ServiceReply>) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(StringIdRequest {
                id,
                value,
            });
            Self::handle_empty_response(client.change_mount_point(request).await, ServiceReply::ChangeMountPoint, tx.clone());
        });
    }

    fn service_change_data_dir(&mut self, value: String, tx: Sender<ServiceReply>) {
        let id = self.id.as_ref().unwrap().clone() as u32;
        RT.spawn(async move {
            let mut client = if let Ok(client) = Self::create_client(tx.clone()).await { client } else { return; };

            let request = tonic::Request::new(StringIdRequest {
                id,
                value,
            });
            Self::handle_empty_response(client.change_data_dir(request).await, ServiceReply::ChangeDataDir, tx);
        });
    }

    fn handle_empty_response(result: Result<Response<EmptyReply>, Status>, f: impl FnOnce(EmptyReply) -> ServiceReply, tx: Sender<ServiceReply>) {
        match result {
            Ok(response) => tx.send(f(response.into_inner())).unwrap(),
            Err(err) => {
                let vault_service_error: Result<VaultServiceError, _> = err.clone().try_into();
                match vault_service_error {
                    Ok(err2) => {
                        error!("Error: {}", err2);
                        tx.send(ServiceReply::VaultServiceError(err2)).unwrap()
                    }
                    _ => {
                        error!("Error: {}", err);
                        tx.send(ServiceReply::Error(format!("Error: {}", err))).unwrap()
                    }
                }
            }
        }
    }

    fn db_save(&mut self) -> QueryResult<()> {
        use encrypted_fs_desktop_common::schema::vaults::*;

        let mut lock = DB_CONN.lock().unwrap();
        let mut dao = VaultDao::new(&mut lock);
        if self.id.is_some() {
            dao.transaction(|mut dao| {
                dao.update(self.id.as_ref().unwrap().clone(), name.eq(self.name.clone()))?;
                dao.update(self.id.as_ref().unwrap().clone(), mount_point.eq(self.mount_point.as_ref().unwrap().clone()))?;
                dao.update(self.id.as_ref().unwrap().clone(), data_dir.eq(self.data_dir.as_ref().unwrap().clone()))?;
                dao.update(self.id.as_ref().unwrap().clone(), locked.eq(if self.locked { 1 } else { 0 }))?;

                Ok(1)
            })?;

            Ok(())
        } else {
            let vault = NewVault {
                name: self.name.clone(),
                mount_point: self.mount_point.as_ref().unwrap().clone(),
                data_dir: self.data_dir.as_ref().unwrap().clone(),
            };
            dao.insert(&vault)
        }
    }

    fn db_delete(&self) -> QueryResult<()> {
        let mut lock = DB_CONN.lock().unwrap();
        let mut dao = VaultDao::new(&mut lock);
        dao.delete(self.id.as_ref().unwrap().clone())
    }

    fn ui_on_name_lost_focus(&mut self) {
        if let Some(id_v) = self.id {
            let mut guard = DB_CONN.lock().unwrap();
            let mut dao = VaultDao::new(&mut guard);
            if dao.get(id_v).unwrap().name != self.name {
                dao.update(id_v, name.eq(self.name.clone())).unwrap();
                self.tx_parent.send(UiReply::VaultUpdated).unwrap();
                debug!("name updated");
            }
        }
    }
}
