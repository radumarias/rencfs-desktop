use std::{fs, sync};
use std::sync::mpsc::Sender;
use std::time::Duration;
use sync::mpsc::Receiver;

use diesel::{ExpressionMethods, QueryResult};
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error::DatabaseError;
use eframe::{egui, Frame};
use eframe::egui::Context;
use egui::{Button, ecolor, Widget};
use egui_notify::{Toast, Toasts};
use tracing::instrument;

use daemon_service::DaemonService;
use rencfs_desktop_common::is_debug;
use rencfs_desktop_common::models::NewVault;
use rencfs_desktop_common::schema::vaults::{data_dir, mount_point, name};
use rencfs_desktop_common::vault_service_error::VaultServiceError;

use crate::daemon_service::EmptyReply;
use crate::dashboard::{Item, UiReply};
use crate::detail::db_service::DbService;

mod daemon_service;
mod db_service;

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

    tx_parent: Sender<UiReply>,
    rx_service: Receiver<ServiceReply>,

    daemon_service: DaemonService,
    db_service: DbService,

    confirmation_delete_pending: bool,

    toasts: Toasts,
}

impl eframe::App for ViewGroupDetail {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
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
                    self.tx_parent.send(UiReply::VaultUpdated(false)).unwrap();
                }
                ServiceReply::LockVaultReply(_) => {
                    self.locked = true;
                    customize_toast(self.toasts.success("vault locked"));
                    self.tx_parent.send(UiReply::VaultUpdated(false)).unwrap();
                }
                ServiceReply::ChangeMountPoint(_) => {
                    self.db_reload();
                    customize_toast(self.toasts.success("mount point changed"));
                }
                ServiceReply::ChangeDataDir(_) => {
                    self.db_reload();
                    customize_toast(self.toasts.success("data dir changed"));
                }
                ServiceReply::VaultServiceError(err) => customize_toast(self.toasts.error(err.to_string())),
                ServiceReply::Error(s) => customize_toast(self.toasts.error(s.clone())),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                if self.id.is_some() {
                    ui.horizontal(|ui| {
                        ui.set_max_width(80.0);
                        ui.vertical_centered(|ui| {
                            if Button::new(if self.locked { "Unlock vault" } else { "Lock vault" })
                                .fill(if self.locked { ecolor::Color32::DARK_GRAY } else { ecolor::Color32::DARK_GREEN })
                                .min_size(egui::vec2(80.0, 30.0))
                                .ui(ui).on_hover_ui(|ui| {
                                ui.label(if self.locked { "Unlock the vault" } else { "Lock the vault" });
                            }).clicked() {
                                if self.locked {
                                    self.daemon_service.unlock_vault();
                                    customize_toast_duration(self.toasts.warning("please wait, it takes up to 10 seconds to unlock the vault, you will be notified"), 8);
                                } else {
                                    self.daemon_service.lock_vault();
                                }
                            }
                        });
                    });
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
                            if let Some(path) = &self.mount_point {
                                ui.horizontal(|ui| {
                                    ui.monospace(path);
                                });
                            }
                        });
                    });
                    if ui.button("...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            if self.id.is_some() && path.to_string_lossy() == self.mount_point.as_ref().unwrap().as_str() {
                                customize_toast(self.toasts.error("you need to select a different path than existing one"));
                            } else {
                                if fs::read_dir(path.clone()).unwrap().count() > 0 {
                                    customize_toast(self.toasts.error("mount point must be empty"));
                                } else {
                                    let path = path.display().to_string();
                                    if self.id.is_some() {
                                        if !self.locked {
                                            customize_toast_duration(self.toasts.warning("please wait, it takes up to 10 seconds to change mount point, you will be notified"), 8);
                                            customize_toast_duration(self.toasts.warning("it will lock the vault meanwhile"), 8)
                                        }
                                        let old_path = self.mount_point.as_ref().unwrap().clone();
                                        self.db_service.update(mount_point.eq(path.clone()));
                                        self.daemon_service.change_mount_point(old_path);
                                    }
                                    self.mount_point = Some(path);
                                }
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Data dir");
                    ui.push_id(1001, |ui| {
                        egui::ScrollArea::horizontal().
                            max_width(400.0).show(ui, |ui| {
                            if let Some(path) = &self.data_dir {
                                ui.horizontal(|ui| {
                                    ui.monospace(path);
                                });
                            }
                        });
                    });
                    if ui.button("...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            if self.id.is_some() && path.to_string_lossy() == self.data_dir.as_ref().unwrap().as_str() {
                                customize_toast(self.toasts.error("you need to select a different path than existing one"));
                            } else {
                                if !is_debug() && fs::read_dir(path.clone()).unwrap().count() > 0 {
                                    customize_toast(self.toasts.error("data dir must be empty"));
                                } else {
                                    let path = path.display().to_string();
                                    if self.id.is_some() {
                                        if !self.locked {
                                            customize_toast_duration(self.toasts.warning("it could take longer to move the data to the new location, you will be notified"), 8);
                                            customize_toast_duration(self.toasts.warning("it will lock the vault meanwhile"), 8)
                                        }
                                        let old_path = self.data_dir.as_ref().unwrap().clone();
                                        self.db_service.update(data_dir.eq(path.clone()));
                                        self.daemon_service.change_data_dir(old_path);
                                    }
                                    self.data_dir = Some(path);
                                }
                            }
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
                                match self.db_insert() {
                                    Ok(_) => {
                                        self.tx_parent.send(UiReply::VaultInserted).unwrap();
                                        msg = Some(format!("vault {} saved", self.name));
                                    }
                                    Err(DatabaseError(UniqueViolation, _)) => {
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
                        if Button::new(if !self.confirmation_delete_pending { "Delete" } else { "Confirm DELETE" })
                            .fill(ecolor::Color32::DARK_RED)
                            .ui(ui).on_hover_ui(|ui| {
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
                                if let Err(err) = self.db_service.delete() {
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
    #[instrument(name = "ViewGroupDetail", skip(tx_parent), err)]
    pub fn new(tx_parent: Sender<UiReply>) -> Result<Self, String> {
        let (tx_service, rx_service) = sync::mpsc::channel::<ServiceReply>();
        let daemon_service = DaemonService::new(None, tx_service.clone(), tx_parent.clone());
        if let Err(err) = daemon_service {
            return Err(err);
        }
        let daemon_service = daemon_service.unwrap();

        Ok(ViewGroupDetail {
            id: None,
            name: "".to_string(),
            mount_point: None,
            data_dir: None,
            locked: true,
            confirmation_delete_pending: false,
            rx_service,
            tx_parent: tx_parent.clone(),
            daemon_service,
            db_service: DbService::new(None, tx_parent),
            toasts: Toasts::default(),
        })
    }

    #[instrument(name = "ViewGroupDetail", skip(tx_parent), err)]
    pub fn new_by_item(item: Item, tx_parent: Sender<UiReply>) -> Result<Self, String> {
        let (tx_service, rx_service) = sync::mpsc::channel::<ServiceReply>();
        let daemon_service = DaemonService::new(Some(item.id), tx_service.clone(), tx_parent.clone());
        if let Err(err) = daemon_service {
            return Err(err);
        }
        let daemon_service = daemon_service.unwrap();

        Ok(ViewGroupDetail {
            id: Some(item.id),
            name: item.name,
            mount_point: Some(item.mount_point),
            data_dir: Some(item.data_dir),
            locked: item.locked,
            confirmation_delete_pending: false,
            rx_service,
            tx_parent: tx_parent.clone(),
            daemon_service,
            db_service: DbService::new(Some(item.id), tx_parent),
            toasts: Toasts::default(),
        })
    }

    fn db_insert(&mut self) -> QueryResult<()> {
        let new_vault = NewVault {
            name: self.name.clone(),
            mount_point: self.mount_point.as_ref().unwrap().clone(),
            data_dir: self.data_dir.as_ref().unwrap().clone(),
        };
        self.db_service.insert(new_vault)
    }

    fn db_reload(&mut self) {
        let vault = self.db_service.get_vault().unwrap();
        self.name = vault.name;
        self.mount_point = Some(vault.mount_point);
        self.data_dir = Some(vault.data_dir);
        self.locked = vault.locked == 1;
        self.tx_parent.send(UiReply::VaultUpdated(false)).unwrap();
    }

    fn ui_on_name_lost_focus(&mut self) {
        if let Some(_) = self.id {
            let old_name = self.db_service.get_vault().unwrap().name;
            if old_name != self.name {
                self.db_service.update(name.eq(self.name.clone()));
                self.tx_parent.send(UiReply::VaultUpdated(true)).unwrap();
            }
        }
    }
}
