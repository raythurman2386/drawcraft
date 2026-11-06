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
    let icon = load_app_icon()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 620.0])
            .with_title("DrawCraft")
            .with_app_id("drawcraft")
            .with_icon(icon),
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

/// Decode the checked-in, aspect-square RGBA app icon for the native viewport.
///
/// Unlike a platform resource, this also applies to Wayland and X11 at runtime.
fn load_app_icon() -> Result<egui::IconData, Box<dyn std::error::Error>> {
    let image = image::load_from_memory_with_format(
        include_bytes!("../../../assets/logo/drawcraft-256.png"),
        image::ImageFormat::Png,
    )?
    .into_rgba8();
    let (width, height) = image.dimensions();
    Ok(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod icon_tests {
    #[test]
    fn embedded_window_icon_is_square_rgba() {
        let icon = super::load_app_icon().unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        assert!(
            icon.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] == 255)
        );
    }
}
