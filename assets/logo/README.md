# DrawCraft brand assets

`drawcraft-icon.svg` is the approved forest-green app icon. `drawcraft-mark.svg`
uses the same cream drawing mark on transparency. Both are hand-traced Bézier
paths from the approved t2148u artwork; no raster image tracing, fonts, filters,
or gradients are embedded. The foreground uses `#FBF6EA` and background uses
`#174A30`.

The PNGs were rendered from the SVG at 16, 32, 48, 64, 128, 256, 512, and 1024
pixels with librsvg and ImageMagick (Lanczos reduction). `drawcraft.ico` contains
six uncompressed BMP/DIB frames (16–256) and `drawcraft.icns` contains ten valid ICNS
PNG entries, including Retina representations.

`egui::ViewportBuilder` consumes the embedded 256 px PNG for the native window;
on Linux `with_app_id("drawcraft")` matches the desktop entry to the hicolor
icon. The app build script embeds the ICO only when targeting Windows. `xtask
bundle` adds a portable macOS app bundle with this ICNS on macOS, and release CI
carries the standard metadata without introducing a new installer stack.
