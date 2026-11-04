use serde::{Deserialize, Serialize};

/// An sRGB color with straight alpha.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0x1e, 0x1e, 0x2e);
    pub const WHITE: Self = Self::rgb(0xff, 0xff, 0xff);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self { a: alpha, ..self }
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// CSS `rgb()` / `rgba()` notation, used by the SVG exporter.
    #[must_use]
    pub fn to_css(self) -> String {
        if self.a == 255 {
            format!("rgb({},{},{})", self.r, self.g, self.b)
        } else {
            let alpha = f64::from(self.a) / 255.0;
            format!("rgba({},{},{},{alpha:.3})", self.r, self.g, self.b)
        }
    }

    #[must_use]
    pub fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Parse a CSS-ish color string.
///
/// Supports `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb(r,g,b)`,
/// `rgba(r,g,b,a)`, and a small set of common names. Returns `None` for
/// unrecognised input; callers should fall back to a default rather than
/// rejecting a whole document for one bad color.
#[must_use]
pub fn parse_color(input: &str) -> Option<Color> {
    let value = input.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex(hex);
    }

    if let Some(inner) = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() < 3 {
            return None;
        }
        let r = parts.first()?.parse::<u8>().ok()?;
        let g = parts.get(1)?.parse::<u8>().ok()?;
        let b = parts.get(2)?.parse::<u8>().ok()?;
        let a = match parts.get(3) {
            Some(raw) => {
                let alpha = raw.parse::<f64>().ok()?;
                (alpha.clamp(0.0, 1.0) * 255.0).round() as u8
            }
            None => 255,
        };
        return Some(Color::rgba(r, g, b, a));
    }

    let named = match value.to_ascii_lowercase().as_str() {
        "black" => Color::rgb(0, 0, 0),
        "white" => Color::WHITE,
        "red" => Color::rgb(0xe7, 0x4c, 0x3c),
        "orange" => Color::rgb(0xf2, 0x8b, 0x30),
        "yellow" => Color::rgb(0xf5, 0xc2, 0x42),
        "green" => Color::rgb(0x4c, 0xaf, 0x50),
        "blue" => Color::rgb(0x42, 0x85, 0xf4),
        "purple" => Color::rgb(0x8e, 0x44, 0xad),
        "pink" => Color::rgb(0xe9, 0x1e, 0x63),
        "gray" | "grey" => Color::rgb(0x90, 0x90, 0x90),
        "transparent" => Color::TRANSPARENT,
        _ => return None,
    };
    Some(named)
}

fn parse_hex(hex: &str) -> Option<Color> {
    let digits: Vec<u8> = hex
        .chars()
        .map(|ch| ch.to_digit(16).map(|digit| digit as u8))
        .collect::<Option<Vec<u8>>>()?;
    match digits.len() {
        3 | 4 => {
            let r = digits[0] * 17;
            let g = digits[1] * 17;
            let b = digits[2] * 17;
            let a = digits.get(3).map_or(255, |value| value * 17);
            Some(Color::rgba(r, g, b, a))
        }
        6 | 8 => {
            let r = digits[0] * 16 + digits[1];
            let g = digits[2] * 16 + digits[3];
            let b = digits[4] * 16 + digits[5];
            let a = if digits.len() == 8 {
                digits[6] * 16 + digits[7]
            } else {
                255
            };
            Some(Color::rgba(r, g, b, a))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_forms() {
        assert_eq!(parse_color("#fff"), Some(Color::WHITE));
        assert_eq!(parse_color("#000000"), Some(Color::rgb(0, 0, 0)));
        assert_eq!(parse_color("#ff000080"), Some(Color::rgba(255, 0, 0, 128)));
    }

    #[test]
    fn parses_css_forms() {
        assert_eq!(
            parse_color("rgba(10, 20, 30, 0.5)"),
            Some(Color::rgba(10, 20, 30, 128))
        );
        assert_eq!(parse_color("blue"), Some(Color::rgb(0x42, 0x85, 0xf4)));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_color("#12").is_none());
        assert!(parse_color("not-a-color").is_none());
    }
}
