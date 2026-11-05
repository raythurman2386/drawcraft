//! DrawCraft desktop application.
//!
//! The UI is a thin egui/eframe layer. All editing and persistence live in the
//! headless crates, so the same commands are available to the CLI, JSON channel,
//! and MCP server.

// UI code nests conditionals around egui responses; splitting every one into
// guard clauses makes the drawing code harder to follow.
#![allow(clippy::collapsible_if, clippy::too_many_arguments)]

pub mod app;
pub mod canvas;
pub mod icons;
pub mod inline_edit;
pub mod lifecycle;
pub mod panels;
pub mod state;
pub mod theme;
pub mod toolbar;
pub mod transform;

pub use app::DrawCraftApp;
pub use state::UiState;

use std::path::PathBuf;

/// Launch the native desktop application.
///
/// Returns an error only when the window or renderer cannot be created; document
/// load errors are surfaced in the app status bar.
pub fn run_native(path: Option<PathBuf>, dark: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = DrawCraftApp::new(None);
    app.session.editor.view.dark = dark;
    if let Some(path) = path {
        if let Err(error) = app.open_path(path) {
            app.state.last_error = Some(error);
        }
    }
    app.check_recovery();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 620.0])
            .with_title("DrawCraft"),
        ..Default::default()
    };
    eframe::run_native(
        "DrawCraft",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, dark);
            app.theme_dark = Some(dark);
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}
