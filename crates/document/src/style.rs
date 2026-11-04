use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Stroke dash pattern.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dash {
    /// No visible stroke.
    None,
    #[default]
    Solid,
    Dashed,
    Dotted,
    /// Original deterministic "marker" wobble.
    HandDrawn,
}

impl Dash {
    pub const ALL: [Self; 5] = [
        Self::Solid,
        Self::Dashed,
        Self::Dotted,
        Self::HandDrawn,
        Self::None,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Solid => "solid",
            Self::Dashed => "dashed",
            Self::Dotted => "dotted",
            Self::HandDrawn => "hand_drawn",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "solid" => Some(Self::Solid),
            "dashed" => Some(Self::Dashed),
            "dotted" => Some(Self::Dotted),
            "hand_drawn" | "handdrawn" => Some(Self::HandDrawn),
            _ => None,
        }
    }

    /// SVG `stroke-dasharray` value, if any.
    #[must_use]
    pub fn dash_array(self, width: f64) -> Option<String> {
        match self {
            Self::Solid | Self::None | Self::HandDrawn => None,
            Self::Dashed => Some(format!("{:.2} {:.2}", width * 4.0, width * 3.0)),
            Self::Dotted => Some(format!("0 {:.2}", (width * 2.5).max(1.5))),
        }
    }
}

/// Fill treatment for closed shapes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fill {
    None,
    #[default]
    Semi,
    Solid,
    Pattern,
    Full,
}

impl Fill {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Semi,
        Self::Solid,
        Self::Pattern,
        Self::Full,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Semi => "semi",
            Self::Solid => "solid",
            Self::Pattern => "pattern",
            Self::Full => "full",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "semi" => Some(Self::Semi),
            "solid" => Some(Self::Solid),
            "pattern" => Some(Self::Pattern),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    /// Alpha multiplier applied to the fill color.
    #[must_use]
    pub const fn alpha(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Semi => 0.35,
            Self::Solid => 0.8,
            Self::Pattern => 0.25,
            Self::Full => 1.0,
        }
    }
}

/// Visual style shared by all shape kinds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    /// Stroke and text color.
    pub stroke: String,
    /// Fill color.
    pub fill_color: String,
    pub stroke_width: f64,
    pub fill: Fill,
    pub dash: Dash,
    /// Item opacity in `0.0..=1.0`.
    pub opacity: f64,
    pub font_size: f64,
    pub font_family: String,
    /// Unknown style keys are preserved round-trip.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Style {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_stroke(mut self, color: impl Into<String>) -> Self {
        self.stroke = color.into();
        self
    }

    #[must_use]
    pub fn with_fill(mut self, color: impl Into<String>, fill: Fill) -> Self {
        self.fill_color = color.into();
        self.fill = fill;
        self
    }

    #[must_use]
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    #[must_use]
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Validate limits that keep rendering and exports bounded.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err("opacity must be between 0 and 1".into());
        }
        if !self.stroke_width.is_finite() || !(0.0..=100.0).contains(&self.stroke_width) {
            return Err("stroke width must be between 0 and 100".into());
        }
        if !self.font_size.is_finite() || !(1.0..=512.0).contains(&self.font_size) {
            return Err("font size must be between 1 and 512".into());
        }
        Ok(())
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            stroke: "#262626".into(),
            fill_color: "#d9c8f5".into(),
            stroke_width: 2.0,
            fill: Fill::Semi,
            dash: Dash::Solid,
            opacity: 1.0,
            font_size: 18.0,
            font_family: "sans".into(),
            extra: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_round_trip_preserves_unknown_keys() {
        let raw = r##"{"stroke":"#000000","custom":"kept","opacity":0.5}"##;
        let style: Style = serde_json::from_str(raw).unwrap_or_default();
        assert_eq!(style.opacity, 0.5);
        assert_eq!(style.extra.get("custom"), Some(&serde_json::json!("kept")));
        let json = serde_json::to_value(&style).unwrap_or_default();
        assert_eq!(json.get("custom"), Some(&serde_json::json!("kept")));
    }

    #[test]
    fn rejects_invalid_opacity() {
        let mut style = Style::default();
        style.opacity = f64::NAN;
        assert!(style.validate().is_err());
    }

    #[test]
    fn dash_arrays() {
        assert!(Dash::Dashed.dash_array(2.0).is_some());
        assert!(Dash::Solid.dash_array(2.0).is_none());
    }
}
