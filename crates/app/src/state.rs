use std::collections::HashMap;

use drawcraft_document::{Rect, ShapeId, ShapeRecord, Vec2};
use drawcraft_editor::Tool;
use egui::TextureHandle;

/// Which resize handle the pointer is dragging.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResizeHandle {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl ResizeHandle {
    pub const ALL: [Self; 8] = [
        Self::NorthWest,
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
    ];

    #[must_use]
    pub const fn is_west(self) -> bool {
        matches!(self, Self::NorthWest | Self::West | Self::SouthWest)
    }

    #[must_use]
    pub const fn is_east(self) -> bool {
        matches!(self, Self::NorthEast | Self::East | Self::SouthEast)
    }

    #[must_use]
    pub const fn is_north(self) -> bool {
        matches!(self, Self::NorthWest | Self::North | Self::NorthEast)
    }

    #[must_use]
    pub const fn is_south(self) -> bool {
        matches!(self, Self::SouthWest | Self::South | Self::SouthEast)
    }

    /// Cursor hint for the handle.
    #[must_use]
    pub const fn cursor(self) -> egui::CursorIcon {
        match self {
            Self::NorthWest | Self::SouthEast => egui::CursorIcon::ResizeNwSe,
            Self::NorthEast | Self::SouthWest => egui::CursorIcon::ResizeNeSw,
            Self::North | Self::South => egui::CursorIcon::ResizeVertical,
            Self::East | Self::West => egui::CursorIcon::ResizeHorizontal,
        }
    }
}

/// In-progress pointer gesture.
#[derive(Clone, Debug)]
pub enum DragState {
    /// Moving the current selection.
    Move {
        start: Vec2,
        original: Vec<ShapeRecord>,
    },
    /// Rubber-band selection.
    Marquee {
        start: Vec2,
        current: Vec2,
        additive: bool,
    },
    /// Freehand lasso polygon selection.
    Lasso { points: Vec<Vec2>, additive: bool },
    /// Dragging out a new shape, frame, or text box.
    Create {
        tool: Tool,
        start: Vec2,
        current: Vec2,
    },
    /// Collecting freehand points.
    Freehand { tool: Tool, points: Vec<Vec2> },
    /// Scaling the selection.
    Resize {
        handle: ResizeHandle,
        start_bounds: Rect,
        pointer_start: Vec2,
        original: Vec<ShapeRecord>,
    },
    /// Rotating the selection.
    Rotate {
        pivot: Vec2,
        start_angle: f64,
        original: Vec<ShapeRecord>,
    },
    /// Erasing shapes under the pointer.
    Erase { last: Vec2 },
}

/// Floating text editor for a shape.
#[derive(Clone, Debug)]
pub struct TextEditor {
    pub id: ShapeId,
    pub text: String,
}

/// Per-window UI state that is not part of the document.
#[derive(Default)]
pub struct UiState {
    pub drag: Option<DragState>,
    pub text_edit: Option<TextEditor>,
    pub hover: Option<ShapeId>,
    pub last_pointer_world: Vec2,
    pub textures: HashMap<ShapeId, TextureHandle>,
    pub show_help: bool,
    pub show_properties: bool,
    pub status: String,
    pub palette_open: bool,
    pub page_rename: Option<String>,
    pub palette_filter: String,
    pub last_error: Option<String>,
}

impl UiState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            show_properties: true,
            status: "Ready".into(),
            ..Self::default()
        }
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }
}
