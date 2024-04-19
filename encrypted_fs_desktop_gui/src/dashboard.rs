use std::borrow::Cow;
use std::sync;
use std::sync::RwLock;

use diesel::SqliteConnection;
use eframe::egui::{
    CentralPanel, Color32, Context, FontId, Margin, RichText, SidePanel, TopBottomPanel,
};
use eframe::egui;
use eframe::emath::Align;
use egui::{Layout, Ui};

use encrypted_fs_desktop_common::dao::VaultDao;

use crate::detail::ViewGroupDetail;
use crate::ListView;
use crate::listview::r#trait::ItemTrait;
use crate::listview::state::State;

static CURRENT_VAULT_ITEM: RwLock<Option<Item>> = RwLock::new(None);
static CURRENT_VAULT_ID: RwLock<Option<i32>> = RwLock::new(None);

pub(crate) enum UiReply {
    VaultInserted,
    VaultUpdated,
    VaultDeleted,
    GoBack,
}

#[derive(Clone)]
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
                    ui.label(self.name.clone());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(format!("ID {}", self.id));
                });
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
        };
        out.items = Self::load_items(&mut out.conn);

        out
    }

    fn load_items(conn: &mut SqliteConnection) -> Vec<Item> {
        let mut dao = VaultDao::new(conn);
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
                UiReply::VaultUpdated => {
                    self.items = Self::load_items(&mut self.conn);
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
                    self.items = Self::load_items(&mut self.conn);
                }
                UiReply::VaultInserted => {
                    self.state = None;
                    self.items = Self::load_items(&mut self.conn);
                }
            }
        }

        ctx.set_pixels_per_point(1.5);

        TopBottomPanel::top("top_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().button_frame = false;
                // TODO: take from config
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
            });
        });
        SidePanel::left("order_group_list")
            .resizable(false)
            .default_width(250.0)
            .show(ctx, |ui| {
                egui::Frame::default().outer_margin(6.0).show(ui, |ui| {
                    ui.with_layout(
                        Layout::bottom_up(Align::Center).with_cross_justify(true),
                        |ui| {
                            egui::Frame::default()
                                .outer_margin({
                                    let mut margin = Margin::default();
                                    margin.top = 6.0;
                                    margin
                                })
                                .show(ui, |ui| {
                                    if ui.button("Add Vault").clicked() {
                                        if self.prev_state.is_none() {
                                            self.prev_state = self.state.take();
                                        }
                                        self.state = Some(State::Detail(
                                            ViewGroupDetail::new(self.tx.clone()),
                                        ));
                                    }
                                });

                            let mut margin = Margin::default();
                            margin.bottom = 60.0;
                            ListView::new(self.items.iter(), ())
                                .title("Vaults".into())
                                .striped()
                                .show(ctx, ui);
                            if CURRENT_VAULT_ITEM.read().unwrap().is_some() {
                                let mut writer = CURRENT_VAULT_ITEM.write().unwrap();
                                if let Some(item) = std::mem::take(&mut *writer) {
                                    self.prev_state = self.state.take();
                                    self.state = Some(State::Detail(
                                        ViewGroupDetail::new_by_item(item, self.tx.clone()),
                                    ));
                                }
                            }
                        },
                    );
                });
            });
        if let Some(state) = self.state.as_mut() {
            state.as_app().update(ctx, frame);
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("Nothing Selected")
                            .font(FontId::proportional(20.0))
                            .color(Color32::GRAY),
                    )
                });
            });
        }
    }
}
