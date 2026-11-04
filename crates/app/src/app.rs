use std::path::{Path, PathBuf};

use drawcraft_command::ids;
use drawcraft_editor::Tool;
use drawcraft_format::ExportScope;
use eframe::CreationContext;
use egui::Key;
use serde_json::{Value, json};

use crate::state::UiState;
use crate::theme;

/// The DrawCraft desktop application.
pub struct DrawCraftApp {
    pub(crate) session: drawcraft_session::Session,
    pub(crate) state: UiState,
    pub(crate) theme_dark: Option<bool>,
}

impl DrawCraftApp {
    #[must_use]
    pub fn new(cc: Option<&CreationContext<'_>>) -> Self {
        if let Some(cc) = cc {
            theme::apply(&cc.egui_ctx, false);
        }
        Self {
            session: drawcraft_session::Session::new(),
            state: UiState::new(),
            theme_dark: None,
        }
    }

    /// Open a document after construction.
    pub fn open_path(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        self.session
            .open(path)
            .map(|_| {
                self.state
                    .set_status(format!("Opened {}", self.session.display_name()));
            })
            .map_err(|error| {
                let message = error.to_string();
                self.state.last_error = Some(message.clone());
                message
            })
    }

    pub(crate) fn run(&mut self, id: &str, params: Value) {
        match self.session.execute(id, &params) {
            Ok(value) => {
                self.state.set_status(format!("{id}: {value}"));
                self.state.last_error = None;
            }
            Err(error) => {
                self.state.set_status(format!("error: {error}"));
                self.state.last_error = Some(error.to_string());
            }
        }
    }

    pub(crate) fn save_current(&mut self) {
        if self.session.current_path().is_some() {
            match self.session.save() {
                Ok(path) => {
                    self.state.set_status(format!("Saved {}", path.display()));
                    self.state.last_error = None;
                }
                Err(error) => self.state.last_error = Some(error.to_string()),
            }
        } else {
            self.save_as_dialog();
        }
    }

    pub(crate) fn save_as_dialog(&mut self) {
        let name = self.session.display_name();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("DrawCraft", &["drawcraft"])
            .set_file_name(&name)
            .save_file()
        else {
            return;
        };
        let path = ensure_extension(path, "drawcraft");
        match self.session.save_as(&path) {
            Ok(path) => {
                self.state.set_status(format!("Saved {}", path.display()));
                self.state.last_error = None;
            }
            Err(error) => self.state.last_error = Some(error.to_string()),
        }
    }

    pub(crate) fn open_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("DrawCraft", &["drawcraft"])
            .pick_file()
        else {
            return;
        };
        if let Err(error) = self.open_path(path) {
            self.state.last_error = Some(error);
        }
    }

    pub(crate) fn export_dialog(&mut self, format: &str) {
        let default_name = self
            .session
            .current_path()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .unwrap_or("drawcraft")
            .to_owned();
        let (filter_name, extension) = if format == "png" {
            ("PNG image", "png")
        } else {
            ("SVG image", "svg")
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, &[extension])
            .set_file_name(format!("{default_name}.{extension}"))
            .save_file()
        else {
            return;
        };
        let path = ensure_extension(path, extension);
        let scope = if self.session.selection_ids().is_empty() {
            ExportScope::Page
        } else {
            ExportScope::Selection
        };
        let result = if format == "png" {
            self.session.export_png(&path, scope, 1.0, false)
        } else {
            self.session.export_svg(&path, scope, false)
        };
        match result {
            Ok(()) => {
                self.state
                    .set_status(format!("Exported {}", path.display()));
                self.state.last_error = None;
            }
            Err(error) => self.state.last_error = Some(error.to_string()),
        }
    }

    pub(crate) fn set_tool(&mut self, tool: Tool) {
        self.run(ids::TOOL_SET, json!({"tool": tool.as_str()}));
    }

    pub(crate) fn toggle_dark(&mut self) {
        let dark = !self.session.editor.view.dark;
        self.run(ids::VIEW_SET_DARK, json!({"dark": dark}));
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.state.palette_open {
            if ctx.input(|input| input.key_pressed(Key::Escape)) {
                self.state.palette_open = false;
            }
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let command = ctx.input(|input| input.modifiers.command);
        let shift = ctx.input(|input| input.modifiers.shift);

        if command && ctx.input(|input| input.key_pressed(Key::Z)) {
            let id = if shift {
                ids::EDIT_REDO
            } else {
                ids::EDIT_UNDO
            };
            self.run(id, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::Y)) {
            self.run(ids::EDIT_REDO, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::S)) {
            self.save_current();
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::O)) {
            self.open_dialog();
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::N)) {
            self.run(ids::FILE_NEW, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::A)) {
            self.run(ids::EDIT_SELECT_ALL, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::C)) {
            self.run(ids::EDIT_COPY, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::X)) {
            self.run(ids::EDIT_CUT, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::V)) {
            self.run(ids::EDIT_PASTE, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::D)) {
            self.run(ids::EDIT_DUPLICATE, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::G)) {
            let id = if shift {
                ids::EDIT_UNGROUP
            } else {
                ids::EDIT_GROUP
            };
            self.run(id, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::Num0)) {
            self.run(ids::VIEW_ZOOM_TO_FIT, json!({}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::Num1)) {
            self.run(ids::VIEW_SET_ZOOM, json!({"zoom": 1.0}));
            return;
        }
        if command && ctx.input(|input| input.key_pressed(Key::K)) {
            self.state.palette_open = true;
            return;
        }
        if ctx.input(|input| input.key_pressed(Key::Delete)) {
            self.run(ids::EDIT_DELETE, json!({}));
            return;
        }
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            if self.state.drag.is_some() {
                self.state.drag = None;
            } else {
                self.run(ids::EDIT_SELECT_NONE, json!({}));
            }
            return;
        }
        if shift {
            let amount = 10.0;
            if ctx.input(|input| input.key_pressed(Key::ArrowLeft)) {
                self.run(ids::EDIT_MOVE, json!({"dx": -amount, "dy": 0.0}));
            } else if ctx.input(|input| input.key_pressed(Key::ArrowRight)) {
                self.run(ids::EDIT_MOVE, json!({"dx": amount, "dy": 0.0}));
            } else if ctx.input(|input| input.key_pressed(Key::ArrowUp)) {
                self.run(ids::EDIT_MOVE, json!({"dx": 0.0, "dy": -amount}));
            } else if ctx.input(|input| input.key_pressed(Key::ArrowDown)) {
                self.run(ids::EDIT_MOVE, json!({"dx": 0.0, "dy": amount}));
            }
        }

        // Tool shortcuts are plain keys so they do not conflict with Cmd/Ctrl.
        if command {
            return;
        }
        let key = ctx.input(|input| {
            [
                Key::V,
                Key::H,
                Key::D,
                Key::E,
                Key::A,
                Key::T,
                Key::S,
                Key::N,
                Key::I,
                Key::L,
                Key::P,
                Key::K,
                Key::J,
                Key::F,
                Key::R,
                Key::O,
            ]
            .into_iter()
            .find(|key| input.key_pressed(*key))
        });
        if let Some(key) = key {
            let tool = match key {
                Key::V => Some(Tool::Select),
                Key::H => Some(Tool::Hand),
                Key::D => Some(Tool::Draw),
                Key::E => Some(Tool::Eraser),
                Key::A => Some(Tool::Arrow),
                Key::T => Some(Tool::Text),
                Key::S => Some(Tool::Sticky),
                Key::N => Some(Tool::Note),
                Key::I => Some(Tool::Image),
                Key::L => Some(Tool::Line),
                Key::P => Some(Tool::Polyline),
                Key::K => Some(Tool::Highlighter),
                Key::J => Some(Tool::Laser),
                Key::F => Some(Tool::Frame),
                Key::R => Some(Tool::Rectangle),
                Key::O => Some(Tool::Ellipse),
                _ => None,
            };
            if let Some(tool) = tool {
                self.set_tool(tool);
            }
        }
    }

    fn show_text_editor(&mut self, ctx: &egui::Context) {
        let Some(mut editor) = self.state.text_edit.take() else {
            return;
        };
        if self.session.editor.doc.shape(&editor.id).is_none() {
            return;
        }
        let mut open = true;
        let mut apply = false;
        let mut close = false;
        egui::Window::new("Edit text")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::multiline(&mut editor.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(6),
                );
                if response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    if ui.input(|input| input.modifiers.command) {
                        apply = true;
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    ui.label("Cmd+Enter applies");
                });
            });
        if apply {
            let text = editor.text.clone();
            self.run(ids::SHAPE_SET_TEXT, json!({"id": editor.id, "text": text}));
        } else if open && !close {
            self.state.text_edit = Some(editor);
        }
    }
}

impl eframe::App for DrawCraftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::palette(self.session.editor.view.dark)
            .surface
            .to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let dark = self.session.editor.view.dark;
        if self.theme_dark != Some(dark) {
            theme::apply(ui.ctx(), dark);
            self.theme_dark = Some(dark);
        }
        self.handle_shortcuts(ui.ctx());

        // egui hands out space in call order: claim the top and bottom bars
        // before the side panels so both bars span the full window width.
        self.top_bar(ui);
        self.status_bar(ui);
        self.tool_rail(ui);
        self.right_panel(ui);
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas(ui);
        });
        self.show_text_editor(ui.ctx());
        self.show_command_palette(ui.ctx());
        self.show_help(ui.ctx());
        self.show_error(ui.ctx());
    }
}

fn ensure_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(extension);
    }
    path
}
