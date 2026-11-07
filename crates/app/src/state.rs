use std::collections::{BTreeMap, HashMap};

use drawcraft_document::{Rect, ShapeId, ShapeRecord, Vec2};
use drawcraft_editor::Tool;
use egui::{Pos2, TextureHandle, TouchId, TouchPhase};

use crate::lifecycle::{ExternalChangePrompt, LeaveDirtyPrompt, RecoveryPrompt};

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
        /// Applied angle this frame (after optional Shift snap), for the readout.
        angle: f64,
        original: Vec<ShapeRecord>,
    },
    /// Erasing shapes under the pointer.
    Erase { last: Vec2 },
}

/// Inline text editor for a shape, placed directly on the canvas.
#[derive(Clone, Debug)]
pub struct InlineTextEdit {
    pub id: ShapeId,
    pub text: String,
    /// Focus has been requested on the text field (done once, on the first
    /// frame after the editor opens).
    pub focus_requested: bool,
    /// The text field held keyboard focus at the end of the previous frame.
    pub had_focus: bool,
}

impl InlineTextEdit {
    /// Open an editor for `id`, seeded with its current text.
    pub fn new(id: ShapeId, text: String) -> Self {
        Self {
            id,
            text,
            focus_requested: false,
            had_focus: false,
        }
    }
}

/// Active touch contacts, tracked from egui events.
///
/// egui does not say which device drove the pointer (a pen and a finger are
/// both "the pointer"), so the canvas keeps contact positions to tell a
/// finger-driven drag from a pen-driven one.
#[derive(Default)]
pub struct Touches {
    active: BTreeMap<TouchId, Pos2>,
}

impl Touches {
    pub fn update(&mut self, id: TouchId, phase: TouchPhase, pos: Pos2) {
        match phase {
            TouchPhase::Start | TouchPhase::Move => {
                self.active.insert(id, pos);
            }
            TouchPhase::End | TouchPhase::Cancel => {
                self.active.remove(&id);
            }
        }
    }

    #[must_use]
    pub fn any(&self) -> bool {
        !self.active.is_empty()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.active.len()
    }

    /// True when `pos` is within a finger's width of a contact, which is how a
    /// touch-emulated pointer is recognised.
    #[must_use]
    pub fn contains_near(&self, pos: Pos2) -> bool {
        self.active
            .values()
            .any(|contact| contact.distance(pos) <= 2.0)
    }
}

/// Per-window UI state that is not part of the document.
#[derive(Default)]
pub struct UiState {
    pub drag: Option<DragState>,
    /// Pre-mutation document and complete history for the active edit gesture.
    /// Dropped on commit; consumed on cancellation, even if snapshots deduplicated.
    pub drag_checkpoint: Option<drawcraft_editor::EditCheckpoint>,
    /// Whether the current drag was started by a finger (as opposed to a pen
    /// or mouse). A pen stroke is never cancelled by a touch landing on the
    /// screen. This does not reject palm contacts at the hardware level.
    pub drag_from_touch: bool,
    /// Fingers currently on the touchscreen.
    pub touches: Touches,
    /// A finger is currently on the touchscreen.
    pub touch_active: bool,
    /// The current touch sequence pans/zooms instead of using the active tool.
    pub touch_navigating: bool,
    pub inline_text_edit: Option<InlineTextEdit>,
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
    pub recovery_prompt: Option<RecoveryPrompt>,
    pub external_change_prompt: Option<ExternalChangePrompt>,
    pub leave_dirty_prompt: Option<LeaveDirtyPrompt>,
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

    /// Update the touch gesture state and return whether the canvas should
    /// navigate (pan/zoom) instead of running the active tool.
    ///
    /// A pen is delivered as a normal pointer, so it never reaches this path.
    /// Finger input navigates unless "Draw with touch" is enabled. Once a
    /// sequence grows to two or more fingers it keeps navigating until every
    /// finger lifts, so lifting one finger from a pinch cannot leave a stray
    /// stroke behind.
    pub fn update_touch_mode(
        &mut self,
        any_touch: bool,
        multi_touch: bool,
        touch_draw: bool,
    ) -> bool {
        if !any_touch {
            self.touch_active = false;
            self.touch_navigating = false;
            return false;
        }
        if !self.touch_active {
            self.touch_navigating = !touch_draw;
        }
        self.touch_active = true;
        if multi_touch {
            self.touch_navigating = true;
        }
        self.touch_navigating
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }
}

#[cfg(test)]
mod tests {
    use egui::{TouchId, TouchPhase, pos2};

    use super::{Touches, UiState};

    #[test]
    fn touch_navigates_when_touch_drawing_is_off() {
        let mut state = UiState::new();
        assert!(state.update_touch_mode(true, false, false));
        assert!(state.touch_active);
    }

    #[test]
    fn touch_draws_when_enabled() {
        let mut state = UiState::new();
        assert!(!state.update_touch_mode(true, false, true));
        assert!(state.touch_active);
    }

    #[test]
    fn second_finger_switches_to_navigation_until_all_lift() {
        let mut state = UiState::new();
        assert!(!state.update_touch_mode(true, false, true));
        assert!(state.update_touch_mode(true, true, true), "pinch navigates");
        // One finger left: the sequence must keep navigating.
        assert!(state.update_touch_mode(true, false, true));
        // All fingers lifted: the next tap is a fresh sequence.
        assert!(!state.update_touch_mode(false, false, true));
        assert!(!state.touch_active);
        assert!(!state.update_touch_mode(true, false, true));
    }

    #[test]
    fn touches_track_contacts_by_id() {
        let mut touches = Touches::default();
        touches.update(TouchId(1), TouchPhase::Start, pos2(10.0, 10.0));
        touches.update(TouchId(2), TouchPhase::Start, pos2(200.0, 10.0));
        assert!(touches.any());
        assert_eq!(touches.count(), 2);

        touches.update(TouchId(1), TouchPhase::Move, pos2(12.0, 10.0));
        touches.update(TouchId(1), TouchPhase::End, pos2(12.0, 10.0));
        assert_eq!(touches.count(), 1);

        touches.update(TouchId(2), TouchPhase::Cancel, pos2(200.0, 10.0));
        assert!(!touches.any());
    }

    #[test]
    fn contains_near_only_matches_the_contact_under_the_pointer() {
        let mut touches = Touches::default();
        touches.update(TouchId(1), TouchPhase::Start, pos2(10.0, 10.0));
        // The emulated pointer sits on the contact.
        assert!(touches.contains_near(pos2(10.0, 10.0)));
        // A pen resting elsewhere on the screen is not that contact.
        assert!(!touches.contains_near(pos2(400.0, 300.0)));
    }
}
