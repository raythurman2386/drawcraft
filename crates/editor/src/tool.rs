use std::str::FromStr;

use drawcraft_document::ShapeKind;
use thiserror::Error;

/// A pointer tool.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Select,
    Hand,
    Draw,
    Eraser,
    Arrow,
    Text,
    Sticky,
    Note,
    Image,
    Line,
    Polyline,
    Highlighter,
    Laser,
    Frame,
    Rectangle,
    Ellipse,
    Triangle,
    Diamond,
    Pentagon,
    Hexagon,
    Octagon,
    Star,
    Heart,
    RoundRect,
    Parallelogram,
    Trapezoid,
    ArrowShape,
    DoubleArrow,
    Cross,
    HomePlate,
    Chevron,
    Cylinder,
    Cloud,
}

/// Error returned when parsing an unknown tool name.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("unknown tool: {0}")]
pub struct UnknownTool(pub String);

impl Tool {
    pub const ALL: [Self; 33] = [
        Self::Select,
        Self::Hand,
        Self::Draw,
        Self::Eraser,
        Self::Arrow,
        Self::Text,
        Self::Sticky,
        Self::Note,
        Self::Image,
        Self::Line,
        Self::Polyline,
        Self::Highlighter,
        Self::Laser,
        Self::Frame,
        Self::Rectangle,
        Self::Ellipse,
        Self::Triangle,
        Self::Diamond,
        Self::Pentagon,
        Self::Hexagon,
        Self::Octagon,
        Self::Star,
        Self::Heart,
        Self::RoundRect,
        Self::Parallelogram,
        Self::Trapezoid,
        Self::ArrowShape,
        Self::DoubleArrow,
        Self::Cross,
        Self::HomePlate,
        Self::Chevron,
        Self::Cylinder,
        Self::Cloud,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Hand => "hand",
            Self::Draw => "draw",
            Self::Eraser => "eraser",
            Self::Arrow => "arrow",
            Self::Text => "text",
            Self::Sticky => "sticky",
            Self::Note => "note",
            Self::Image => "image",
            Self::Line => "line",
            Self::Polyline => "polyline",
            Self::Highlighter => "highlighter",
            Self::Laser => "laser",
            Self::Frame => "frame",
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Triangle => "triangle",
            Self::Diamond => "diamond",
            Self::Pentagon => "pentagon",
            Self::Hexagon => "hexagon",
            Self::Octagon => "octagon",
            Self::Star => "star",
            Self::Heart => "heart",
            Self::RoundRect => "round_rect",
            Self::Parallelogram => "parallelogram",
            Self::Trapezoid => "trapezoid",
            Self::ArrowShape => "arrow_shape",
            Self::DoubleArrow => "double_arrow",
            Self::Cross => "cross",
            Self::HomePlate => "home_plate",
            Self::Chevron => "chevron",
            Self::Cylinder => "cylinder",
            Self::Cloud => "cloud",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "select" | "selection" | "pointer" => Self::Select,
            "hand" | "pan" => Self::Hand,
            "draw" | "pen" => Self::Draw,
            "eraser" => Self::Eraser,
            "arrow" => Self::Arrow,
            "text" => Self::Text,
            "sticky" => Self::Sticky,
            "note" => Self::Note,
            "image" => Self::Image,
            "line" => Self::Line,
            "polyline" => Self::Polyline,
            "highlighter" => Self::Highlighter,
            "laser" => Self::Laser,
            "frame" => Self::Frame,
            "rectangle" | "rect" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "triangle" => Self::Triangle,
            "diamond" => Self::Diamond,
            "pentagon" => Self::Pentagon,
            "hexagon" => Self::Hexagon,
            "octagon" => Self::Octagon,
            "star" => Self::Star,
            "heart" => Self::Heart,
            "round_rect" | "roundrect" => Self::RoundRect,
            "parallelogram" => Self::Parallelogram,
            "trapezoid" => Self::Trapezoid,
            "arrow_shape" | "arrowshape" => Self::ArrowShape,
            "double_arrow" | "doublearrow" => Self::DoubleArrow,
            "cross" => Self::Cross,
            "home_plate" | "homeplate" => Self::HomePlate,
            "chevron" => Self::Chevron,
            "cylinder" => Self::Cylinder,
            "cloud" => Self::Cloud,
            _ => return None,
        })
    }

    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Select => "↖",
            Self::Hand => "✥",
            Self::Draw => "✎",
            Self::Eraser => "⌫",
            Self::Arrow => "→",
            Self::Text => "T",
            Self::Sticky => "▤",
            Self::Note => "▥",
            Self::Image => "▧",
            Self::Line => "／",
            Self::Polyline => "⌁",
            Self::Highlighter => "▰",
            Self::Laser => "✦",
            Self::Frame => "▭",
            Self::Rectangle => "▢",
            Self::Ellipse => "◯",
            Self::Triangle => "△",
            Self::Diamond => "◇",
            Self::Pentagon => "⬠",
            Self::Hexagon => "⬡",
            Self::Octagon => "⬢",
            Self::Star => "★",
            Self::Heart => "♡",
            Self::RoundRect => "▣",
            Self::Parallelogram => "▱",
            Self::Trapezoid => "▰",
            Self::ArrowShape => "➤",
            Self::DoubleArrow => "⇄",
            Self::Cross => "✚",
            Self::HomePlate => "⌂",
            Self::Chevron => "❯",
            Self::Cylinder => "⬭",
            Self::Cloud => "☁",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Hand => "Hand",
            Self::Draw => "Draw",
            Self::Eraser => "Eraser",
            Self::Arrow => "Arrow",
            Self::Text => "Text",
            Self::Sticky => "Sticky",
            Self::Note => "Note",
            Self::Image => "Image",
            Self::Line => "Line",
            Self::Polyline => "Polyline",
            Self::Highlighter => "Highlighter",
            Self::Laser => "Laser",
            Self::Frame => "Frame",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Triangle => "Triangle",
            Self::Diamond => "Diamond",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Octagon => "Octagon",
            Self::Star => "Star",
            Self::Heart => "Heart",
            Self::RoundRect => "Rounded",
            Self::Parallelogram => "Parallelogram",
            Self::Trapezoid => "Trapezoid",
            Self::ArrowShape => "Arrow shape",
            Self::DoubleArrow => "Double arrow",
            Self::Cross => "Cross",
            Self::HomePlate => "Home plate",
            Self::Chevron => "Chevron",
            Self::Cylinder => "Cylinder",
            Self::Cloud => "Cloud",
        }
    }

    /// The shape kind created by this tool, if any.
    #[must_use]
    pub const fn shape_kind(self) -> Option<ShapeKind> {
        Some(match self {
            Self::Text => ShapeKind::Text,
            Self::Sticky => ShapeKind::Sticky,
            Self::Note => ShapeKind::Note,
            Self::Image => ShapeKind::Image,
            Self::Frame => ShapeKind::Frame,
            Self::Draw => ShapeKind::Draw,
            Self::Highlighter => ShapeKind::Highlighter,
            Self::Line => ShapeKind::Line,
            Self::Polyline => ShapeKind::Polyline,
            Self::Arrow => ShapeKind::Arrow,
            Self::Laser => ShapeKind::Laser,
            Self::Rectangle => ShapeKind::Rectangle,
            Self::Ellipse => ShapeKind::Ellipse,
            Self::Triangle => ShapeKind::Triangle,
            Self::Diamond => ShapeKind::Diamond,
            Self::Pentagon => ShapeKind::Pentagon,
            Self::Hexagon => ShapeKind::Hexagon,
            Self::Octagon => ShapeKind::Octagon,
            Self::Star => ShapeKind::Star,
            Self::Heart => ShapeKind::Heart,
            Self::RoundRect => ShapeKind::RoundRect,
            Self::Parallelogram => ShapeKind::Parallelogram,
            Self::Trapezoid => ShapeKind::Trapezoid,
            Self::ArrowShape => ShapeKind::ArrowShape,
            Self::DoubleArrow => ShapeKind::DoubleArrow,
            Self::Cross => ShapeKind::Cross,
            Self::HomePlate => ShapeKind::HomePlate,
            Self::Chevron => ShapeKind::Chevron,
            Self::Cylinder => ShapeKind::Cylinder,
            Self::Cloud => ShapeKind::Cloud,
            Self::Select | Self::Hand | Self::Eraser => return None,
        })
    }

    #[must_use]
    pub const fn is_shape_tool(self) -> bool {
        self.shape_kind().is_some()
    }

    #[must_use]
    pub const fn is_linear_tool(self) -> bool {
        matches!(
            self,
            Self::Draw
                | Self::Highlighter
                | Self::Line
                | Self::Polyline
                | Self::Arrow
                | Self::Laser
        )
    }

    /// Single-key shortcut shown in the UI.
    #[must_use]
    pub const fn shortcut(self) -> &'static str {
        match self {
            Self::Select => "V",
            Self::Hand => "H",
            Self::Draw => "D",
            Self::Eraser => "E",
            Self::Arrow => "A",
            Self::Text => "T",
            Self::Sticky => "S",
            Self::Note => "N",
            Self::Image => "I",
            Self::Line => "L",
            Self::Polyline => "P",
            Self::Highlighter => "K",
            Self::Laser => "J",
            Self::Frame => "F",
            Self::Rectangle => "R",
            Self::Ellipse => "O",
            Self::Triangle => "3",
            Self::Diamond => "4",
            Self::Pentagon => "5",
            Self::Hexagon => "6",
            Self::Octagon => "8",
            Self::Star => "7",
            Self::Heart => "9",
            Self::RoundRect => "U",
            Self::Parallelogram => "0",
            Self::Trapezoid => "-",
            Self::ArrowShape => "=",
            Self::DoubleArrow => "[",
            Self::Cross => "]",
            Self::HomePlate => ";",
            Self::Chevron => "'",
            Self::Cylinder => ",",
            Self::Cloud => ".",
        }
    }
}

impl FromStr for Tool {
    type Err = UnknownTool;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(value).ok_or_else(|| UnknownTool(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_round_trip() {
        for tool in Tool::ALL {
            assert_eq!(Tool::parse(tool.as_str()), Some(tool));
        }
    }

    #[test]
    fn shape_tools_map_to_kinds() {
        assert_eq!(Tool::Rectangle.shape_kind(), Some(ShapeKind::Rectangle));
        assert_eq!(Tool::Select.shape_kind(), None);
    }

    #[test]
    fn unknown_tool_is_rejected() {
        assert!("nope".parse::<Tool>().is_err());
    }
}
