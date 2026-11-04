//! DrawCraft command registry.
//!
//! Every user-visible action is described here exactly once. The UI, CLI,
//! localhost JSON channel, and MCP server all dispatch the same IDs, so an agent
//! can do anything a human can do.
//!
//! The registry is pure metadata; execution lives in `drawcraft-editor`
//! (document operations) and `drawcraft-session` (file and export operations).

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

mod registry;
pub use registry::{Category, CommandDef, ParamDef, ParamKind, Registry, Scope, commands};

/// Stable command identifiers.
///
/// Using constants avoids typos in the editor, session, UI, and tests.
pub mod ids {
    // File / document lifecycle (session scope).
    pub const FILE_NEW: &str = "file.new";
    pub const FILE_OPEN: &str = "file.open";
    pub const FILE_SAVE: &str = "file.save";
    pub const FILE_SAVE_AS: &str = "file.save_as";
    pub const FILE_EXPORT_SVG: &str = "file.export_svg";
    pub const FILE_EXPORT_PNG: &str = "file.export_png";
    pub const DOC_RECENT: &str = "doc.recent";
    pub const DOC_INFO: &str = "doc.info";

    // History / clipboard.
    pub const EDIT_UNDO: &str = "edit.undo";
    pub const EDIT_REDO: &str = "edit.redo";
    pub const EDIT_COPY: &str = "edit.copy";
    pub const EDIT_CUT: &str = "edit.cut";
    pub const EDIT_PASTE: &str = "edit.paste";
    pub const EDIT_DUPLICATE: &str = "edit.duplicate";
    pub const EDIT_DELETE: &str = "edit.delete";
    pub const EDIT_SELECT: &str = "edit.select";
    pub const EDIT_SELECT_ALL: &str = "edit.select_all";
    pub const EDIT_SELECT_NONE: &str = "edit.select_none";

    // Selection transforms.
    pub const EDIT_MOVE: &str = "edit.move";
    pub const EDIT_RESIZE: &str = "edit.resize";
    pub const EDIT_ROTATE: &str = "edit.rotate";
    pub const EDIT_FLIP: &str = "edit.flip";
    pub const EDIT_ALIGN: &str = "edit.align";
    pub const EDIT_DISTRIBUTE: &str = "edit.distribute";
    pub const EDIT_ZORDER: &str = "edit.zorder";
    pub const EDIT_LOCK: &str = "edit.lock";
    pub const EDIT_GROUP: &str = "edit.group";
    pub const EDIT_UNGROUP: &str = "edit.ungroup";

    // Shape mutation.
    pub const SHAPE_ADD: &str = "shape.add";
    pub const SHAPE_MOVE: &str = "shape.move";
    pub const SHAPE_RESIZE: &str = "shape.resize";
    pub const SHAPE_ROTATE: &str = "shape.rotate";
    pub const SHAPE_FLIP: &str = "shape.flip";
    pub const SHAPE_SET_TEXT: &str = "shape.set_text";
    pub const SHAPE_SET_LABEL: &str = "shape.set_label";
    pub const SHAPE_SET_STYLE: &str = "shape.set_style";
    pub const SHAPE_DELETE: &str = "shape.delete";
    pub const SHAPE_LOCK: &str = "shape.lock";
    pub const SHAPE_BIND: &str = "shape.bind";

    // Pages.
    pub const PAGE_ADD: &str = "page.add";
    pub const PAGE_DELETE: &str = "page.delete";
    pub const PAGE_RENAME: &str = "page.rename";
    pub const PAGE_SET_ACTIVE: &str = "page.set_active";
    pub const PAGE_DUPLICATE: &str = "page.duplicate";
    pub const PAGE_MOVE: &str = "page.move";
    pub const PAGE_LIST: &str = "page.list";

    // View.
    pub const VIEW_ZOOM_TO_FIT: &str = "view.zoom_to_fit";
    pub const VIEW_ZOOM_TO_SELECTION: &str = "view.zoom_to_selection";
    pub const VIEW_ZOOM_IN: &str = "view.zoom_in";
    pub const VIEW_ZOOM_OUT: &str = "view.zoom_out";
    pub const VIEW_SET_ZOOM: &str = "view.set_zoom";
    pub const VIEW_SET_DARK: &str = "view.set_dark";
    pub const VIEW_SET_GRID: &str = "view.set_grid";
    pub const VIEW_SET_FOCUS: &str = "view.set_focus";
    pub const VIEW_SET_MINIMAP: &str = "view.set_minimap";

    // Tools.
    pub const TOOL_SET: &str = "tool.set";
}
