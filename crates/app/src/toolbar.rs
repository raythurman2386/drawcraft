use drawcraft_command::ids;
use drawcraft_document::ShapeKind;
use drawcraft_editor::Tool;
use egui::{
    Align, Align2, Button, CornerRadius, FontId, Frame, Layout, Margin, Rect, Response, RichText,
    Sense, Stroke, pos2, vec2,
};
use serde_json::json;

use crate::app::DrawCraftApp;
use crate::icons;
use crate::theme;

impl DrawCraftApp {
    pub(crate) fn top_bar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        egui::Panel::top("drawcraft-top-bar")
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.highlight))
                    .inner_margin(Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    logo(ui, p);
                    ui.label(RichText::new("DrawCraft").size(14.0).strong().color(p.text));
                    if self.session.dirty {
                        ui.label(RichText::new("•").size(10.0).color(p.accent));
                    }
                    ui.add_space(8.0);

                    menu_file(self, ui);
                    menu_edit(self, ui);
                    menu_view(self, ui);
                    menu_help(self, ui);
                    self.shapes_menu(ui);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if text_button(
                            ui,
                            if self.session.editor.view.dark {
                                "Day"
                            } else {
                                "Night"
                            },
                            "Toggle GrokDay / GrokNight",
                            true,
                        )
                        .clicked()
                        {
                            self.toggle_dark();
                        }
                        if text_button(ui, "?", "Keyboard shortcuts", true).clicked() {
                            self.state.show_help = true;
                        }
                        ui.add_space(4.0);
                        Frame::new()
                            .fill(p.highlight)
                            .corner_radius(6)
                            .inner_margin(Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    if text_button(ui, "-", "Zoom out", true).clicked() {
                                        self.run(ids::VIEW_ZOOM_OUT, json!({}));
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "{:.0}%",
                                            self.session.editor.camera.zoom * 100.0
                                        ))
                                        .size(11.0)
                                        .monospace()
                                        .color(p.secondary),
                                    );
                                    if text_button(ui, "+", "Zoom in", true).clicked() {
                                        self.run(ids::VIEW_ZOOM_IN, json!({}));
                                    }
                                    if text_button(ui, "Fit", "Zoom to fit", true).clicked() {
                                        self.run(ids::VIEW_ZOOM_TO_FIT, json!({}));
                                    }
                                });
                            });
                        ui.add_space(2.0);
                        if text_button(
                            ui,
                            "Redo",
                            "Redo (Cmd/Ctrl+Shift+Z)",
                            self.session.editor.can_redo(),
                        )
                        .clicked()
                        {
                            self.run(ids::EDIT_REDO, json!({}));
                        }
                        if text_button(
                            ui,
                            "Undo",
                            "Undo (Cmd/Ctrl+Z)",
                            self.session.editor.can_undo(),
                        )
                        .clicked()
                        {
                            self.run(ids::EDIT_UNDO, json!({}));
                        }
                    });
                });
            });
    }

    pub(crate) fn tool_rail(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        egui::Panel::left("drawcraft-tools")
            .resizable(false)
            .show_separator_line(false)
            .default_size(188.0)
            .min_size(188.0)
            .max_size(188.0)
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.highlight))
                    .inner_margin(Margin::symmetric(10, 10)),
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
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.spacing_mut().item_spacing.x = 4.0;
                        section_label(ui, "CREATE", p.muted);
                        self.tool_button(ui, Tool::Select);
                        self.tool_button(ui, Tool::Hand);

                        section_label(ui, "DRAW", p.muted);
                        self.tool_button(ui, Tool::Draw);
                        self.tool_button(ui, Tool::Highlighter);
                        self.tool_button(ui, Tool::Eraser);
                        self.tool_button(ui, Tool::Laser);

                        section_label(ui, "CONNECT", p.muted);
                        self.tool_button(ui, Tool::Line);
                        self.tool_button(ui, Tool::Arrow);

                        section_label(ui, "CONTENT", p.muted);
                        for chunk in [
                            &[Tool::Text, Tool::Sticky][..],
                            &[Tool::Note, Tool::Image][..],
                            &[Tool::Frame][..],
                        ] {
                            ui.horizontal(|ui| {
                                let width = 77.0;
                                self.compact_tool_button(ui, chunk[0], width);
                                if let Some(tool) = chunk.get(1) {
                                    self.compact_tool_button(ui, *tool, width);
                                } else {
                                    ui.add_space(width);
                                }
                            });
                        }
                    });
            });
    }

    fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool) -> bool {
        let p = theme::palette(self.session.editor.view.dark);
        let selected = self.session.editor.tool == tool;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(160.0, 26.0), Sense::click());
        let hovered = response.hovered();
        let painter = ui.painter();
        if selected {
            painter.rect_filled(rect, CornerRadius::same(6), theme::with_alpha(p.accent, 30));
            painter.rect_filled(
                Rect::from_min_size(pos2(rect.left() + 1.0, rect.top() + 7.0), vec2(2.0, 16.0)),
                CornerRadius::same(1),
                p.accent,
            );
        } else if hovered {
            painter.rect_filled(rect, CornerRadius::same(6), p.highlight);
        }
        icons::paint_tool_icon(
            painter,
            Rect::from_min_size(
                pos2(rect.left() + 11.0, rect.center().y - 8.0),
                vec2(16.0, 16.0),
            ),
            tool,
            if selected { p.accent } else { p.secondary },
        );
        painter.text(
            pos2(rect.left() + 35.0, rect.center().y),
            Align2::LEFT_CENTER,
            tool.label(),
            FontId::proportional(11.5),
            if selected { p.accent } else { p.secondary },
        );
        painter.text(
            pos2(rect.right() - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            tool.shortcut(),
            FontId::monospace(9.5),
            p.muted,
        );
        let clicked = response.clicked();
        let _ = response.on_hover_text(format!("{} ({})", tool.label(), tool.shortcut()));
        if clicked {
            self.set_tool(tool);
        }
        clicked
    }

    fn compact_tool_button(&mut self, ui: &mut egui::Ui, tool: Tool, width: f32) {
        let p = theme::palette(self.session.editor.view.dark);
        let selected = self.session.editor.tool == tool;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 25.0), Sense::click());
        let hovered = response.hovered();
        let painter = ui.painter();
        if selected {
            painter.rect_filled(rect, CornerRadius::same(6), theme::with_alpha(p.accent, 30));
        } else if hovered {
            painter.rect_filled(rect, CornerRadius::same(6), p.highlight);
        }
        icons::paint_tool_icon(
            painter,
            Rect::from_min_size(
                pos2(rect.left() + 7.0, rect.center().y - 8.0),
                vec2(16.0, 16.0),
            ),
            tool,
            if selected { p.accent } else { p.secondary },
        );
        painter.text(
            pos2(rect.left() + 28.0, rect.center().y),
            Align2::LEFT_CENTER,
            tool.label(),
            FontId::proportional(10.5),
            if selected { p.accent } else { p.secondary },
        );
        let clicked = response.clicked();
        let _ = response.on_hover_text(format!("{} ({})", tool.label(), tool.shortcut()));
        if clicked {
            self.set_tool(tool);
        }
    }

    fn shapes_menu(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        ui.menu_button(
            RichText::new("Shapes…").size(12.0).color(p.secondary),
            |ui| {
                egui::ScrollArea::vertical()
                    .max_height(430.0)
                    .show(ui, |ui| {
                        let mut kinds = ShapeKind::GEOMETRIC.to_vec();
                        kinds.sort_by_key(|kind| kind.label());
                        for kind in kinds {
                            let Some(tool) = Tool::parse(kind.as_str()) else {
                                continue;
                            };
                            if self.tool_button(ui, tool) {
                                ui.close();
                            }
                        }
                    });
            },
        );
    }

    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        egui::Panel::bottom("drawcraft-status")
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.highlight))
                    .inner_margin(Margin::symmetric(12, 3)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("•").size(11.0).color(p.accent));
                    ui.label(
                        RichText::new(self.session.editor.tool.label())
                            .size(11.0)
                            .strong()
                            .color(p.secondary),
                    );
                    dot(ui, p.muted);
                    let selection = self.session.editor.selection.len();
                    ui.label(
                        RichText::new(if selection == 0 {
                            "Nothing selected".to_owned()
                        } else {
                            format!("{selection} selected")
                        })
                        .size(11.0)
                        .color(p.muted),
                    );
                    dot(ui, p.muted);
                    let page = self
                        .session
                        .editor
                        .doc
                        .active()
                        .map(|page| page.name.clone())
                        .unwrap_or_else(|| "No page".into());
                    ui.label(RichText::new(page).size(11.0).color(p.muted));
                    dot(ui, p.muted);
                    ui.label(RichText::new(&self.state.status).size(11.0).color(p.muted));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pointer = self.state.last_pointer_world;
                        ui.label(
                            RichText::new(format!(
                                "{} shapes   {:.0}%   {:.0}, {:.0}",
                                self.session.editor.doc.shapes().len(),
                                self.session.editor.camera.zoom * 100.0,
                                pointer.x,
                                pointer.y
                            ))
                            .size(10.5)
                            .monospace()
                            .color(p.muted),
                        );
                    });
                });
            });
    }
}

fn menu_file(app: &mut DrawCraftApp, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
        if ui.button("New").clicked() {
            app.run(ids::FILE_NEW, json!({}));
            ui.close();
        }
        if ui.button("Open…").clicked() {
            app.open_dialog();
            ui.close();
        }
        if ui.button("Save").clicked() {
            app.save_current();
            ui.close();
        }
        if ui.button("Save as…").clicked() {
            app.save_as_dialog();
            ui.close();
        }
        ui.separator();
        if ui.button("Export SVG…").clicked() {
            app.export_dialog("svg");
            ui.close();
        }
        if ui.button("Export PNG…").clicked() {
            app.export_dialog("png");
            ui.close();
        }
        ui.separator();
        ui.menu_button("Open recent", |ui| {
            let recent = app.session.recent().to_vec();
            if recent.is_empty() {
                ui.weak("No recent documents");
            }
            for path in recent {
                let label = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("document");
                if ui
                    .button(label)
                    .on_hover_text(path.display().to_string())
                    .clicked()
                {
                    if let Err(error) = app.open_path(&path) {
                        app.state.last_error = Some(error);
                    }
                    ui.close();
                }
            }
        });
    });
}

fn menu_edit(app: &mut DrawCraftApp, ui: &mut egui::Ui) {
    ui.menu_button("Edit", |ui| {
        if ui.button("Undo").clicked() {
            app.run(ids::EDIT_UNDO, json!({}));
            ui.close();
        }
        if ui.button("Redo").clicked() {
            app.run(ids::EDIT_REDO, json!({}));
            ui.close();
        }
        ui.separator();
        if ui.button("Duplicate").clicked() {
            app.run(ids::EDIT_DUPLICATE, json!({}));
            ui.close();
        }
        if ui.button("Delete").clicked() {
            app.run(ids::EDIT_DELETE, json!({}));
            ui.close();
        }
        if ui.button("Select all").clicked() {
            app.run(ids::EDIT_SELECT_ALL, json!({}));
            ui.close();
        }
        ui.separator();
        if ui.button("Group").clicked() {
            app.run(ids::EDIT_GROUP, json!({}));
            ui.close();
        }
        if ui.button("Ungroup").clicked() {
            app.run(ids::EDIT_UNGROUP, json!({}));
            ui.close();
        }
    });
}

fn menu_view(app: &mut DrawCraftApp, ui: &mut egui::Ui) {
    ui.menu_button("View", |ui| {
        if ui.button("Zoom to fit").clicked() {
            app.run(ids::VIEW_ZOOM_TO_FIT, json!({}));
            ui.close();
        }
        if ui.button("Zoom to selection").clicked() {
            app.run(ids::VIEW_ZOOM_TO_SELECTION, json!({}));
            ui.close();
        }
        if ui.button("Actual size").clicked() {
            app.run(ids::VIEW_SET_ZOOM, json!({"zoom": 1.0}));
            ui.close();
        }
        ui.separator();
        let mut dark = app.session.editor.view.dark;
        if ui.checkbox(&mut dark, "GrokNight").clicked() {
            app.toggle_dark();
            ui.close();
        }
        if ui
            .checkbox(&mut app.session.editor.view.grid, "Dot grid")
            .clicked()
        {
            let grid = app.session.editor.view.grid;
            app.run(ids::VIEW_SET_GRID, json!({"grid": grid}));
        }
        if ui
            .checkbox(&mut app.session.editor.view.focus, "Focus mode")
            .clicked()
        {
            let focus = app.session.editor.view.focus;
            app.run(ids::VIEW_SET_FOCUS, json!({"focus": focus}));
        }
        if ui
            .checkbox(&mut app.session.editor.view.minimap, "Minimap")
            .clicked()
        {
            let minimap = app.session.editor.view.minimap;
            app.run(ids::VIEW_SET_MINIMAP, json!({"minimap": minimap}));
        }
    });
}

fn menu_help(app: &mut DrawCraftApp, ui: &mut egui::Ui) {
    ui.menu_button("Help", |ui| {
        if ui.button("Keyboard shortcuts").clicked() {
            app.state.show_help = true;
            ui.close();
        }
        if ui.button("Command palette").clicked() {
            app.state.palette_open = true;
            ui.close();
        }
    });
}

fn section_label(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    ui.add_space(5.0);
    ui.label(RichText::new(label).size(9.5).strong().color(color));
    ui.add_space(2.0);
}

fn dot(ui: &mut egui::Ui, color: egui::Color32) {
    ui.label(RichText::new("·").size(11.0).color(color));
}

fn text_button(ui: &mut egui::Ui, label: &str, tooltip: &str, enabled: bool) -> Response {
    let button = Button::new(RichText::new(label).size(10.5))
        .frame(false)
        .min_size(egui::vec2(24.0, 26.0));
    ui.add_enabled(enabled, button).on_hover_text(tooltip)
}

fn logo(ui: &mut egui::Ui, p: theme::Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), p.accent);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "D",
        FontId::proportional(12.0),
        p.on_accent,
    );
}
