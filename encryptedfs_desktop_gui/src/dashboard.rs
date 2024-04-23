use std::borrow::Cow;
use std::sync;
use std::sync::RwLock;

use diesel::SqliteConnection;
use eframe::egui::{
    CentralPanel, Color32, Context, FontId, Margin, RichText, SidePanel, TopBottomPanel,
};
use eframe::egui;
use eframe::emath::Align;
use egui::{Frame, Layout, Ui};
use egui_notify::Toasts;

use encryptedfs_desktop_common::dao::VaultDao;

use crate::detail::ViewGroupDetail;
use crate::ListView;
use crate::listview::r#trait::ItemTrait;
use crate::listview::state::State;
use crate::util::customize_toast;

static CURRENT_VAULT_ITEM: RwLock<Option<Item>> = RwLock::new(None);
static CURRENT_VAULT_ID: RwLock<Option<i32>> = RwLock::new(None);

pub(crate) enum UiReply {
    VaultInserted,
    VaultUpdated(bool),
    VaultDeleted,
    GoBack,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub mount_point: String,
    pub data_dir: String,
    pub locked: bool,
}

impl ItemTrait for Item {
    type Data<'a> = ();

    fn id(&self, _data: Self::Data<'_>) -> egui::Id {
        egui::Id::new(self.id)
    }

    fn style_clicked(&self, frame: &mut Frame) {
        frame.fill = if self.locked { Color32::DARK_GRAY } else { Color32::DARK_GREEN };
    }

    fn show(
        &self,
        selected: bool,
        _hover: bool,
        _ctx: &Context,
        ui: &mut Ui,
        _data: Self::Data<'_>,
    ) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.set_min_height(42.0);

                    ui.label(RichText::new(if self.locked {
                        format!("🔒 {}", self.name)
                    } else {
                        format!("🔓 {}", self.name)
                    }).size(20.0).strong());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |_ui| {});
            });
        });

        if selected && *CURRENT_VAULT_ID.read().unwrap() != Some(self.id) {
            *CURRENT_VAULT_ID.write().unwrap() = Some(self.id);
            *CURRENT_VAULT_ITEM.write().unwrap() = Some(self.clone());
        }
    }

    fn hovered_text(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn selected_item(&self, _data: Self::Data<'_>) {}

    fn on_search(&self, text: &str, _data: Self::Data<'_>) -> bool {
        self.name.contains(text)
    }
}

pub(crate) struct Dashboard {
    conn: SqliteConnection,
    pub(crate) items: Vec<Item>,
    pub(crate) state: Option<State>,
    prev_state: Option<State>,

    tx: sync::mpsc::Sender<UiReply>,
    rx: sync::mpsc::Receiver<UiReply>,

    toasts: Toasts,
}

impl Dashboard {
    pub(crate) fn new(conn: SqliteConnection) -> Self {
        let (tx, rx) = sync::mpsc::channel::<UiReply>();
        let mut out = Self {
            conn,
            items: vec![],
            state: None,
            prev_state: None,
            tx,
            rx,
            toasts: Toasts::default(),
        };
        out.items = out.load_items();
        if out.items.len() > 0 {
            *CURRENT_VAULT_ID.write().unwrap() = Some(out.items[0].id);
            *CURRENT_VAULT_ITEM.write().unwrap() = Some(out.items[0].clone());
        }

        out
    }

    fn load_items(&mut self) -> Vec<Item> {
        let mut dao = VaultDao::new(&mut self.conn);
        dao.get_all(None).unwrap().iter().map(|v| {
            Item {
                id: v.id,
                name: v.name.clone(),
                mount_point: v.mount_point.clone(),
                data_dir: v.data_dir.clone(),
                locked: if v.locked == 1 { true } else { false },
            }
        }).collect()
    }
}

impl eframe::App for Dashboard {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(msg) = self.rx.try_recv() {
            match msg {
                UiReply::VaultUpdated(show_message) => {
                    if show_message {
                        customize_toast(self.toasts.success("vault updated"));
                    }
                    self.items = self.load_items();
                }
                UiReply::GoBack => {
                    if let Some(state) = self.prev_state.take() {
                        self.state = Some(state);
                    } else {
                        self.state = None;
                    }
                }
                UiReply::VaultDeleted => {
                    self.state = None;
                    self.items = self.load_items();
                }
                UiReply::VaultInserted => {
                    self.state = None;
                    self.items = self.load_items();
                }
                UiReply::Error(err) => customize_toast(self.toasts.error(err)),
            }
        }

        ctx.set_pixels_per_point(1.5);

        TopBottomPanel::top("top_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().button_frame = false;
                // TODO: keep in config
                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });
        SidePanel::left("order_group_list")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                egui::Frame::default().outer_margin(6.0).show(ui, |ui| {
                    ui.with_layout(
                        Layout::bottom_up(Align::Center).with_cross_justify(true),
                        |ui| {
                            let mut reset_list_selection = false;
                            Frame::default()
                                .outer_margin({
                                    let mut margin = Margin::default();
                                    margin.top = 6.0;
                                    margin
                                })
                                .show(ui, |ui| {
                                    if ui.button("Add Vault").clicked() {
                                        reset_list_selection = true;
                                        match ViewGroupDetail::new(self.tx.clone()) {
                                            Ok(v) => self.state = Some(State::Detail(v)),
                                            Err(err) => customize_toast(self.toasts.error(err)),
                                        }
                                    }
                                });

                            let mut margin = Margin::default();
                            margin.bottom = 60.0;
                            let mut list_view = ListView::new(self.items.iter(), ());
                            if reset_list_selection {
                                list_view = list_view.reset_selection();
                                if CURRENT_VAULT_ID.read().unwrap().is_some() {
                                    CURRENT_VAULT_ID.write().unwrap().take();
                                    CURRENT_VAULT_ITEM.write().unwrap().take();
                                }
                            }
                            if CURRENT_VAULT_ITEM.read().unwrap().is_some() {
                                list_view = list_view.selected_item(CURRENT_VAULT_ITEM.read().unwrap().as_ref().unwrap().id(()));
                            }
                            list_view
                                .striped()
                                .show(ctx, ui);
                            if CURRENT_VAULT_ITEM.read().unwrap().is_some() {
                                let mut writer = CURRENT_VAULT_ITEM.write().unwrap();
                                if let Some(item) = std::mem::take(&mut *writer) {
                                    match ViewGroupDetail::new_by_item(item, self.tx.clone()) {
                                        Ok(v) => self.state = Some(State::Detail(v)),
                                        Err(err) => customize_toast(self.toasts.error(err)),
                                    }
                                }
                            }
                        },
                    );
                });
            });
        if let Some(state) = self.state.as_mut() {
            state.as_app().update(ctx, frame);
        } else {
            let text = if self.items.len() > 0 {
                "Select a vault or add a new one"
            } else {
                "No vaults found, add a new one"
            };
            CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new(text)
                                 .font(FontId::proportional(20.0))
                                 .color(Color32::GRAY),
                    )
                });
            });
        }

        self.toasts.show(ctx);
    }
}
