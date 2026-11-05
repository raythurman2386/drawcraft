use crate::ids;
use serde::{Deserialize, Serialize};

/// Commands are either document edits or file/export operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Editor,
    Session,
}

/// UI grouping for the command palette and menus.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    File,
    Edit,
    View,
    Shape,
    Page,
    Tool,
}

impl Category {
    pub const ALL: [Self; 6] = [
        Self::File,
        Self::Edit,
        Self::View,
        Self::Shape,
        Self::Page,
        Self::Tool,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Edit => "edit",
            Self::View => "view",
            Self::Shape => "shape",
            Self::Page => "page",
            Self::Tool => "tool",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::View => "View",
            Self::Shape => "Shape",
            Self::Page => "Page",
            Self::Tool => "Tool",
        }
    }
}

/// The type of a command parameter, used for UI widgets and MCP schemas.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParamKind {
    String,
    Number,
    Integer,
    Boolean,
    Color,
    Enum(&'static [&'static str]),
    Array,
    Json,
}

impl ParamKind {
    #[must_use]
    pub fn json_type(self) -> &'static str {
        match self {
            Self::String | Self::Color | Self::Enum(_) => "string",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Json => "object",
        }
    }
}

/// One parameter accepted by a command.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ParamDef {
    pub name: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub required: bool,
    pub description: &'static str,
}

impl ParamDef {
    #[must_use]
    pub const fn required(name: &'static str, label: &'static str, kind: ParamKind) -> Self {
        Self {
            name,
            label,
            kind,
            required: true,
            description: "",
        }
    }

    #[must_use]
    pub const fn optional(name: &'static str, label: &'static str, kind: ParamKind) -> Self {
        Self {
            name,
            label,
            kind,
            required: false,
            description: "",
        }
    }

    #[must_use]
    pub const fn described(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }
}

/// Metadata for one user-visible action.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CommandDef {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub scope: Scope,
    pub params: &'static [ParamDef],
}

impl CommandDef {
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&'static ParamDef> {
        self.params.iter().find(|param| param.name == name)
    }
}

const NONE: &[ParamDef] = &[];

const P_PATH: &[ParamDef] = &[ParamDef::required("path", "File path", ParamKind::String)];
const P_CONFIRM: &[ParamDef] = &[ParamDef::optional(
    "confirm",
    "Continue despite unsaved changes (keeps recovery autosave)",
    ParamKind::Boolean,
)];
const P_OPEN: &[ParamDef] = &[
    ParamDef::required("path", "File path", ParamKind::String),
    ParamDef::optional(
        "confirm",
        "Continue despite unsaved changes (keeps recovery autosave)",
        ParamKind::Boolean,
    ),
];
const P_SAVE: &[ParamDef] = &[
    ParamDef::optional("path", "File path", ParamKind::String),
    ParamDef::optional(
        "force",
        "Overwrite despite external change",
        ParamKind::Boolean,
    ),
];
const P_SAVE_AS: &[ParamDef] = &[
    ParamDef::required("path", "File path", ParamKind::String),
    ParamDef::optional(
        "force",
        "Overwrite despite external change",
        ParamKind::Boolean,
    ),
];
const P_EXPORT_SVG: &[ParamDef] = &[
    ParamDef::required("path", "Output path", ParamKind::String),
    ParamDef::optional(
        "scope",
        "Export scope",
        ParamKind::Enum(&["page", "selection"]),
    ),
    ParamDef::optional("transparent", "Transparent background", ParamKind::Boolean),
];
const P_EXPORT_PNG: &[ParamDef] = &[
    ParamDef::required("path", "Output path", ParamKind::String),
    ParamDef::optional(
        "scope",
        "Export scope",
        ParamKind::Enum(&["page", "selection"]),
    ),
    ParamDef::optional("scale", "Pixel scale", ParamKind::Number),
    ParamDef::optional("transparent", "Transparent background", ParamKind::Boolean),
];
const P_AUTOSAVE_SET: &[ParamDef] = &[
    ParamDef::optional("enabled", "Enable autosave", ParamKind::Boolean),
    ParamDef::optional("interval_ms", "Autosave interval (ms)", ParamKind::Integer),
];
const RECOVER_ACTIONS: &[&str] = &["restore", "discard"];
const P_RECOVER: &[ParamDef] = &[
    ParamDef::required(
        "action",
        "Recovery action",
        ParamKind::Enum(RECOVER_ACTIONS),
    ),
    ParamDef::optional("autosave", "Autosave path", ParamKind::String),
];
const P_RELOAD: &[ParamDef] = &[ParamDef::optional(
    "force",
    "Discard dirty edits",
    ParamKind::Boolean,
)];

const AXIS: &[&str] = &["horizontal", "vertical"];
const ALIGN: &[&str] = &["left", "center", "right", "top", "middle", "bottom"];
const ZORDER: &[&str] = &["front", "back", "forward", "backward"];
const TOOLS: &[&str] = &[
    "select",
    "hand",
    "draw",
    "eraser",
    "arrow",
    "text",
    "sticky",
    "note",
    "image",
    "line",
    "polyline",
    "highlighter",
    "laser",
    "frame",
    "rectangle",
    "ellipse",
    "triangle",
    "diamond",
    "pentagon",
    "hexagon",
    "octagon",
    "star",
    "heart",
    "round_rect",
    "parallelogram",
    "trapezoid",
    "arrow_shape",
    "double_arrow",
    "cross",
    "home_plate",
    "chevron",
    "cylinder",
    "cloud",
];
const SHAPE_KINDS: &[&str] = &[
    "rectangle",
    "ellipse",
    "triangle",
    "diamond",
    "pentagon",
    "hexagon",
    "octagon",
    "star",
    "heart",
    "round_rect",
    "parallelogram",
    "trapezoid",
    "arrow_shape",
    "double_arrow",
    "cross",
    "home_plate",
    "chevron",
    "cylinder",
    "cloud",
    "note",
    "text",
    "sticky",
    "note",
    "image",
    "frame",
    "line",
    "polyline",
    "arrow",
    "draw",
    "highlighter",
];
const DASHES: &[&str] = &["solid", "dashed", "dotted", "hand_drawn", "none"];
const FILLS: &[&str] = &["none", "semi", "solid", "pattern", "full"];
const ARROW_STYLES: &[&str] = &["straight", "bent", "elbow"];
const ARROW_HEADS: &[&str] = &["none", "arrow", "triangle", "diamond", "square", "circle"];

const fn cmd(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    category: Category,
    scope: Scope,
    params: &'static [ParamDef],
) -> CommandDef {
    CommandDef {
        id,
        label,
        description,
        category,
        scope,
        params,
    }
}

/// The complete static command table.
#[must_use]
pub fn commands() -> &'static [CommandDef] {
    &COMMANDS
}

static COMMANDS: [CommandDef; 69] = [
    // ---- File & document -------------------------------------------------
    cmd(
        ids::FILE_NEW,
        "New document",
        "Start a blank page. Pass confirm=true when the current document is dirty; recovery autosave is kept.",
        Category::File,
        Scope::Session,
        P_CONFIRM,
    ),
    cmd(
        ids::FILE_OPEN,
        "Open document",
        "Open a .drawcraft file from disk. Pass confirm=true when the current document is dirty; recovery autosave is kept.",
        Category::File,
        Scope::Session,
        P_OPEN,
    ),
    cmd(
        ids::FILE_SAVE,
        "Save",
        "Save to the current path (or the supplied path).",
        Category::File,
        Scope::Session,
        P_SAVE,
    ),
    cmd(
        ids::FILE_SAVE_AS,
        "Save as",
        "Save the document to a new path.",
        Category::File,
        Scope::Session,
        P_SAVE_AS,
    ),
    cmd(
        ids::FILE_EXPORT_SVG,
        "Export SVG",
        "Export the page or selection as SVG.",
        Category::File,
        Scope::Session,
        P_EXPORT_SVG,
    ),
    cmd(
        ids::FILE_EXPORT_PNG,
        "Export PNG",
        "Rasterise the page or selection as PNG.",
        Category::File,
        Scope::Session,
        P_EXPORT_PNG,
    ),
    cmd(
        ids::DOC_RECENT,
        "Recent documents",
        "List recently opened document paths.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::DOC_RECENT_CLEAR,
        "Clear recent documents",
        "Remove every entry from the recent documents list.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::DOC_RECENT_REMOVE,
        "Remove recent document",
        "Remove one path from the recent documents list.",
        Category::File,
        Scope::Session,
        P_PATH,
    ),
    cmd(
        ids::DOC_INFO,
        "Document info",
        "Return path, dirty state, page count, and selection summary.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::SESSION_AUTOSAVE_STATUS,
        "Autosave status",
        "Report autosave enabled state, interval, path, and pending work.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::SESSION_AUTOSAVE_SET,
        "Configure autosave",
        "Enable or disable autosave and set the debounce interval.",
        Category::File,
        Scope::Session,
        P_AUTOSAVE_SET,
    ),
    cmd(
        ids::SESSION_AUTOSAVE_NOW,
        "Autosave now",
        "Write the current document to its autosave path synchronously.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::SESSION_RECOVER_LIST,
        "List recovery candidates",
        "List autosaves that are newer than and differ from their source files.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::SESSION_RECOVER,
        "Recover autosave",
        "Restore or discard a recovery candidate.",
        Category::File,
        Scope::Session,
        P_RECOVER,
    ),
    cmd(
        ids::SESSION_EXTERNAL_CHECK,
        "Check external change",
        "Report whether the on-disk file changed since the last open or save.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    cmd(
        ids::SESSION_RELOAD,
        "Reload from disk",
        "Reload the current path from disk, optionally discarding dirty edits.",
        Category::File,
        Scope::Session,
        P_RELOAD,
    ),
    cmd(
        ids::SESSION_KEEP_MINE,
        "Keep mine",
        "Accept the current disk fingerprint as the baseline and keep local edits.",
        Category::File,
        Scope::Session,
        NONE,
    ),
    // ---- History & clipboard --------------------------------------------
    cmd(
        ids::EDIT_UNDO,
        "Undo",
        "Undo the last document edit.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_REDO,
        "Redo",
        "Redo the last undone edit.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_COPY,
        "Copy",
        "Copy the selection to the internal clipboard.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_CUT,
        "Cut",
        "Copy and delete the selection.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_PASTE,
        "Paste",
        "Paste clipboard shapes with a small offset.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_DUPLICATE,
        "Duplicate",
        "Duplicate the selection with a small offset.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_DELETE,
        "Delete",
        "Delete the selection.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_SELECT,
        "Select shapes",
        "Select specific shape ids (groups expand). Additive extends after filtering nonexistent ids.",
        Category::Edit,
        Scope::Editor,
        &[
            ParamDef::optional("id", "Shape id", ParamKind::String),
            ParamDef::optional("ids", "Shape ids", ParamKind::Array),
            ParamDef::optional("additive", "Add to selection", ParamKind::Boolean),
        ],
    ),
    cmd(
        ids::EDIT_SELECT_ALL,
        "Select all",
        "Select every unlocked shape on the page.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_SELECT_NONE,
        "Clear selection",
        "Clear the current selection.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_SELECT_LASSO,
        "Lasso select",
        "Select shapes whose bounds are fully inside a polygon. Shift/additive toggles at group level.",
        Category::Edit,
        Scope::Editor,
        &[
            ParamDef::required("points", "Polygon points [[x,y],...]", ParamKind::Array),
            ParamDef::optional(
                "additive",
                "Toggle units against selection",
                ParamKind::Boolean,
            ),
        ],
    ),
    cmd(
        ids::EDIT_SELECT_FIRST,
        "Select first",
        "Select the bottommost unlocked shape by z-order (group expands).",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_SELECT_LAST,
        "Select last",
        "Select the topmost unlocked shape by z-order (group expands).",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    // ---- Transform ------------------------------------------------------
    cmd(
        ids::EDIT_MOVE,
        "Move selection",
        "Translate the selection by a world-space delta.",
        Category::Edit,
        Scope::Editor,
        &[
            ParamDef::required("dx", "Delta X", ParamKind::Number),
            ParamDef::required("dy", "Delta Y", ParamKind::Number),
        ],
    ),
    cmd(
        ids::EDIT_RESIZE,
        "Resize selection",
        "Scale the selection bounds to a new size.",
        Category::Edit,
        Scope::Editor,
        &[
            ParamDef::required("width", "Width", ParamKind::Number),
            ParamDef::required("height", "Height", ParamKind::Number),
        ],
    ),
    cmd(
        ids::EDIT_ROTATE,
        "Rotate selection",
        "Rotate the selection around its centre.",
        Category::Edit,
        Scope::Editor,
        &[ParamDef::required("degrees", "Degrees", ParamKind::Number)],
    ),
    cmd(
        ids::EDIT_FLIP,
        "Flip selection",
        "Flip the selection horizontally or vertically.",
        Category::Edit,
        Scope::Editor,
        &[ParamDef::required("axis", "Axis", ParamKind::Enum(AXIS))],
    ),
    cmd(
        ids::EDIT_ALIGN,
        "Align selection",
        "Align selected shapes to a shared edge or centre.",
        Category::Edit,
        Scope::Editor,
        &[ParamDef::required("edge", "Edge", ParamKind::Enum(ALIGN))],
    ),
    cmd(
        ids::EDIT_DISTRIBUTE,
        "Distribute selection",
        "Distribute selected shapes evenly along an axis.",
        Category::Edit,
        Scope::Editor,
        &[ParamDef::required("axis", "Axis", ParamKind::Enum(AXIS))],
    ),
    cmd(
        ids::EDIT_ZORDER,
        "Change z-order",
        "Move the selection forward, backward, to front, or to back.",
        Category::Edit,
        Scope::Editor,
        &[ParamDef::required(
            "action",
            "Action",
            ParamKind::Enum(ZORDER),
        )],
    ),
    cmd(
        ids::EDIT_LOCK,
        "Lock selection",
        "Lock or unlock the selection.",
        Category::Edit,
        Scope::Editor,
        &[ParamDef::optional("locked", "Locked", ParamKind::Boolean)],
    ),
    cmd(
        ids::EDIT_GROUP,
        "Group",
        "Group the selected shapes.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::EDIT_UNGROUP,
        "Ungroup",
        "Ungroup the selected shapes.",
        Category::Edit,
        Scope::Editor,
        NONE,
    ),
    // ---- Shapes ---------------------------------------------------------
    cmd(
        ids::SHAPE_ADD,
        "Add shape",
        "Create a shape at a world-space rectangle.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("kind", "Kind", ParamKind::Enum(SHAPE_KINDS)),
            ParamDef::optional("x", "X", ParamKind::Number),
            ParamDef::optional("y", "Y", ParamKind::Number),
            ParamDef::optional("width", "Width", ParamKind::Number),
            ParamDef::optional("height", "Height", ParamKind::Number),
            ParamDef::optional("text", "Text", ParamKind::String),
            ParamDef::optional("stroke", "Stroke color", ParamKind::Color),
            ParamDef::optional("fill", "Fill color", ParamKind::Color),
        ],
    ),
    cmd(
        ids::SHAPE_MOVE,
        "Move shape",
        "Translate one shape by a world-space delta.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::required("dx", "Delta X", ParamKind::Number),
            ParamDef::required("dy", "Delta Y", ParamKind::Number),
        ],
    ),
    cmd(
        ids::SHAPE_RESIZE,
        "Resize shape",
        "Resize one shape's bounding box.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::required("width", "Width", ParamKind::Number),
            ParamDef::required("height", "Height", ParamKind::Number),
        ],
    ),
    cmd(
        ids::SHAPE_ROTATE,
        "Rotate shape",
        "Rotate one shape around its centre.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::required("degrees", "Degrees", ParamKind::Number),
        ],
    ),
    cmd(
        ids::SHAPE_FLIP,
        "Flip shape",
        "Flip one shape horizontally or vertically.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::required("axis", "Axis", ParamKind::Enum(AXIS)),
        ],
    ),
    cmd(
        ids::SHAPE_SET_TEXT,
        "Set shape text",
        "Replace the text content of a shape.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::required("text", "Text", ParamKind::String),
        ],
    ),
    cmd(
        ids::SHAPE_SET_LABEL,
        "Set shape label",
        "Replace the label of a shape or arrow.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::required("label", "Label", ParamKind::String),
        ],
    ),
    cmd(
        ids::SHAPE_SET_STYLE,
        "Style shape",
        "Update stroke, fill, dash, opacity, or font properties.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::optional("stroke", "Stroke color", ParamKind::Color),
            ParamDef::optional("fill_color", "Fill color", ParamKind::Color),
            ParamDef::optional("stroke_width", "Stroke width", ParamKind::Number),
            ParamDef::optional("fill", "Fill", ParamKind::Enum(FILLS)),
            ParamDef::optional("dash", "Dash", ParamKind::Enum(DASHES)),
            ParamDef::optional("opacity", "Opacity", ParamKind::Number),
            ParamDef::optional("font_size", "Font size", ParamKind::Number),
        ],
    ),
    cmd(
        ids::SHAPE_DELETE,
        "Delete shape",
        "Delete one shape by id.",
        Category::Shape,
        Scope::Editor,
        &[ParamDef::required("id", "Shape id", ParamKind::String)],
    ),
    cmd(
        ids::SHAPE_LOCK,
        "Lock shape",
        "Lock or unlock one shape.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Shape id", ParamKind::String),
            ParamDef::optional("locked", "Locked", ParamKind::Boolean),
        ],
    ),
    cmd(
        ids::SHAPE_BIND,
        "Bind arrow",
        "Bind an arrow endpoint to a shape and optionally change routing or heads.",
        Category::Shape,
        Scope::Editor,
        &[
            ParamDef::required("id", "Arrow id", ParamKind::String),
            ParamDef::optional("start", "Start shape id", ParamKind::String),
            ParamDef::optional("end", "End shape id", ParamKind::String),
            ParamDef::optional("style", "Routing", ParamKind::Enum(ARROW_STYLES)),
            ParamDef::optional("start_head", "Start head", ParamKind::Enum(ARROW_HEADS)),
            ParamDef::optional("end_head", "End head", ParamKind::Enum(ARROW_HEADS)),
        ],
    ),
    // ---- Pages ----------------------------------------------------------
    cmd(
        ids::PAGE_ADD,
        "Add page",
        "Create a new page.",
        Category::Page,
        Scope::Editor,
        &[ParamDef::optional("name", "Name", ParamKind::String)],
    ),
    cmd(
        ids::PAGE_DELETE,
        "Delete page",
        "Delete a page (the last page cannot be deleted).",
        Category::Page,
        Scope::Editor,
        &[ParamDef::required("id", "Page id", ParamKind::String)],
    ),
    cmd(
        ids::PAGE_RENAME,
        "Rename page",
        "Rename a page.",
        Category::Page,
        Scope::Editor,
        &[
            ParamDef::required("id", "Page id", ParamKind::String),
            ParamDef::required("name", "Name", ParamKind::String),
        ],
    ),
    cmd(
        ids::PAGE_SET_ACTIVE,
        "Switch page",
        "Make a page active.",
        Category::Page,
        Scope::Editor,
        &[ParamDef::required("id", "Page id", ParamKind::String)],
    ),
    cmd(
        ids::PAGE_DUPLICATE,
        "Duplicate page",
        "Copy a page and all of its shapes.",
        Category::Page,
        Scope::Editor,
        &[
            ParamDef::required("id", "Page id", ParamKind::String),
            ParamDef::optional("name", "Name", ParamKind::String),
        ],
    ),
    cmd(
        ids::PAGE_MOVE,
        "Move page",
        "Move a page earlier or later in the page list.",
        Category::Page,
        Scope::Editor,
        &[
            ParamDef::required("id", "Page id", ParamKind::String),
            ParamDef::required("offset", "Offset", ParamKind::Integer),
        ],
    ),
    cmd(
        ids::PAGE_LIST,
        "List pages",
        "Return page ids, names, and the active page.",
        Category::Page,
        Scope::Editor,
        NONE,
    ),
    // ---- View -----------------------------------------------------------
    cmd(
        ids::VIEW_ZOOM_TO_FIT,
        "Zoom to fit",
        "Fit all content on the active page into the viewport.",
        Category::View,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::VIEW_ZOOM_TO_SELECTION,
        "Zoom to selection",
        "Fit the current selection into the viewport.",
        Category::View,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::VIEW_ZOOM_IN,
        "Zoom in",
        "Zoom in around the viewport centre.",
        Category::View,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::VIEW_ZOOM_OUT,
        "Zoom out",
        "Zoom out around the viewport centre.",
        Category::View,
        Scope::Editor,
        NONE,
    ),
    cmd(
        ids::VIEW_SET_ZOOM,
        "Set zoom",
        "Set an absolute zoom factor.",
        Category::View,
        Scope::Editor,
        &[ParamDef::required("zoom", "Zoom", ParamKind::Number)],
    ),
    cmd(
        ids::VIEW_SET_DARK,
        "Dark mode",
        "Toggle the dark canvas theme.",
        Category::View,
        Scope::Editor,
        &[ParamDef::required("dark", "Dark", ParamKind::Boolean)],
    ),
    cmd(
        ids::VIEW_SET_GRID,
        "Show grid",
        "Toggle the background grid.",
        Category::View,
        Scope::Editor,
        &[ParamDef::required("grid", "Grid", ParamKind::Boolean)],
    ),
    cmd(
        ids::VIEW_SET_FOCUS,
        "Focus mode",
        "Dim everything except the selection.",
        Category::View,
        Scope::Editor,
        &[ParamDef::required("focus", "Focus", ParamKind::Boolean)],
    ),
    cmd(
        ids::VIEW_SET_MINIMAP,
        "Minimap",
        "Toggle the minimap.",
        Category::View,
        Scope::Editor,
        &[ParamDef::required("minimap", "Minimap", ParamKind::Boolean)],
    ),
    // ---- Tools ----------------------------------------------------------
    cmd(
        ids::TOOL_SET,
        "Select tool",
        "Switch the active canvas tool.",
        Category::Tool,
        Scope::Editor,
        &[ParamDef::required("tool", "Tool", ParamKind::Enum(TOOLS))],
    ),
];

/// Query helper around the static command table.
#[derive(Copy, Clone, Debug, Default)]
pub struct Registry;

impl Registry {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn all(&self) -> &'static [CommandDef] {
        commands()
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&'static CommandDef> {
        self.all().iter().find(|command| command.id == id)
    }

    #[must_use]
    pub fn is_known(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &'static CommandDef> {
        self.all()
            .iter()
            .filter(move |command| command.category == category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for command in commands() {
            assert!(seen.insert(command.id), "duplicate command {}", command.id);
        }
    }

    #[test]
    fn every_command_has_a_label_and_description() {
        for command in commands() {
            assert!(!command.label.is_empty(), "{} has no label", command.id);
            assert!(
                !command.description.is_empty(),
                "{} has no description",
                command.id
            );
        }
    }

    #[test]
    fn required_params_are_unique_per_command() {
        for command in commands() {
            let mut names = HashSet::new();
            for param in command.params {
                assert!(
                    names.insert(param.name),
                    "{} repeats param {}",
                    command.id,
                    param.name
                );
            }
        }
    }
}
