# Sidebars: tool rail and alignment controls (S1)

Scope: left tool rail width/spacing and right inspector alignment commands.
Not covered: rotation balloons, inline text, lifecycle, and point editing.

## Ordinary GUI regression

`scripts/gui-sidebars.sh P BIN` runs a private `Xvfb :198` display, isolates
per-case `XDG_CONFIG_HOME`, and uses disposable copies of the offset seed fixture. It
captures actual framebuffer PNGs with `import -window root`. XTest supplies
ordinary focus, mouse clicks, and keyboard shortcuts; no internal Editor setters
or MCP mutation are used for actions. Optional app tracing emits read-only button
rectangles/labels/actual `Response.enabled()` state to per-control JSON only in the proof run. XTest clicks
those actual coordinates, avoiding fonts/window scale-dependent hard-coded positions.

The fixture contains a 100x80 rectangle at (20,30), an 80x120 ellipse at
(180,110), and a 60x40 rectangle at (340,20). Select All selects the three.
The six align actions are invoked separately, then undo via Ctrl+Z. Saved JSON
is compared against calculated alignment bounds and the original document.
It then checks one-selection disabled alignment/distribution, enables Distribute
H/V and flips for their appropriate selection counts, and verifies one-step undo.
Window sizes tested are 960x620, 1280x800, and 1440x900. The same fixture exercises single-shape Flip H/V, ignored disabled Align, Distribute H/V geometry, and one-step undo. Long-label tools are clicked through their actual rectangles. `run-passed.txt` is written only after every assertion succeeds; wrapper cleanup runs on either status.

Every enabled three-shape hover and disabled one-shape hover asserts both the real `Response.enabled()` and the exact text of the visible Label created inside egui's Tooltip UI. Its visible bounds must contain real framebuffer pixels in the corresponding `import -window root` capture; this is not an intended-boolean or PNG-presence-only assertion. Disabled clicks remain document no-ops.

All six actual allocated Response rectangles must have vertical centres within 1 physical pixel on every alignment control read. `alignment-row-proof.json` additionally records the actual rectangles and centre spreads at 960x620, 1280x800, and 1440x900; inspect the matching real framebuffer PNGs for a level row.

Prerequisites: `Xvfb`, ImageMagick `import`, `libX11`, `libXtst`, Python 3 with Pillow.
Extend this script with a new ordinary-click case when the sidebar gains a command.

The script is expected to pass against the new binary. The baseline comparison must use the frozen exact-main binary without proof instrumentation; its legacy glyph-button positions are calibrated separately.
