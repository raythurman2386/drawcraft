use thiserror::Error;

/// Rendering and rasterisation failures.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("invalid SVG: {0}")]
    Svg(String),
    #[error("invalid raster size: {width}x{height}")]
    InvalidSize { width: f64, height: f64 },
    #[error("raster too large: {width}x{height} pixels")]
    TooLarge { width: u32, height: u32 },
    #[error("PNG encoding failed: {0}")]
    Encoding(String),
    #[error("image decoding failed: {0}")]
    Decode(String),
}
