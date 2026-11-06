use drawcraft_command::ids;
use drawcraft_document::ShapeKind;
use drawcraft_editor::Tool;
use egui::{
    Align, Align2, Button, CornerRadius, FontId, Frame, Layout, Margin, Rect, Response, RichText,
    Sense, Stroke, TextStyle, WidgetInfo, WidgetType, pos2, vec2,
};
use serde_json::json;
use std::path::Path;

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
                    ui.label(
                        RichText::new("DrawCraft")
                            .text_style(TextStyle::Body)
                            .strong()
                            .color(p.text),
                    );
                    if self.session.dirty {
                        ui.label(
                            RichText::new("•")
                                .text_style(TextStyle::Body)
                                .color(p.accent),
                        )
                        .on_hover_text("Unsaved changes")
                        .widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Label, true, "Unsaved changes")
                        });
                    }
                    ui.add_space(8.0);

                    menu_file(self, ui);
                    menu_edit(self, ui);
                    menu_view(self, ui);
                    menu_help(self, ui);
                    ui.add_space(8.0);

                    // Right cluster. Children are added right to left, so the
                    // visual order is: Undo Redo [- 100% + Fit] ? Night.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // The cluster only gets the space left after the menus.
                        // Clip to it so it can never paint over them, even if
                        // the window is narrower than the cluster.
                        let clip = ui.max_rect().intersect(ui.clip_rect());
                        ui.set_clip_rect(clip);

                        let dark = self.session.editor.view.dark;
                        let (theme_label, theme_name) = if dark {
                            ("Day", "Switch to GrokDay (light theme)")
                        } else {
                            ("Night", "Switch to GrokNight (dark theme)")
                        };
                        if text_button(ui, theme_label, theme_name, true).clicked() {
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
                                // `horizontal` sizes the pill to its contents and
                                // inherits right-to-left, so add in reverse. A nested
                                // `with_layout(left_to_right)` here claimed all the
                                // leftover width and pushed Undo/Redo onto the menus.
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if text_button(ui, "Fit", "Zoom to fit", true).clicked() {
                                        self.run(ids::VIEW_ZOOM_TO_FIT, json!({}));
                                    }
                                    if text_button(ui, "+", "Zoom in", true).clicked() {
                                        self.run(ids::VIEW_ZOOM_IN, json!({}));
                                    }
                                    let zoom = self.session.editor.camera.zoom * 100.0;
                                    ui.label(
                                        RichText::new(format!("{zoom:>4.0}%"))
                                            .text_style(TextStyle::Monospace)
                                            .color(p.secondary),
                                    )
                                    .on_hover_text("Zoom level");
                                    if text_button(ui, "-", "Zoom out", true).clicked() {
                                        self.run(ids::VIEW_ZOOM_OUT, json!({}));
                                    }
                                });
                            });
                        ui.add_space(4.0);
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
            .default_size(252.0)
            .min_size(252.0)
            .max_size(252.0)
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
                        self.tool_button(ui, Tool::Lasso);
                        self.tool_button(ui, Tool::Hand);

                        section_label(ui, "DRAW", p.muted);
                        self.tool_button(ui, Tool::Draw);
                        self.tool_button(ui, Tool::Highlighter);
                        self.tool_button(ui, Tool::Eraser);
                        self.tool_button(ui, Tool::Laser);

                        section_label(ui, "CONNECT", p.muted);
                        self.tool_button(ui, Tool::Line);
                        self.tool_button(ui, Tool::Arrow);

                        section_label(ui, "SHAPES", p.muted);
                        {
                            let mut shape_tools = Vec::new();
                            for kind in ShapeKind::GEOMETRIC {
                                let Some(tool) = Tool::parse(kind.as_str()) else {
                                    continue;
                                };
                                if matches!(tool, Tool::Note) {
                                    continue;
                                }
                                shape_tools.push(tool);
                            }
                            let width = compact_tool_width(ui.available_width());
                            for chunk in shape_tools.chunks(2) {
                                ui.horizontal(|ui| {
                                    self.compact_tool_button(ui, chunk[0], width);
                                    if let Some(tool) = chunk.get(1) {
                                        self.compact_tool_button(ui, *tool, width);
                                    } else {
                                        ui.add_space(width);
                                    }
                                });
                            }
                        }

                        section_label(ui, "CONTENT", p.muted);
                        for chunk in [
                            &[Tool::Text, Tool::Sticky][..],
                            &[Tool::Note, Tool::Image][..],
                            &[Tool::Frame][..],
                        ] {
                            let width = compact_tool_width(ui.available_width());
                            ui.horizontal(|ui| {
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
        if let Ok(path) = std::env::var("DRAWCRAFT_GUI_PROOF") {
            let scale = ui.ctx().pixels_per_point();
            let rect = [
                rect.left() * scale,
                rect.top() * scale,
                rect.right() * scale,
                rect.bottom() * scale,
            ];
            let record = serde_json::json!({"label":tool.label(),"rect":rect,"scale":scale});
            let _ = std::fs::write(
                Path::new(&path).join(format!("tool-{}.json", tool.as_str())),
                record.to_string(),
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tool.label())
        });
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
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 27.0), Sense::click());
        let color = if selected { p.accent } else { p.secondary };
        if selected {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), theme::with_alpha(p.accent, 30));
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), p.highlight);
        }
        icons::paint_tool_icon(
            ui.painter(),
            Rect::from_min_size(
                pos2(rect.left() + 8.0, rect.center().y - 8.0),
                vec2(16.0, 16.0),
            ),
            tool,
            color,
        );
        // Clip a long label only at the rail boundary; this width fits every full name.
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + 28.0, rect.top()),
            pos2(rect.right() - 3.0, rect.bottom()),
        );
        ui.painter().with_clip_rect(text_rect).text(
            pos2(text_rect.left(), rect.center().y),
            Align2::LEFT_CENTER,
            tool.label(),
            FontId::proportional(10.5),
            color,
        );
        if let Ok(path) = std::env::var("DRAWCRAFT_GUI_PROOF") {
            let scale = ui.ctx().pixels_per_point();
            let rect = [
                rect.left() * scale,
                rect.top() * scale,
                rect.right() * scale,
                rect.bottom() * scale,
            ];
            let record = serde_json::json!({"label":tool.label(),"rect":rect,"scale":scale});
            let _ = std::fs::write(
                Path::new(&path).join(format!("tool-{}.json", tool.as_str())),
                record.to_string(),
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tool.label())
        });
        let clicked = response.clicked();
        let _ = response.on_hover_text(format!("{} ({})", tool.label(), tool.shortcut()));
        if clicked {
            self.set_tool(tool);
        }
    }

    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(self.session.editor.view.dark);
        // The inspector already shows the selection and the shape count, and the
        // top bar shows the zoom. Only repeat them here when the inspector is hidden.
        let inspector_visible = self.state.show_properties;
        egui::Panel::bottom("drawcraft-status")
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.highlight))
                    .inner_margin(Margin::symmetric(12, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(self.session.editor.tool.label())
                            .text_style(TextStyle::Body)
                            .strong()
                            .color(p.secondary),
                    )
                    .on_hover_text("Active tool");
                    if !inspector_visible {
                        dot(ui, p.muted);
                        let selection = self.session.editor.selection.len();
                        status_text(
                            ui,
                            p,
                            if selection == 0 {
                                "Nothing selected".to_owned()
                            } else {
                                format!("{selection} selected")
                            },
                        );
                    }
                    dot(ui, p.muted);
                    let page = self
                        .session
                        .editor
                        .doc
                        .active()
                        .map(|page| page.name.clone())
                        .unwrap_or_else(|| "No page".into());
                    status_text(ui, p, page);
                    dot(ui, p.muted);
                    status_text(ui, p, self.state.status.clone());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pointer = self.state.last_pointer_world;
                        // Fixed-width slots so the row does not jitter as the pointer moves.
                        ui.label(
                            RichText::new(format!("x {:>6.0}  y {:>6.0}", pointer.x, pointer.y))
                                .text_style(TextStyle::Monospace)
                                .color(p.muted),
                        )
                        .on_hover_text("Pointer position (canvas units)");
                        if !inspector_visible {
                            let shapes = self.session.editor.doc.shapes().len();
                            status_text(
                                ui,
                                p,
                                format!(
                                    "{shapes} {}",
                                    if shapes == 1 { "shape" } else { "shapes" }
                                ),
                            );
                        }
                    });
                });
            });
    }
}

fn menu_file(app: &mut DrawCraftApp, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
        if ui.button("New").clicked() {
            app.request_new_document();
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
        let mut autosave_enabled = app
            .session
            .autosave_status()
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        if ui.checkbox(&mut autosave_enabled, "Autosave").changed() {
            app.run(
                ids::SESSION_AUTOSAVE_SET,
                json!({"enabled": autosave_enabled}),
            );
        }
        ui.menu_button("Open recent", |ui| {
            let entries = app.session.recent_entries();
            if entries.is_empty() {
                ui.weak("No recent documents");
            }
            for (path, exists) in entries {
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("document");
                let label = if exists {
                    file_name.to_owned()
                } else {
                    format!("{file_name} (missing)")
                };
                ui.horizontal(|ui| {
                    let response = if exists {
                        ui.button(label).on_hover_text(path.display().to_string())
                    } else {
                        ui.weak(label)
                            .on_hover_text(format!("{} — file is missing", path.display()))
                    };
                    if response.clicked() {
                        if exists {
                            if let Err(error) = app.open_path(&path) {
                                app.state.last_error = Some(error);
                            }
                        } else {
                            app.state.last_error = Some(format!(
                                "Recent file is missing: {}\nUse Remove to clear it.",
                                path.display()
                            ));
                        }
                        ui.close();
                    }
                    if !exists && ui.small_button("Remove").clicked() {
                        app.run(
                            ids::DOC_RECENT_REMOVE,
                            json!({"path": path.display().to_string()}),
                        );
                    }
                });
            }
            if !app.session.recent().is_empty() {
                ui.separator();
                if ui.button("Clear recent").clicked() {
                    app.run(ids::DOC_RECENT_CLEAR, json!({}));
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
    ui.label(RichText::new("·").text_style(TextStyle::Body).color(color));
}

fn status_text(ui: &mut egui::Ui, p: theme::Palette, text: String) {
    ui.label(
        RichText::new(text)
            .text_style(TextStyle::Body)
            .color(p.muted),
    );
}

/// A frameless chrome button. `tooltip` doubles as the accessible name, so
/// glyph-only labels such as "?" or "-" are still announced meaningfully.
fn text_button(ui: &mut egui::Ui, label: &str, tooltip: &str, enabled: bool) -> Response {
    let button = Button::new(RichText::new(label).text_style(TextStyle::Button))
        .frame(false)
        .min_size(egui::vec2(24.0, 26.0));
    let response = ui.add_enabled(enabled, button);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, tooltip));
    response.on_hover_text(tooltip)
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

const fn compact_tool_width(available_width: f32) -> f32 {
    const COLUMN_GAP: f32 = 4.0;
    (available_width - COLUMN_GAP) / 2.0
}

#[cfg(test)]
mod tests {
    use super::compact_tool_width;

    #[test]
    fn expanded_rail_gives_each_tool_a_complete_label_area() {
        let width = compact_tool_width(252.0 - 20.0);
        assert_eq!(width, 114.0);
        assert!(width > 1.0);
        assert_eq!(width - 31.0, 83.0);
    }
}
