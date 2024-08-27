use crate::listview::r#trait::ItemTrait;
use egui::{Align, Id, Label, Layout, Margin, RichText, Rounding, ScrollArea, Sense, TextEdit};
use std::borrow::Cow;

pub mod state;
pub mod r#trait;

pub struct ListView<'a, W: ItemTrait + 'a, L: Iterator<Item = &'a W>> {
    pub(crate) with_search: bool,
    pub(crate) title: Option<Cow<'a, str>>,
    pub(crate) hold_text: Option<Cow<'a, str>>,
    pub(crate) items: L,
    pub(crate) data: W::Data<'a>,
    pub(crate) inner_margin: Margin,
    pub(crate) outer_margin: Margin,
    pub(crate) rounding: Rounding,
    pub(crate) striped: bool,
    pub(crate) reset_selection: bool,
    pub(crate) selected_item_id: Option<Id>,
}

impl<'a, W: ItemTrait + 'a, L: Iterator<Item = &'a W>> ListView<'a, W, L> {
    pub fn new(items: L, data: W::Data<'a>) -> Self {
        Self {
            reset_selection: false,
            with_search: false,
            title: None,
            hold_text: None,
            items,
            data,
            inner_margin: Margin::same(3.0),
            outer_margin: Margin::default(),
            rounding: Rounding::default(),
            striped: false,
            selected_item_id: None,
        }
    }
}

impl<'a, W: ItemTrait + 'a, L: Iterator<Item = &'a W>> ListView<'a, W, L> {
    pub fn selected_item(mut self, id: Id) -> Self {
        self.selected_item_id = Some(id);
        self
    }

    pub fn reset_selection(mut self) -> Self {
        self.reset_selection = true;
        self
    }

    pub fn title(mut self, title: Cow<'a, str>) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_search(mut self, with_search: bool) -> Self {
        self.with_search = with_search;
        self
    }

    pub fn hold_text(mut self, text: Cow<'a, str>) -> Self {
        self.hold_text = Some(text);
        self
    }

    pub fn inner_margin(mut self, margin: Margin) -> Self {
        self.inner_margin = margin;
        self
    }

    pub fn outer_margin(mut self, margin: Margin) -> Self {
        self.outer_margin = margin;
        self
    }

    pub fn rounding(mut self, rounding: Rounding) -> Self {
        self.rounding = rounding;
        self
    }

    pub fn striped(mut self) -> Self {
        self.striped = true;
        self
    }

    pub fn show(
        self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) -> egui::InnerResponse<Option<&'a W>> {
        let mut selected_item = None;

        let mut resp = ui.vertical(|outer_ui| {
            let ListView {
                selected_item_id,
                reset_selection,
                with_search,
                title,
                hold_text,
                items,
                data,
                inner_margin,
                outer_margin,
                rounding,
                striped,
            } = self;

            let resp = outer_ui.scope(|ui| {
                let root_id = ui.auto_id_with("ListView");
                let search_id = root_id.with("search");
                let selected_id = root_id.with("selected");
                let hovered_id = root_id.with("hovered");

                if reset_selection {
                    ui.data_mut(|d| {
                        d.remove_temp::<Option<Id>>(selected_id);
                        d.remove_temp::<Option<Id>>(hovered_id);
                    });
                } else if let Some(id) = selected_item_id {
                    ui.data_mut(|d| {
                        d.insert_temp(selected_id, Some(id));
                    });
                }

                let mut search: String = ui.data_mut(|d| d.get_temp(search_id)).unwrap_or_default();
                let mut selected: Option<Id> =
                    ui.data_mut(|d| d.get_temp(selected_id)).unwrap_or_default();
                let old_selected = selected;
                let mut hovered: Option<Id> =
                    ui.data_mut(|d| d.get_temp(hovered_id)).unwrap_or_default();

                ui.horizontal_top(|ui| {
                    if let Some(title) = title {
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            ui.visuals_mut().button_frame = true;
                            ui.add(Label::new(RichText::new(title).strong()));
                        });
                    }
                    if with_search {
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            if !search.is_empty() {
                                ui.visuals_mut().button_frame = false;
                                if ui.button("✖").on_hover_text("Clear search text").clicked() {
                                    search.clear();
                                }
                            }

                            let mut search_text = TextEdit::singleline(&mut search);
                            if let Some(text) = hold_text {
                                search_text = search_text.hint_text(text);
                            }
                            ui.add(search_text);
                        });
                    }
                });

                ScrollArea::vertical()
                    .id_source(root_id.with("list"))
                    .hscroll(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("list view container")
                            .num_columns(1)
                            .striped(striped)
                            .show(ui, |ui| {
                                for item in items {
                                    let id = item.id(data);
                                    let checked = selected == Some(id);
                                    let hover = hovered == Some(id);

                                    if checked {
                                        selected_item = Some(item);
                                    }

                                    if search.is_empty() || item.on_search(&search, data) {
                                        let mut child_frame = egui::Frame::default()
                                            .inner_margin(inner_margin)
                                            .outer_margin(outer_margin)
                                            .rounding(rounding);
                                        if checked {
                                            item.style_clicked(&mut child_frame);
                                        } else if hover {
                                            item.style_hovered(&mut child_frame);
                                        } else {
                                            item.style_normal(&mut child_frame);
                                        }
                                        let mut interact_area = child_frame
                                            .show(ui, |ui| {
                                                item.show(checked, hover, ctx, ui, data);
                                                ui.interact(
                                                    ui.max_rect(),
                                                    item.id(data),
                                                    Sense::click(),
                                                )
                                            })
                                            .inner;
                                        if let Some(tips) = item.hovered_text() {
                                            interact_area = interact_area.on_hover_text(tips);
                                        }

                                        if interact_area.hovered() && hovered != Some(id) {
                                            hovered = Some(id);
                                        }

                                        if interact_area.clicked() && !checked {
                                            selected = Some(id);
                                            selected_item = Some(item);
                                        }

                                        ui.end_row();
                                    }
                                }
                            });
                    });

                if let Some(item) = selected_item {
                    item.selected_item(data);
                }

                ui.data_mut(|d| {
                    d.insert_temp(search_id, search);
                    d.insert_temp(selected_id, selected);
                    d.insert_temp(hovered_id, hovered);
                });

                old_selected != selected
            });

            resp.inner
        });

        if resp.inner {
            resp.response.mark_changed();
        }

        egui::InnerResponse::new(selected_item, resp.response)
    }
}
