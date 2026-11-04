use resvg::tiny_skia;
use resvg::usvg;

use crate::RenderError;

/// Maximum output dimension in pixels, to keep exports bounded.
pub const MAX_DIMENSION: u32 = 16_384;
/// Maximum total pixels (roughly 64 megapixels).
pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;

/// Rasterise an SVG document to PNG bytes.
pub fn render_png(svg: &str, scale: f64, transparent: bool) -> Result<Vec<u8>, RenderError> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree =
        usvg::Tree::from_str(svg, &options).map_err(|error| RenderError::Svg(error.to_string()))?;
    let size = tree.size();
    let scale = if scale.is_finite() {
        scale.clamp(0.05, 8.0)
    } else {
        1.0
    };
    let width = f64::from(size.width()) * scale;
    let height = f64::from(size.height()) * scale;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidSize { width, height });
    }
    let width = width.ceil() as u32;
    let height = height.ceil() as u32;
    if width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err(RenderError::TooLarge { width, height });
    }
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or(RenderError::InvalidSize {
        width: f64::from(width),
        height: f64::from(height),
    })?;
    if !transparent {
        pixmap.fill(tiny_skia::Color::WHITE);
    }
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| RenderError::Encoding(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_tiny_svg() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>"##;
        let bytes = render_png(svg, 1.0, false).unwrap_or_default();
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn rejects_absurd_scale() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="100000"/>"##;
        assert!(render_png(svg, 8.0, false).is_err());
    }
}
