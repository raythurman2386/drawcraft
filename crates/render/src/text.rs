/// Approximate character width as a fraction of font size.
///
/// This is intentionally simple and deterministic: both the GPU UI and SVG
/// export use the same measurement so wrapped text stays close between them.
const CHAR_WIDTH_RATIO: f64 = 0.55;

/// Horizontal text alignment.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    #[must_use]
    pub const fn as_svg(self) -> &'static str {
        match self {
            Self::Left => "start",
            Self::Center => "middle",
            Self::Right => "end",
        }
    }
}

/// Estimate the width of one line.
#[must_use]
pub fn text_width(text: &str, font_size: f64) -> f64 {
    text.chars().count() as f64 * font_size * CHAR_WIDTH_RATIO
}

#[must_use]
pub fn line_height(font_size: f64) -> f64 {
    font_size * 1.25
}

/// Greedy word wrapping with hard breaks for very long words.
#[must_use]
pub fn wrap_text(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let font_size = font_size.max(1.0);
    let max_width = max_width.max(font_size);
    let max_chars = ((max_width / (font_size * CHAR_WIDTH_RATIO)).floor() as usize).max(1);
    let mut lines = Vec::new();
    for source_line in text.split('\n').take(10_000) {
        if source_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in source_line.split_whitespace() {
            let mut word = word.to_owned();
            if current.is_empty() {
                while word.chars().count() > max_chars {
                    let split = word
                        .char_indices()
                        .nth(max_chars)
                        .map_or(word.len(), |(index, _)| index);
                    lines.push(word[..split].to_owned());
                    word = word[split..].to_owned();
                }
                current = word;
            } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
                current.push(' ');
                current.push_str(&word);
            } else {
                lines.push(std::mem::take(&mut current));
                while word.chars().count() > max_chars {
                    let split = word
                        .char_indices()
                        .nth(max_chars)
                        .map_or(word.len(), |(index, _)| index);
                    lines.push(word[..split].to_owned());
                    word = word[split..].to_owned();
                }
                current = word;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Escape text for XML/SVG content.
#[must_use]
pub fn escape_xml(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_on_words() {
        let lines = wrap_text("hello world again", 70.0, 20.0);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| !line.is_empty()));
    }

    #[test]
    fn preserves_newlines() {
        let lines = wrap_text("a\n\nb", 1000.0, 10.0);
        assert_eq!(lines, vec!["a", "", "b"]);
    }

    #[test]
    fn escapes_xml() {
        assert_eq!(escape_xml("<a&b>"), "&lt;a&amp;b&gt;");
    }
}
