use drawcraft_command::{Registry, ids};
use drawcraft_document::{ArrowHead, ArrowStyle, Dash, Fill, Rect as DocRect, ShapeKind, Vec2};
use drawcraft_editor::{StyleUpdate, Tool};
use egui::{
    Align, Align2, Button, Color32, ComboBox, CornerRadius, Frame, Key, Layout, Margin, Response,
    RichText, Sense, Slider, Stroke, TextEdit,
};
use serde_json::json;

use crate::app::DrawCraftApp;
use crate::icons;
use crate::state::InlineTextEdit;
use crate::theme::{self, Palette};

impl DrawCraftApp {
    pub(crate) fn right_panel(&mut self, ui: &mut egui::Ui) {
        if !self.state.show_properties {
            return;
        }
        let p = theme::palette(self.session.editor.view.dark);
        egui::Panel::right("drawcraft-inspector")
            .resizable(true)
            .show_separator_line(false)
            .default_size(310.0)
            .min_size(250.0)
            .max_size(440.0)
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.highlight))
                    .inner_margin(Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                ui.painter().rect_filled(ui.max_rect(), 0.0, p.surface);
                let mut style = (**ui.style()).clone();
                style.visuals.extreme_bg_color = p.surface;
                style.visuals.widgets.inactive.bg_fill = p.surface;
                style.visuals.widgets.inactive.weak_bg_fill = p.surface;
                style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.muted);
                style.spacing.scroll.foreground_color = true;
                ui.set_style(style);
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.inspector_section(ui);
                        self.pages_section(ui);
                        if self.session.editor.view.minimap {
                            self.minimap_section(ui);
                        }
                        ui.add_space(18.0);
                    });
            });
    }

    fn inspector_section(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        section_label(ui, "INSPECTOR", p);

        let Some(selected) = self
            .session
            .editor
            .selected_shapes()
            .first()
            .copied()
            .cloned()
        else {
            card(ui, p, |ui| {
                let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover());
                icons::paint_tool_icon(ui.painter(), icon_rect, Tool::Select, p.accent);
                ui.label(
                    RichText::new("Nothing selected")
                        .size(12.5)
                        .strong()
                        .color(p.text),
                );
                ui.label(
                    RichText::new("Pick a tool on the left, or click a shape on the canvas.")
                        .size(11.0)
                        .color(p.muted),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} shapes on this page",
                        self.session.editor.doc.shapes().len()
                    ))
                    .size(10.5)
                    .color(p.muted),
                );
            });
            return;
        };

        let count = self.session.editor.selection.len();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(selected.kind.label())
                    .size(15.0)
                    .strong()
                    .color(p.text),
            );
            if count > 1 {
                ui.label(
                    RichText::new(format!("+{}", count - 1))
                        .size(10.5)
                        .strong()
                        .color(p.accent),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(selected.id.as_str())
                        .size(9.5)
                        .monospace()
                        .color(p.muted),
                );
            });
        });
        ui.add_space(6.0);

        let style = selected.style.clone();
        card(ui, p, |ui| {
            section_label_tight(ui, "APPEARANCE", p);
            self.color_row(ui, p, "Stroke", &style.stroke, |app, value| {
                app.apply_style(StyleUpdate {
                    stroke: Some(value),
                    ..StyleUpdate::default()
                });
            });
            self.color_row(ui, p, "Fill", &style.fill_color, |app, value| {
                app.apply_style(StyleUpdate {
                    fill_color: Some(value),
                    ..StyleUpdate::default()
                });
            });

            let mut width = style.stroke_width as f32;
            if ui
                .add(
                    Slider::new(&mut width, 0.0..=24.0)
                        .text(RichText::new("Stroke width").size(11.0).color(p.muted))
                        .fixed_decimals(0),
                )
                .changed()
            {
                self.apply_style(StyleUpdate {
                    stroke_width: Some(f64::from(width)),
                    ..StyleUpdate::default()
                });
            }

            ui.add_space(4.0);
            ui.label(RichText::new("Fill").size(10.0).color(p.muted));
            let mut fill = style.fill;
            let fill_options: Vec<(Fill, &'static str)> = Fill::ALL
                .iter()
                .map(|value| (*value, value.as_str()))
                .collect();
            if segmented(ui, &mut fill, &fill_options, p) {
                self.apply_style(StyleUpdate {
                    fill: Some(fill),
                    ..StyleUpdate::default()
                });
            }

            ui.add_space(4.0);
            ui.label(RichText::new("Stroke style").size(10.0).color(p.muted));
            let mut dash = style.dash;
            let dash_options: Vec<(Dash, &'static str)> = Dash::ALL
                .iter()
                .map(|value| (*value, value.as_str()))
                .collect();
            if segmented(ui, &mut dash, &dash_options, p) {
                self.apply_style(StyleUpdate {
                    dash: Some(dash),
                    ..StyleUpdate::default()
                });
            }

            let mut opacity = style.opacity as f32;
            if ui
                .add(
                    Slider::new(&mut opacity, 0.0..=1.0)
                        .text(RichText::new("Opacity").size(11.0).color(p.muted))
                        .fixed_decimals(2),
                )
                .changed()
            {
                self.apply_style(StyleUpdate {
                    opacity: Some(f64::from(opacity)),
                    ..StyleUpdate::default()
                });
            }

            let mut font_size = style.font_size as f32;
            if ui
                .add(
                    Slider::new(&mut font_size, 8.0..=96.0)
                        .text(RichText::new("Font size").size(11.0).color(p.muted))
                        .fixed_decimals(0),
                )
                .changed()
            {
                self.apply_style(StyleUpdate {
                    font_size: Some(f64::from(font_size)),
                    ..StyleUpdate::default()
                });
            }
        });

        section_label(ui, "ACTIONS", p);
        card(ui, p, |ui| {
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, p, "Edit text").clicked() {
                    self.state.inline_text_edit = Some(InlineTextEdit {
                        id: selected.id.clone(),
                        text: selected.display_text().to_owned(),
                    });
                }
                if action_button(ui, p, "Duplicate").clicked() {
                    self.run(ids::EDIT_DUPLICATE, json!({}));
                }
                if action_button(ui, p, if selected.locked { "Unlock" } else { "Lock" }).clicked() {
                    self.run(ids::EDIT_LOCK, json!({"locked": !selected.locked}));
                }
            });
        });

        if selected.kind == ShapeKind::Arrow {
            section_label(ui, "ARROW", p);
            card(ui, p, |ui| {
                let mut routing = selected.arrow_style();
                ComboBox::from_id_salt("arrow-routing")
                    .selected_text(routing.as_str())
                    .show_ui(ui, |ui| {
                        for value in ArrowStyle::ALL {
                            ui.selectable_value(&mut routing, value, value.as_str());
                        }
                    });
                let mut start_head = selected.start_head();
                let mut end_head = selected.end_head();
                ui.horizontal(|ui| {
                    ComboBox::from_id_salt("arrow-start-head")
                        .selected_text(format!("Start: {}", start_head.as_str()))
                        .show_ui(ui, |ui| {
                            for value in ArrowHead::ALL {
                                ui.selectable_value(&mut start_head, value, value.as_str());
                            }
                        });
                    ComboBox::from_id_salt("arrow-end-head")
                        .selected_text(format!("End: {}", end_head.as_str()))
                        .show_ui(ui, |ui| {
                            for value in ArrowHead::ALL {
                                ui.selectable_value(&mut end_head, value, value.as_str());
                            }
                        });
                });
                if routing != selected.arrow_style()
                    || start_head != selected.start_head()
                    || end_head != selected.end_head()
                {
                    self.run(
                        ids::SHAPE_BIND,
                        json!({
                            "id": selected.id.clone(),
                            "style": routing.as_str(),
                            "start_head": start_head.as_str(),
                            "end_head": end_head.as_str(),
                        }),
                    );
                }
                if selected.arrow_start.is_some() || selected.arrow_end.is_some() {
                    ui.label(RichText::new("Bound to shapes").size(10.0).color(p.success));
                }
            });
        }

        section_label(ui, "ARRANGE", p);
        card(ui, p, |ui| {
            ui.horizontal_wrapped(|ui| {
                if glyph_button(ui, p, "⤓", "Send to back").clicked() {
                    self.run(ids::EDIT_ZORDER, json!({"action": "back"}));
                }
                if glyph_button(ui, p, "↓", "Send backward").clicked() {
                    self.run(ids::EDIT_ZORDER, json!({"action": "backward"}));
                }
                if glyph_button(ui, p, "↑", "Bring forward").clicked() {
                    self.run(ids::EDIT_ZORDER, json!({"action": "forward"}));
                }
                if glyph_button(ui, p, "⤒", "Bring to front").clicked() {
                    self.run(ids::EDIT_ZORDER, json!({"action": "front"}));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if glyph_button(ui, p, "⇤", "Align left").clicked() {
                    self.run(ids::EDIT_ALIGN, json!({"edge": "left"}));
                }
                if glyph_button(ui, p, "↔", "Align centre").clicked() {
                    self.run(ids::EDIT_ALIGN, json!({"edge": "center"}));
                }
                if glyph_button(ui, p, "⇥", "Align right").clicked() {
                    self.run(ids::EDIT_ALIGN, json!({"edge": "right"}));
                }
                if glyph_button(ui, p, "⤒", "Align top").clicked() {
                    self.run(ids::EDIT_ALIGN, json!({"edge": "top"}));
                }
                if glyph_button(ui, p, "↕", "Align middle").clicked() {
                    self.run(ids::EDIT_ALIGN, json!({"edge": "middle"}));
                }
                if glyph_button(ui, p, "⤓", "Align bottom").clicked() {
                    self.run(ids::EDIT_ALIGN, json!({"edge": "bottom"}));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, p, "Distribute H").clicked() {
                    self.run(ids::EDIT_DISTRIBUTE, json!({"axis": "horizontal"}));
                }
                if action_button(ui, p, "Distribute V").clicked() {
                    self.run(ids::EDIT_DISTRIBUTE, json!({"axis": "vertical"}));
                }
                if action_button(ui, p, "Flip H").clicked() {
                    self.run(ids::EDIT_FLIP, json!({"axis": "horizontal"}));
                }
                if action_button(ui, p, "Flip V").clicked() {
                    self.run(ids::EDIT_FLIP, json!({"axis": "vertical"}));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, p, "Group").clicked() {
                    self.run(ids::EDIT_GROUP, json!({}));
                }
                if action_button(ui, p, "Ungroup").clicked() {
                    self.run(ids::EDIT_UNGROUP, json!({}));
                }
                if ui
                    .add(
                        Button::new(RichText::new("Delete").size(11.0).color(p.error)).frame(false),
                    )
                    .clicked()
                {
                    self.run(ids::EDIT_DELETE, json!({}));
                }
            });
        });
    }

    fn color_row(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        label: &str,
        current: &str,
        on_change: impl FnOnce(&mut Self, String),
    ) {
        let mut color = theme::solid(current);
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(11.0).color(p.muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let response = ui.color_edit_button_srgba(&mut color);
                if response.changed() {
                    on_change(self, color_to_hex(color));
                }
                ui.label(RichText::new(current).size(9.5).monospace().color(p.muted));
            });
        });
    }

    fn apply_style(&mut self, update: StyleUpdate) {
        if self.session.editor.selection.is_empty() {
            return;
        }
        self.session.editor.set_style_selection(&update);
        self.session.mark_dirty();
    }

    fn pages_section(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        section_label(ui, "PAGES", p);
        card(ui, p, |ui| {
            let active = self.session.editor.doc.active_page.clone();
            let pages: Vec<(drawcraft_document::PageId, String, usize)> = self
                .session
                .editor
                .doc
                .pages
                .iter()
                .map(|page| (page.id.clone(), page.name.clone(), page.shapes.len()))
                .collect();
            for (id, name, count) in pages {
                let selected = id == active;
                if page_row(ui, p, &name, count, selected).clicked() && !selected {
                    self.run(ids::PAGE_SET_ACTIVE, json!({"id": id}));
                }
            }

            if let Some(mut name) = self.state.page_rename.take() {
                ui.add_space(4.0);
                let mut apply = false;
                let mut cancel = false;
                ui.horizontal(|ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut name)
                            .hint_text("Page name")
                            .desired_width(150.0),
                    );
                    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        apply = true;
                    }
                    if ui.small_button("Save").clicked() {
                        apply = true;
                    }
                    if ui.small_button("Cancel").clicked()
                        || ui.input(|input| input.key_pressed(Key::Escape))
                    {
                        cancel = true;
                    }
                });
                if apply {
                    let id = self.session.editor.doc.active_page.clone();
                    self.run(ids::PAGE_RENAME, json!({"id": id, "name": name}));
                } else if !cancel {
                    self.state.page_rename = Some(name);
                }
            }

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, p, "+ Page").clicked() {
                    self.run(ids::PAGE_ADD, json!({"name": "Page"}));
                }
                if action_button(ui, p, "Rename").clicked() {
                    self.state.page_rename = self
                        .session
                        .editor
                        .doc
                        .active()
                        .map(|page| page.name.clone());
                }
                if action_button(ui, p, "Duplicate").clicked() {
                    let id = self.session.editor.doc.active_page.clone();
                    self.run(ids::PAGE_DUPLICATE, json!({"id": id}));
                }
                if action_button(ui, p, "Delete").clicked() {
                    let id = self.session.editor.doc.active_page.clone();
                    self.run(ids::PAGE_DELETE, json!({"id": id}));
                }
            });
        });
    }

    fn minimap_section(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        section_label(ui, "MINIMAP", p);
        card(ui, p, |ui| {
            let size = egui::vec2(ui.available_width().max(180.0), 112.0);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            let painter = ui.painter().with_clip_rect(rect);
            painter.rect_filled(rect, CornerRadius::same(6), p.terminal);

            let content = self
                .session
                .editor
                .doc
                .active()
                .and_then(|page| page.bounds())
                .unwrap_or_else(|| DocRect::from_xywh(0.0, 0.0, 800.0, 600.0));
            let content = content.expand(40.0);
            let scale = (rect.width() as f64 / content.width().max(1.0))
                .min(rect.height() as f64 / content.height().max(1.0));
            let offset = rect.center()
                - egui::vec2(
                    (content.center().x as f32) * scale as f32,
                    (content.center().y as f32) * scale as f32,
                );
            let to_map = |point: Vec2| {
                egui::pos2(
                    offset.x + point.x as f32 * scale as f32,
                    offset.y + point.y as f32 * scale as f32,
                )
            };

            let shapes: Vec<(DocRect, bool)> = self
                .session
                .editor
                .doc
                .active()
                .map(|page| {
                    page.shapes
                        .iter()
                        .map(|shape| {
                            (
                                shape.world_bounds(),
                                self.session.editor.selection.contains(&shape.id),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            for (bounds, selected) in shapes {
                let min = to_map(bounds.min);
                let max = to_map(bounds.max);
                painter.rect_filled(
                    egui::Rect::from_min_max(min, max),
                    0.5,
                    if selected {
                        p.accent
                    } else {
                        p.muted.gamma_multiply(0.55)
                    },
                );
            }

            let view = self
                .session
                .editor
                .camera
                .visible_world_rect(self.session.editor.viewport);
            let min = to_map(view.min);
            let max = to_map(view.max);
            painter.rect_stroke(
                egui::Rect::from_min_max(min, max),
                CornerRadius::same(2),
                Stroke::new(1.5, p.accent),
                egui::StrokeKind::Inside,
            );

            if let Some(pointer) = response.interact_pointer_pos() {
                if response.clicked() || response.dragged() {
                    let world = Vec2::new(
                        f64::from((pointer.x - offset.x) / scale as f32),
                        f64::from((pointer.y - offset.y) / scale as f32),
                    );
                    self.session.editor.camera.center = world;
                }
            }
        });
    }

    pub(crate) fn show_help(&mut self, ctx: &egui::Context) {
        if !self.state.show_help {
            return;
        }
        let p = theme::palette(self.session.editor.view.dark);
        let mut open = true;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Move faster on the canvas")
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                for (shortcut, action) in [
                    ("V / Q / H", "Select / lasso / hand"),
                    ("D / K / E", "Draw / highlighter / eraser"),
                    ("L / A", "Line / arrow"),
                    ("T / S / N", "Text / sticky / note"),
                    ("I / F", "Image / frame"),
                    ("Space + drag", "Pan canvas"),
                    ("Cmd/Ctrl + scroll", "Zoom"),
                    ("Cmd/Ctrl + Z", "Undo"),
                    ("Cmd/Ctrl + Shift + Z", "Redo"),
                    ("Cmd/Ctrl + C / V / D", "Copy / paste / duplicate"),
                    ("Cmd/Ctrl + A", "Select all"),
                    ("Cmd/Ctrl + G", "Group (Shift to ungroup)"),
                    ("Cmd/Ctrl + K", "Command palette"),
                    ("Delete", "Delete selection"),
                    ("Shift + arrows", "Nudge selection by 10"),
                    ("Escape", "Clear selection / cancel drag"),
                ] {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [158.0, 18.0],
                            egui::Label::new(
                                RichText::new(shortcut)
                                    .size(10.5)
                                    .monospace()
                                    .color(p.accent),
                            ),
                        );
                        ui.label(RichText::new(action).size(11.5).color(p.secondary));
                    });
                }
            });
        self.state.show_help = open;
    }

    pub(crate) fn show_command_palette(&mut self, ctx: &egui::Context) {
        if !self.state.palette_open {
            return;
        }
        let p = theme::palette(self.session.editor.view.dark);
        let mut open = true;
        let mut chosen = false;
        egui::Window::new("Command palette")
            .open(&mut open)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.add(
                    TextEdit::singleline(&mut self.state.palette_filter)
                        .hint_text("Filter commands…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                let filter = self.state.palette_filter.to_ascii_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .show(ui, |ui| {
                        for command in Registry::new().all() {
                            if !filter.is_empty()
                                && !command.id.to_ascii_lowercase().contains(&filter)
                                && !command.label.to_ascii_lowercase().contains(&filter)
                            {
                                continue;
                            }
                            let label = format!("{}   {}", command.label, command.id);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 25.0],
                                    Button::new(RichText::new(label).size(11.5).color(p.secondary))
                                        .frame(false),
                                )
                                .clicked()
                            {
                                let id = command.id;
                                self.run(id, json!({}));
                                chosen = true;
                            }
                        }
                    });
            });
        self.state.palette_open = open && !chosen;
    }

    pub(crate) fn show_error(&mut self, ctx: &egui::Context) {
        let Some(error) = self.state.last_error.clone() else {
            return;
        };
        let p = theme::palette(self.session.editor.view.dark);
        let mut open = true;
        let mut close = false;
        egui::Window::new("Something went wrong")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("DrawCraft could not complete that action")
                        .size(12.5)
                        .strong()
                        .color(p.error),
                );
                ui.label(RichText::new(error).size(11.0).color(p.secondary));
                if action_button(ui, p, "Dismiss").clicked() {
                    close = true;
                }
            });
        if !open || close {
            self.state.last_error = None;
        }
    }
}

/// One page in the Pages list: name on the left, shape count on the right.
fn page_row(ui: &mut egui::Ui, p: Palette, name: &str, count: usize, selected: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 25.0), Sense::click());
    let shapes = format!("{count} {}", if count == 1 { "shape" } else { "shapes" });
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            format!("{name}, {shapes}"),
        )
    });
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, CornerRadius::same(6), theme::with_alpha(p.accent, 30));
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(6), p.highlight);
    }
    let body = egui::TextStyle::Body.resolve(ui.style());
    let small = egui::TextStyle::Small.resolve(ui.style());
    let inner = rect.shrink2(egui::vec2(8.0, 0.0));
    painter.text(
        inner.left_center(),
        Align2::LEFT_CENTER,
        name,
        body,
        if selected { p.accent } else { p.secondary },
    );
    painter.text(
        inner.right_center(),
        Align2::RIGHT_CENTER,
        count.to_string(),
        small,
        p.muted,
    );
    response.on_hover_text(shapes)
}

fn section_label(ui: &mut egui::Ui, label: &str, p: Palette) {
    ui.add_space(12.0);
    ui.label(RichText::new(label).size(9.5).strong().color(p.muted));
    ui.add_space(3.0);
}

fn section_label_tight(ui: &mut egui::Ui, label: &str, p: Palette) {
    ui.label(RichText::new(label).size(9.5).strong().color(p.muted));
    ui.add_space(2.0);
}

fn card<R>(ui: &mut egui::Ui, p: Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    Frame::new()
        .fill(p.main)
        .stroke(Stroke::new(1.0, p.highlight))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, add)
        .inner
}

fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    values: &[(T, &'static str)],
    p: Palette,
) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for (value, label) in values {
            let selected = *current == *value;
            let text =
                RichText::new(*label)
                    .size(10.0)
                    .color(if selected { p.accent } else { p.muted });
            if ui.selectable_value(current, *value, text).clicked() {
                changed = true;
            }
        }
    });
    changed
}

fn action_button(ui: &mut egui::Ui, p: Palette, label: &str) -> Response {
    ui.add(
        Button::new(RichText::new(label).size(10.5).color(p.secondary))
            .frame(false)
            .min_size(egui::vec2(0.0, 24.0)),
    )
}

fn glyph_button(ui: &mut egui::Ui, p: Palette, glyph: &str, tooltip: &str) -> Response {
    ui.add(
        Button::new(RichText::new(glyph).size(14.0).color(p.secondary))
            .frame(false)
            .min_size(egui::vec2(26.0, 24.0)),
    )
    .on_hover_text(tooltip)
}

fn color_to_hex(color: Color32) -> String {
    if color.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.r(),
            color.g(),
            color.b(),
            color.a()
        )
    }
}
