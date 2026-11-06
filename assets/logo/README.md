# DrawCraft brand assets

`drawcraft-icon.svg` is Raymond's own true-vector artwork, produced with
Vector Craft MCP. It is installed byte-for-byte as the app-icon master: three
paths, a `0 0 1024 1024` viewBox, and a rounded `#15472E` background. The white
`#FFFFFF` D and separate pencil tip form the foreground. `drawcraft-mark.svg`
retains those same two white foreground paths on transparency, without the
background. No path geometry is redrawn or hand-traced from a JPEG, and no
fonts, filters, or gradients are embedded.

The PNGs are rendered from that master SVG at 16, 32, 48, 64, 128, 256, 512,
and 1024 pixels with librsvg and ImageMagick (Lanczos reduction); fully
transparent pixels are normalized to `(0, 0, 0, 0)`. `drawcraft.ico` contains
six uncompressed, 32-bit BMP/DIB frames (16–256), not PNG-compressed frames.
`drawcraft.icns` contains ten valid length-prefixed PNG entries, including
Retina representations. Corners outside the rounded square remain transparent.

Export fidelity means these raster and container assets match renders from
this byte-exact Vector Craft master SVG, allowing documented 8-bit conversion
quantization—not a pixel- or tip-percentage comparison to the earlier JPEG.

`egui::ViewportBuilder` consumes the embedded 256 px PNG for the native window;
on Linux `with_app_id("drawcraft")` matches the desktop entry to the hicolor
icon. The app build script embeds the ICO only when targeting Windows. `xtask
bundle` adds a portable macOS app bundle with this ICNS on macOS, and release CI
carries the standard metadata without introducing a new installer stack.
