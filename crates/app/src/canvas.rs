use base64::Engine as _;
use drawcraft_command::ids;
use drawcraft_document::{Dash, Rect as DocRect, ShapeId, ShapeKind, ShapeRecord, Vec2};
use drawcraft_editor::{Tool, default_style_for, linear_create_drag_commits};
use drawcraft_geometry::Camera;
use drawcraft_render::{Path, TextAlign, build_scene, triangulate};
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Key, Mesh, Pos2,
    Rect as EguiRect, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2 as EVec2,
    pos2, vec2,
};
use serde_json::json;

use crate::app::DrawCraftApp;
use crate::inline_edit::{self, InlineEditAction, InlineKey};
use crate::state::{DragState, InlineTextEdit, ResizeHandle};
use crate::theme;
use crate::transform;

/// A camera plus the screen-space origin of the canvas within the window.
#[derive(Copy, Clone)]
pub(crate) struct View {
    pub camera: Camera,
    pub viewport: Vec2,
    pub origin: Pos2,
}

impl View {
    fn pos(self, point: Vec2) -> Pos2 {
        let screen = self.camera.world_to_screen(point, self.viewport);
        self.origin + vec2(screen.x as f32, screen.y as f32)
    }

    fn rect(self, rect: DocRect) -> EguiRect {
        EguiRect::from_min_max(self.pos(rect.min), self.pos(rect.max))
    }

    fn to_world(self, point: Vec2) -> Vec2 {
        self.camera.screen_to_world(point, self.viewport)
    }
}

impl DrawCraftApp {
    pub(crate) fn canvas(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        if available.x < 4.0 || available.y < 4.0 {
            return;
        }
        let response = ui.allocate_response(available, Sense::click_and_drag());
        let rect = response.rect;
        let viewport = Vec2::new(f64::from(rect.width()), f64::from(rect.height()));
        self.session.editor.set_viewport(viewport);

        let pointer = ui.input(|input| input.pointer.hover_pos());
        let pointer_relative = pointer.map(|position| to_vec2(position - rect.min));
        self.handle_viewport_input(ui, &response, pointer_relative, viewport);
        self.handle_pointer_input(ui, &response, rect, viewport, pointer);

        let view = View {
            camera: self.session.editor.camera,
            viewport,
            origin: rect.min,
        };
        let colors = theme::canvas_colors(self.session.editor.view.dark);
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, colors.background);
        if self.session.editor.view.grid {
            paint_grid(&painter, rect, view, colors);
        }
        self.paint_scene(ui, &painter, view);
        let palette = theme::palette(self.session.editor.view.dark);
        if self.session.editor.doc.shapes().is_empty() && self.state.drag.is_none() {
            paint_empty_state(&painter, view, palette);
        } else if self.state.drag.is_none() {
            paint_tool_hint(&painter, view, self.session.editor.tool, palette);
        }
        self.paint_overlays(&painter, view, pointer);

        if let Some(pointer) = pointer {
            self.state.last_pointer_world = view.to_world(to_vec2(pointer - rect.min));
        }
        self.update_cursor(ui, &response, rect, view, pointer);
        self.paint_inline_text_edit(ui, view, &palette);
    }

    fn handle_viewport_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        pointer: Option<Vec2>,
        viewport: Vec2,
    ) {
        let camera = &mut self.session.editor.camera;
        let scroll = ui.input(|input| input.smooth_scroll_delta);
        let zoom_delta = ui.input(|input| input.zoom_delta());
        let command = ui.input(|input| input.modifiers.command);
        if let Some(pointer) = pointer {
            if (zoom_delta - 1.0).abs() > 0.001 {
                camera.zoom_at(pointer, viewport, f64::from(zoom_delta));
            }
            if scroll != EVec2::ZERO {
                if command {
                    let factor = (f64::from(scroll.y) * 0.002).exp();
                    camera.zoom_at(pointer, viewport, factor);
                } else {
                    camera.pan(Vec2::new(f64::from(scroll.x), f64::from(scroll.y)));
                }
            }
        }

        let space = ui.input(|input| input.key_down(Key::Space));
        let middle = ui.input(|input| input.pointer.middle_down());
        let hand = self.session.editor.tool == Tool::Hand;
        if middle || (space && response.dragged()) || (hand && response.dragged()) {
            let delta = ui.input(|input| input.pointer.delta());
            camera.pan(Vec2::new(f64::from(delta.x), f64::from(delta.y)));
        }
    }

    fn handle_pointer_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: EguiRect,
        viewport: Vec2,
        pointer: Option<Pos2>,
    ) {
        let Some(pointer) = pointer else {
            return;
        };
        let camera = self.session.editor.camera;
        let view = View {
            camera,
            viewport,
            origin: rect.min,
        };
        let local = to_vec2(pointer - rect.min);
        let world = view.to_world(local);
        let space = ui.input(|input| input.key_down(Key::Space));
        let middle = ui.input(|input| input.pointer.middle_down());
        if space || middle {
            return;
        }
        if self.state.inline_text_edit.is_some() {
            // Inline text editing consumes all pointer input; the inline editor
            // handles commit/cancel via its own focus handling.
            return;
        }
        self.state.hover = self.session.editor.topmost_at(world, 3.0 / camera.zoom);

        if response.drag_started() {
            self.begin_drag(ui, pointer, view, world);
        }
        if response.dragged() {
            if let Some(drag) = self.state.drag.take() {
                self.update_drag(ui, drag, pointer, world);
            }
        }
        if response.drag_stopped() {
            self.finish_drag(ui, world);
        }
        if response.clicked() {
            self.handle_click(ui, world, view);
        }
        if response.double_clicked() {
            if let Some(id) = self.session.editor.topmost_at(world, 3.0 / camera.zoom) {
                if let Some(shape) = self.session.editor.doc.shape(&id) {
                    if shape.kind.has_text() {
                        self.state.inline_text_edit =
                            Some(InlineTextEdit::new(id, shape.display_text().to_owned()));
                    }
                }
            }
        }
    }

    fn begin_drag(&mut self, ui: &egui::Ui, pointer: Pos2, view: View, world: Vec2) {
        let tool = self.session.editor.tool;
        let camera = view.camera;
        if tool == Tool::Select {
            if let Some(bounds) = self.selection_screen_rect(view) {
                if let Some(handle) = hit_handle(pointer, bounds) {
                    let original = self
                        .session
                        .editor
                        .selected_shapes()
                        .into_iter()
                        .cloned()
                        .collect::<Vec<_>>();
                    if original.is_empty() {
                        return;
                    }
                    self.session.editor.snapshot();
                    self.state.drag = Some(DragState::Resize {
                        handle,
                        start_bounds: self
                            .session
                            .editor
                            .selection_bounds()
                            .unwrap_or_else(|| DocRect::from_xywh(world.x, world.y, 1.0, 1.0)),
                        pointer_start: world,
                        original,
                    });
                    return;
                }
                if hit_rotation_handle(pointer, bounds) {
                    let original = self
                        .session
                        .editor
                        .selected_shapes()
                        .into_iter()
                        .cloned()
                        .collect::<Vec<_>>();
                    if original.is_empty() {
                        return;
                    }
                    let pivot = self
                        .session
                        .editor
                        .selection_bounds()
                        .map(|bounds| bounds.center())
                        .unwrap_or(world);
                    self.session.editor.snapshot();
                    let start_angle = (world.y - pivot.y).atan2(world.x - pivot.x);
                    self.state.drag = Some(DragState::Rotate {
                        pivot,
                        start_angle,
                        angle: 0.0,
                        original,
                    });
                    return;
                }
            }

            let additive = ui.input(|input| input.modifiers.shift);
            let alt = ui.input(|input| input.modifiers.alt);
            let hit = self
                .session
                .editor
                .select_at(world, additive, 3.0 / camera.zoom);
            if hit.is_none() {
                self.state.drag = Some(DragState::Marquee {
                    start: world,
                    current: world,
                    additive,
                });
                return;
            }
            if alt {
                self.session
                    .editor
                    .duplicate_selection(Vec2::new(20.0, 20.0));
            } else {
                self.session.editor.snapshot();
            }
            let original = self
                .session
                .editor
                .selected_shapes()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();
            self.state.drag = Some(DragState::Move {
                start: world,
                original,
            });
            return;
        }

        if tool == Tool::Lasso {
            let additive = ui.input(|input| input.modifiers.shift);
            self.state.drag = Some(DragState::Lasso {
                points: vec![world],
                additive,
            });
            return;
        }

        if tool == Tool::Eraser {
            self.session.editor.snapshot();
            self.state.drag = Some(DragState::Erase { last: world });
            return;
        }

        if matches!(
            tool,
            Tool::Draw | Tool::Highlighter | Tool::Laser | Tool::Polyline
        ) {
            self.state.drag = Some(DragState::Freehand {
                tool,
                points: vec![world],
            });
            return;
        }

        if tool.is_shape_tool() || matches!(tool, Tool::Arrow | Tool::Line) {
            self.state.drag = Some(DragState::Create {
                tool,
                start: world,
                current: world,
            });
        }
    }

    fn update_drag(&mut self, ui: &egui::Ui, mut drag: DragState, _pointer: Pos2, world: Vec2) {
        match &mut drag {
            DragState::Move { start, original } => {
                let mut delta = world - *start;
                let snap = ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
                if snap {
                    let ids: Vec<ShapeId> = original.iter().map(|shape| shape.id.clone()).collect();
                    let mut bounds = DocRect::EMPTY;
                    for shape in original.iter() {
                        bounds = bounds.union(shape.world_bounds());
                    }
                    let snap = self.session.editor.snap_offset(
                        bounds.translate(delta),
                        6.0 / self.session.editor.camera.zoom,
                        &ids,
                    );
                    delta += snap;
                }
                self.apply_move(original, delta);
            }
            DragState::Marquee { current, .. } => *current = world,
            DragState::Lasso { points, .. } => {
                let threshold = 2.0 / self.session.editor.camera.zoom.max(0.1);
                if points
                    .last()
                    .is_none_or(|last| last.distance(world) >= threshold)
                {
                    points.push(world);
                }
            }
            DragState::Create { current, .. } => *current = world,
            DragState::Freehand { points, .. } => {
                let threshold = 2.0 / self.session.editor.camera.zoom.max(0.1);
                if points
                    .last()
                    .is_none_or(|last| last.distance(world) >= threshold)
                {
                    points.push(world);
                }
            }
            DragState::Resize {
                handle,
                start_bounds,
                original,
                ..
            } => {
                let modifiers = ui.input(|input| transform::ResizeModifiers {
                    keep_aspect: input.modifiers.shift,
                    from_centre: input.modifiers.alt,
                });
                let (anchor, scale) =
                    transform::resize_scale(*handle, *start_bounds, world, modifiers);
                self.apply_resize(original, anchor, scale);
            }
            DragState::Rotate {
                pivot,
                start_angle,
                angle,
                original,
            } => {
                let snap = ui.input(|input| input.modifiers.shift);
                let raw = (world.y - pivot.y).atan2(world.x - pivot.x) - *start_angle;
                *angle = transform::rotate_angle(raw, snap);
                self.apply_rotate(original, *pivot, *angle);
            }

            DragState::Erase { last } => {
                let step = (world - *last).length();
                if step < 1.0 / self.session.editor.camera.zoom.max(0.1) {
                    *last = world;
                    self.state.drag = Some(drag);
                    return;
                }
                *last = world;
                if let Some(id) = self
                    .session
                    .editor
                    .topmost_at(world, 4.0 / self.session.editor.camera.zoom)
                {
                    let _ = self.session.editor.delete_shape(&id);
                }
            }
        }
        // Mirror the live transform readout in the status bar.
        let readout_angle = match &drag {
            DragState::Rotate { angle, .. } => Some(*angle),
            DragState::Resize { .. } => Some(0.0),
            _ => None,
        };
        if let Some(angle) = readout_angle {
            let size = self
                .session
                .editor
                .selection_bounds()
                .map(|bounds| bounds.size())
                .unwrap_or_default();
            let verb = if matches!(drag, DragState::Rotate { .. }) {
                "Rotate"
            } else {
                "Resize"
            };
            self.state
                .set_status(format!("{verb} {}", transform::readout(size, angle)));
        }
        self.state.drag = Some(drag);
    }

    /// Snap arrow drag endpoints via the shared editor resolver (preview == commit).
    #[must_use]
    fn arrow_preview_endpoints(&self, start: Vec2, current: Vec2, zoom: f64) -> (Vec2, Vec2) {
        let tolerance = 12.0 / zoom.max(0.1);
        self.session
            .editor
            .arrow_endpoints_for_drag(start, current, tolerance)
    }

    fn finish_drag(&mut self, _ui: &egui::Ui, world: Vec2) {
        let Some(drag) = self.state.drag.take() else {
            return;
        };
        match drag {
            DragState::Move { .. } => {
                self.session.editor.refresh_arrows();
                self.session.mark_dirty();
            }
            DragState::Marquee {
                start,
                current,
                additive,
            } => {
                let rect = DocRect::new(start, current);
                self.session.editor.brush_select(rect, additive);
            }
            DragState::Lasso { points, additive } => {
                self.session.editor.lasso_select(&points, additive);
            }
            DragState::Create {
                tool,
                start,
                current,
            } => {
                // Polyline starts DragState::Freehand, so only Arrow|Line land here.
                if matches!(tool, Tool::Arrow | Tool::Line) {
                    // Linear tools are placed by dragging. Gate on drag length so
                    // axis-aligned (horizontal/vertical) strokes still commit.
                    // Ignore accidental tiny drags; never fall back to a default box.
                    if linear_create_drag_commits(start, current) {
                        if tool == Tool::Arrow {
                            let (start_snapped, end_snapped) = self.arrow_preview_endpoints(
                                start,
                                current,
                                self.session.editor.camera.zoom,
                            );
                            let _ = self.session.editor.add_arrow_between(
                                start_snapped,
                                end_snapped,
                                true,
                            );
                        } else {
                            self.create_linear(tool, start, current);
                        }
                    }
                } else {
                    let rect = DocRect::new(start, current);
                    let rect = if rect.width() < 3.0 || rect.height() < 3.0 {
                        default_rect(tool, start)
                    } else {
                        rect
                    };
                    self.create_with_tool(tool, rect, Some(current));
                }
            }
            DragState::Freehand { tool, points } => {
                if tool != Tool::Laser && points.len() >= 2 {
                    self.create_freehand(tool, points);
                }
            }
            DragState::Resize { .. } | DragState::Rotate { .. } => {
                self.session.editor.refresh_arrows();
                self.session.mark_dirty();
            }
            DragState::Erase { .. } => {
                self.session.mark_dirty();
            }
        }
        let _ = world;
    }

    /// Render an inline text editor directly on the canvas over the shape being
    /// edited. Handles commit (Enter / lost focus), Shift+Enter newlines in
    /// Sticky/Note, and cancel (Escape); see [`inline_edit::decide`].
    fn paint_inline_text_edit(&mut self, ui: &mut egui::Ui, view: View, palette: &theme::Palette) {
        let Some(edit) = self.state.inline_text_edit.as_mut() else {
            return;
        };
        let Some(shape) = self.session.editor.doc.shape(&edit.id) else {
            self.state.inline_text_edit = None;
            return;
        };

        let base_rect = shape.rect();
        let (inner_padding, text_rect) = match shape.kind {
            ShapeKind::Text => (
                6.0,
                drawcraft_document::Rect::from_xywh(
                    base_rect.min.x + 4.0,
                    base_rect.min.y + 2.0,
                    base_rect.width().max(24.0),
                    base_rect.height(),
                ),
            ),
            ShapeKind::Sticky | ShapeKind::Note => (
                10.0,
                drawcraft_document::Rect::from_xywh(
                    base_rect.min.x + 16.0,
                    base_rect.min.y + 16.0,
                    (base_rect.width() - 32.0).max(24.0),
                    base_rect.height(),
                ),
            ),
            _ => return,
        };
        let screen_rect = view.rect(text_rect);

        let area_id = Id::new("inline_text_edit").with(edit.id.clone());
        // Distinct from the Area id: sharing one id between the Area and the
        // TextEdit makes their interactions collide.
        let text_id = area_id.with("text");
        let multiline = matches!(shape.kind, ShapeKind::Sticky | ShapeKind::Note);

        // Enter/Escape belong to the editor only while it owns the keyboard.
        // `had_focus` covers Escape, which egui uses to drop focus before
        // widgets run; "nobody focused" covers the frame before focus lands.
        let owns_keyboard = edit.had_focus
            || ui
                .ctx()
                .memory(|mem| mem.has_focus(text_id) || mem.focused().is_none());
        let key = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    modifiers,
                    ..
                } => Some(InlineKey::Enter {
                    shift: modifiers.shift,
                }),
                egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => Some(InlineKey::Escape),
                _ => None,
            })
        });
        let key_action = inline_edit::decide(multiline, owns_keyboard, key, false);
        if key_action.closes() {
            // Consume the key before the TextEdit runs so a multiline field
            // never inserts a stray newline on the committing Enter.
            ui.input_mut(|input| {
                input.events.retain(|event| {
                    !matches!(
                        event,
                        egui::Event::Key {
                            key: Key::Enter | Key::Escape,
                            pressed: true,
                            ..
                        }
                    )
                });
            });
        }

        let mut lost_focus = false;
        let mut has_focus = false;
        let focus_requested = &mut edit.focus_requested;
        let text = &mut edit.text;

        egui::Area::new(area_id)
            .fixed_pos(screen_rect.min)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(screen_rect.size());

                // Background frame: readable over any fill. The focus ring is
                // set after the TextEdit runs, from its own response.
                let mut prepared = Frame::new()
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.0, palette.muted))
                    .inner_margin(inner_padding)
                    .corner_radius(4.0)
                    .begin(ui);
                {
                    let ui = &mut prepared.content_ui;
                    let desired_width = (screen_rect.width() - 2.0 * inner_padding).max(80.0);
                    let widget = if multiline {
                        egui::TextEdit::multiline(text)
                            .id(text_id)
                            .desired_width(desired_width)
                            .desired_rows(4)
                            .hint_text("Type here…")
                    } else {
                        egui::TextEdit::singleline(text)
                            .id(text_id)
                            .desired_width(desired_width)
                            .hint_text("Type here…")
                    };
                    let response = ui.add(widget);
                    if !*focus_requested {
                        response.request_focus();
                        *focus_requested = true;
                    }
                    lost_focus = response.lost_focus();
                    has_focus = response.has_focus();
                }
                if has_focus {
                    prepared.frame.stroke = Stroke::new(1.0, palette.accent);
                }
                prepared.end(ui);
            });
        edit.had_focus = has_focus;

        let action = if key_action.closes() {
            key_action
        } else {
            inline_edit::decide(multiline, owns_keyboard, None, lost_focus)
        };
        if !action.closes() {
            return;
        }
        let id = edit.id.clone();
        let edited = std::mem::take(&mut edit.text);
        self.state.inline_text_edit = None;
        ui.ctx().memory_mut(|mem| mem.surrender_focus(text_id));
        if action == InlineEditAction::Commit {
            let current = self
                .session
                .editor
                .doc
                .shape(&id)
                .map(|shape| shape.display_text().to_owned())
                .unwrap_or_default();
            if let Some(text) = inline_edit::committed_text(&current, &edited) {
                self.run(ids::SHAPE_SET_TEXT, json!({"id": id, "text": text}));
            }
        }
    }

    fn handle_click(&mut self, ui: &egui::Ui, world: Vec2, view: View) {
        let tool = self.session.editor.tool;
        let camera = view.camera;
        let additive = ui.input(|input| input.modifiers.shift);
        match tool {
            Tool::Select => {
                self.session
                    .editor
                    .select_at(world, additive, 3.0 / camera.zoom);
            }
            Tool::Eraser => {
                if let Some(id) = self.session.editor.topmost_at(world, 4.0 / camera.zoom) {
                    let _ = self.session.editor.delete_shape(&id);
                }
            }
            Tool::Text => {
                let rect = DocRect::from_xywh(world.x, world.y, 240.0, 80.0);
                self.create_with_tool(tool, rect, None);
            }
            Tool::Sticky => {
                let rect = DocRect::from_xywh(world.x - 110.0, world.y - 80.0, 220.0, 160.0);
                self.create_with_tool(tool, rect, None);
            }
            Tool::Frame => {
                let rect = DocRect::from_xywh(world.x, world.y, 480.0, 320.0);
                self.create_with_tool(tool, rect, None);
            }
            Tool::Image => {
                let rect = DocRect::from_xywh(world.x, world.y, 320.0, 200.0);
                self.create_with_tool(tool, rect, None);
            }
            Tool::Draw | Tool::Highlighter | Tool::Laser => {}
            Tool::Lasso => {}
            Tool::Hand => {}
            _ => {
                // Drag-based linear tools (Arrow, Line) and freehand tools that
                // reach this arm (Polyline) are not placed by a single click.
                if tool.is_shape_tool() && !tool.is_linear_tool() {
                    let rect = default_rect(tool, world);
                    self.create_with_tool(tool, rect, None);
                }
            }
        }
    }

    fn create_with_tool(
        &mut self,
        tool: Tool,
        rect: DocRect,
        end: Option<Vec2>,
    ) -> Option<ShapeId> {
        let kind = tool.shape_kind()?;
        if kind == ShapeKind::Image {
            return self.create_image(rect);
        }
        if kind == ShapeKind::Arrow {
            let start = rect.min;
            let end = end.unwrap_or(rect.max);
            return self.session.editor.add_arrow_between(start, end, true).ok();
        }
        let id = self.session.editor.next_shape_id();
        let mut shape = ShapeRecord::new(id.clone(), kind, rect.min, rect.size());
        shape.style = default_style_for(kind, self.session.editor.view.dark);
        if kind.is_linear() {
            shape.points = vec![rect.min, rect.max];
            shape.sync_rect_from_points();
        }
        if kind == ShapeKind::Text {
            shape.text = "Text".into();
        }
        if kind == ShapeKind::Note {
            shape.text = "Note".into();
        } else if kind == ShapeKind::Sticky {
            shape.text = "Sticky".into();
        }
        if kind == ShapeKind::Frame {
            shape.style.fill = drawcraft_document::Fill::None;
        }
        let id = self.session.editor.insert_shape(shape).ok()?;
        if matches!(kind, ShapeKind::Text | ShapeKind::Sticky | ShapeKind::Note) {
            let text = self
                .session
                .editor
                .doc
                .shape(&id)
                .map(|shape| shape.display_text().to_owned())
                .unwrap_or_default();
            self.state.inline_text_edit = Some(InlineTextEdit::new(id.clone(), text));
        }
        Some(id)
    }

    fn create_linear(&mut self, tool: Tool, start: Vec2, end: Vec2) {
        let Some(kind) = tool.shape_kind() else {
            return;
        };
        let id = self.session.editor.next_shape_id();
        let mut shape = ShapeRecord::new(id, kind, start, Vec2::new(1.0, 1.0));
        shape.style = default_style_for(kind, self.session.editor.view.dark);
        shape.points = vec![start, end];
        shape.sync_rect_from_points();
        let _ = self.session.editor.insert_shape(shape);
    }

    fn create_image(&mut self, rect: DocRect) -> Option<ShapeId> {
        let path = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
            .pick_file()?;
        let source = path.display().to_string();
        let id = self.session.editor.next_shape_id();
        let mut shape = ShapeRecord::new(id.clone(), ShapeKind::Image, rect.min, rect.size());
        shape.style = default_style_for(ShapeKind::Image, self.session.editor.view.dark);
        shape.src = Some(source);
        self.session.editor.insert_shape(shape).ok()
    }

    fn create_freehand(&mut self, tool: Tool, points: Vec<Vec2>) {
        let kind = match tool {
            Tool::Highlighter => ShapeKind::Highlighter,
            Tool::Polyline => ShapeKind::Polyline,
            _ => ShapeKind::Draw,
        };
        let id = self.session.editor.next_shape_id();
        let mut shape = ShapeRecord::new(
            id,
            kind,
            points.first().copied().unwrap_or_default(),
            Vec2::new(1.0, 1.0),
        );
        shape.style = default_style_for(kind, self.session.editor.view.dark);
        shape.points = points;
        shape.sync_rect_from_points();
        let _ = self.session.editor.insert_shape(shape);
    }

    fn apply_move(&mut self, original: &[ShapeRecord], delta: Vec2) {
        for source in original {
            if let Some(shape) = self.session.editor.doc.shape_mut(&source.id) {
                let mut next = source.clone();
                next.apply_translation(delta);
                *shape = next;
            }
        }
        self.session.editor.refresh_arrows();
    }

    fn apply_resize(&mut self, original: &[ShapeRecord], anchor: Vec2, scale: Vec2) {
        for source in original {
            if let Some(shape) = self.session.editor.doc.shape_mut(&source.id) {
                let mut next = source.clone();
                next.scale_from(anchor, scale);
                *shape = next;
            }
        }
        self.session.editor.refresh_arrows();
    }

    fn apply_rotate(&mut self, original: &[ShapeRecord], pivot: Vec2, angle: f64) {
        for source in original {
            if let Some(shape) = self.session.editor.doc.shape_mut(&source.id) {
                let mut next = source.clone();
                let center = next.center();
                let delta = center - pivot;
                let (sin, cos) = angle.sin_cos();
                let rotated = Vec2::new(
                    pivot.x + delta.x * cos - delta.y * sin,
                    pivot.y + delta.x * sin + delta.y * cos,
                );
                next.apply_translation(rotated - center);
                next.rotate_by(angle);
                *shape = next;
            }
        }
        self.session.editor.refresh_arrows();
    }

    fn paint_scene(&mut self, ui: &egui::Ui, painter: &egui::Painter, view: View) {
        let Some(session) = self.session.editor.doc.active() else {
            return;
        };
        let focus = self.session.editor.view.focus;
        let selection: Vec<ShapeId> = self.session.editor.selection.ids().to_vec();
        let scene = build_scene(session, None);
        for item in &scene.items {
            let selected = item.id.as_ref().is_some_and(|id| selection.contains(id));
            let dim = if focus && !selected { 0.18 } else { 1.0 };
            let opacity = item.opacity * dim;
            let painter = if let Some(clip) = item.clip {
                painter.with_clip_rect(view.rect(clip))
            } else {
                painter.clone()
            };
            for fill in &item.fills {
                let opacity = fill.opacity * opacity;
                if fill.pattern {
                    draw_fill(&painter, &fill.path, &fill.color, opacity * 0.4, view);
                } else {
                    draw_fill(&painter, &fill.path, &fill.color, opacity, view);
                }
            }
            for stroke in &item.strokes {
                draw_stroke(&painter, stroke, stroke.opacity * opacity, view);
            }
            for hatch in &item.hatch {
                draw_stroke(
                    &painter,
                    &drawcraft_render::StrokePaint {
                        path: hatch.clone(),
                        color: item
                            .fills
                            .first()
                            .map(|fill| fill.color.clone())
                            .unwrap_or_else(|| "#1e1e2e".into()),
                        width: 1.0,
                        dash: Dash::Solid,
                        opacity: 1.0,
                    },
                    0.55 * opacity,
                    view,
                );
            }
            if let Some(text) = &item.text {
                draw_text(&painter, text, opacity, view);
            }
            if let Some(image) = &item.image {
                if let Some(id) = &item.id {
                    draw_image(self, ui.ctx(), &painter, id, image, opacity, view);
                }
            }
        }
    }

    fn paint_overlays(&self, painter: &egui::Painter, view: View, pointer: Option<Pos2>) {
        let colors = theme::canvas_colors(self.session.editor.view.dark);
        let palette = theme::palette(self.session.editor.view.dark);
        if let Some(bounds) = self.selection_screen_rect(view) {
            painter.rect_stroke(
                bounds,
                CornerRadius::ZERO,
                Stroke::new(1.35, colors.selection),
                StrokeKind::Middle,
            );
            if self.session.editor.tool == Tool::Select {
                draw_handles(painter, bounds, colors);
            }
            if self.state.drag.is_some() {
                let world = self
                    .session
                    .editor
                    .selection_bounds()
                    .map(|bounds| bounds.size())
                    .unwrap_or_default();
                let angle = match &self.state.drag {
                    Some(DragState::Rotate { angle, .. }) => *angle,
                    _ => 0.0,
                };
                let label = transform::readout(world, angle);
                let label_rect = EguiRect::from_min_size(
                    pos2(bounds.left(), bounds.bottom() + 8.0),
                    vec2(112.0, 18.0),
                );
                painter.rect_filled(label_rect, CornerRadius::same(5), palette.surface);
                painter.rect_stroke(
                    label_rect,
                    CornerRadius::same(5),
                    Stroke::new(1.0, palette.highlight),
                    StrokeKind::Inside,
                );
                painter.text(
                    label_rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(10.5),
                    palette.muted,
                );
            }
        }

        if let Some(DragState::Marquee {
            start,
            current,
            additive: _,
        }) = &self.state.drag
        {
            let rect = DocRect::new(*start, *current);
            let screen = view.rect(rect);
            painter.rect_filled(screen, 0.0, colors.marquee_fill);
            painter.rect_stroke(
                screen,
                CornerRadius::ZERO,
                Stroke::new(1.0, colors.selection),
                StrokeKind::Inside,
            );
        }

        if let Some(DragState::Lasso { points, .. }) = &self.state.drag {
            let screen: Vec<Pos2> = points.iter().map(|point| view.pos(*point)).collect();
            if screen.len() >= 2 {
                painter.add(egui::Shape::line(
                    screen,
                    Stroke::new(1.25, colors.selection),
                ));
            }
        }

        if let Some(DragState::Create {
            tool,
            start,
            current,
        }) = &self.state.drag
        {
            let start_screen = view.pos(*start);
            let current_screen = view.pos(*current);
            match tool {
                Tool::Line => {
                    painter.add(egui::Shape::line(
                        vec![start_screen, current_screen],
                        Stroke::new(2.0, colors.selection),
                    ));
                }
                Tool::Arrow => {
                    let (start_snapped, end_snapped) =
                        self.arrow_preview_endpoints(*start, *current, view.camera.zoom);
                    let start_screen = view.pos(start_snapped);
                    let end_screen = view.pos(end_snapped);
                    painter.add(egui::Shape::line(
                        vec![start_screen, end_screen],
                        Stroke::new(2.0, colors.selection),
                    ));
                    let direction = (end_snapped - start_snapped).normalized();
                    if direction.length_squared() > f64::EPSILON {
                        let normal = direction.perp();
                        let size = 10.0 / view.camera.zoom.max(0.1);
                        let back = end_snapped - direction * size;
                        let half = size * 0.5;
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                view.pos(end_snapped),
                                view.pos(back + normal * half),
                                view.pos(back - normal * half),
                            ],
                            colors.selection,
                            Stroke::new(1.0, colors.selection),
                        ));
                    }
                }
                _ => {
                    let rect = DocRect::new(*start, *current);
                    let screen = view.rect(rect);
                    painter.rect_stroke(
                        screen,
                        CornerRadius::same(2),
                        Stroke::new(1.5, colors.selection),
                        StrokeKind::Inside,
                    );
                    painter.rect_filled(screen, 0.0, colors.marquee_fill);
                }
            }
        }

        if let Some(DragState::Freehand { tool, points }) = &self.state.drag {
            let screen: Vec<Pos2> = points.iter().map(|point| view.pos(*point)).collect();
            let stroke = if *tool == Tool::Highlighter {
                Stroke::new(
                    (14.0 * view.camera.zoom as f32).max(2.0),
                    theme::with_alpha(palette.warning, 110),
                )
            } else if *tool == Tool::Laser {
                Stroke::new(3.0, palette.error)
            } else {
                Stroke::new(3.0, palette.text)
            };
            painter.add(egui::Shape::line(screen, stroke));
        }

        if let Some(DragState::Erase { .. }) = &self.state.drag {
            if let Some(pointer) = pointer {
                painter.circle_stroke(
                    pointer,
                    18.0,
                    Stroke::new(1.5, colors.selection.gamma_multiply(0.8)),
                );
                painter.circle_filled(pointer, 2.0, colors.selection);
            }
        }

        if let Some(hover) = &self.state.hover {
            if !self.session.editor.selection.contains(hover) {
                if let Some(shape) = self.session.editor.doc.shape(hover) {
                    painter.rect_stroke(
                        view.rect(shape.world_bounds()),
                        CornerRadius::ZERO,
                        Stroke::new(1.0, colors.selection.gamma_multiply(0.5)),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }

    fn selection_screen_rect(&self, view: View) -> Option<EguiRect> {
        self.session
            .editor
            .selection_bounds()
            .map(|bounds| view.rect(bounds))
    }

    fn update_cursor(
        &self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: EguiRect,
        view: View,
        pointer: Option<Pos2>,
    ) {
        let camera = self.session.editor.camera;
        let space = ui.input(|input| input.key_down(Key::Space));
        if space || self.session.editor.tool == Tool::Hand {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
            return;
        }
        if let Some(pointer) = pointer {
            if let Some(bounds) = self.selection_screen_rect(view) {
                if hit_rotation_handle(pointer, bounds) {
                    ui.ctx().set_cursor_icon(CursorIcon::Alias);
                    return;
                }
                if let Some(handle) = hit_handle(pointer, bounds) {
                    ui.ctx().set_cursor_icon(handle.cursor());
                    return;
                }
            }
        }
        let cursor = match self.session.editor.tool {
            Tool::Select => {
                if response.hovered() {
                    if let Some(pointer) = pointer {
                        let local = to_vec2(pointer - rect.min);
                        let world = view.to_world(local);
                        if self
                            .session
                            .editor
                            .topmost_at(world, 3.0 / camera.zoom)
                            .is_some()
                        {
                            CursorIcon::PointingHand
                        } else {
                            CursorIcon::Default
                        }
                    } else {
                        CursorIcon::Default
                    }
                } else {
                    CursorIcon::Default
                }
            }
            Tool::Text => CursorIcon::Text,
            Tool::Draw | Tool::Highlighter | Tool::Laser | Tool::Lasso => CursorIcon::Crosshair,
            Tool::Eraser => CursorIcon::NoDrop,
            _ => CursorIcon::Crosshair,
        };
        ui.ctx().set_cursor_icon(cursor);
    }
}

fn paint_empty_state(painter: &egui::Painter, view: View, p: theme::Palette) {
    let canvas = EguiRect::from_min_size(
        view.origin,
        vec2(view.viewport.x as f32, view.viewport.y as f32),
    );
    let card = EguiRect::from_center_size(canvas.center(), vec2(376.0, 190.0));
    painter.rect_filled(card, CornerRadius::same(14), p.surface);
    painter.rect_stroke(
        card,
        CornerRadius::same(14),
        Stroke::new(1.0, p.highlight),
        StrokeKind::Inside,
    );
    let logo =
        EguiRect::from_center_size(pos2(card.center().x, card.top() + 32.0), vec2(38.0, 38.0));
    painter.rect_filled(logo, CornerRadius::same(10), p.accent);
    painter.text(
        logo.center(),
        Align2::CENTER_CENTER,
        "D",
        FontId::proportional(19.0),
        p.on_accent,
    );
    painter.text(
        pos2(card.center().x, card.top() + 72.0),
        Align2::CENTER_CENTER,
        "Start with a thought",
        FontId::proportional(16.0),
        p.text,
    );
    painter.text(
        pos2(card.center().x, card.top() + 96.0),
        Align2::CENTER_CENTER,
        "Draw, connect, and shape your ideas on an infinite canvas.",
        FontId::proportional(11.0),
        p.muted,
    );
    painter.text(
        pos2(card.center().x, card.top() + 136.0),
        Align2::CENTER_CENTER,
        "V  Select     Q  Lasso     D  Draw     T  Text",
        FontId::monospace(10.5),
        p.muted,
    );
}

fn paint_tool_hint(painter: &egui::Painter, view: View, tool: Tool, p: theme::Palette) {
    if matches!(tool, Tool::Select | Tool::Hand) {
        return;
    }
    let origin = view.origin + vec2(14.0, 14.0);
    let rect = EguiRect::from_min_size(origin, vec2(238.0, 30.0));
    painter.rect_filled(rect, CornerRadius::same(7), p.surface);
    painter.rect_stroke(
        rect,
        CornerRadius::same(7),
        Stroke::new(1.0, p.highlight),
        StrokeKind::Inside,
    );
    painter.text(
        pos2(rect.left() + 11.0, rect.center().y),
        Align2::LEFT_CENTER,
        tool.label(),
        FontId::proportional(11.0),
        p.accent,
    );
    painter.text(
        pos2(rect.right() - 11.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "Drag to place",
        FontId::proportional(10.0),
        p.muted,
    );
}

fn default_rect(tool: Tool, at: Vec2) -> DocRect {
    match tool {
        Tool::Sticky => DocRect::from_xywh(at.x - 110.0, at.y - 80.0, 220.0, 160.0),
        Tool::Frame => DocRect::from_xywh(at.x, at.y, 480.0, 320.0),
        Tool::Text => DocRect::from_xywh(at.x, at.y, 240.0, 80.0),
        _ => DocRect::from_xywh(at.x, at.y, 160.0, 110.0),
    }
}

fn to_vec2(vector: EVec2) -> Vec2 {
    Vec2::new(f64::from(vector.x), f64::from(vector.y))
}

fn draw_fill(painter: &egui::Painter, path: &Path, color: &str, opacity: f64, view: View) {
    let points = path.flatten(24);
    if points.len() < 3 {
        return;
    }
    let screen_points: Vec<Pos2> = points.iter().map(|point| view.pos(*point)).collect();
    let triangles = triangulate(&points);
    if triangles.is_empty() {
        return;
    }
    let fill = theme::color(color, opacity);
    let mut mesh = Mesh::default();
    for point in &screen_points {
        mesh.colored_vertex(*point, fill);
    }
    for triangle in triangles {
        mesh.add_triangle(triangle[0], triangle[1], triangle[2]);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn draw_stroke(
    painter: &egui::Painter,
    stroke: &drawcraft_render::StrokePaint,
    opacity: f64,
    view: View,
) {
    let color = theme::color(&stroke.color, opacity);
    let width = (stroke.width * view.camera.zoom).max(0.75) as f32;
    let paths = match stroke.dash {
        Dash::None => Vec::new(),
        Dash::Solid | Dash::HandDrawn => vec![stroke.path.clone()],
        Dash::Dashed => stroke.path.dashed(stroke.width * 4.0, stroke.width * 3.0),
        Dash::Dotted => stroke.path.dashed(stroke.width * 0.1, stroke.width * 2.5),
    };
    for path in paths {
        let points: Vec<Pos2> = path
            .flatten(16)
            .iter()
            .map(|point| view.pos(*point))
            .collect();
        if points.len() >= 2 {
            painter.add(egui::Shape::line(points, Stroke::new(width, color)));
        }
    }
}

fn draw_text(painter: &egui::Painter, text: &drawcraft_render::TextItem, dim: f64, view: View) {
    let color = theme::color(&text.color, text.opacity * dim);
    let font_size = (text.font_size * view.camera.zoom).max(6.0) as f32;
    let font = FontId::proportional(font_size);
    let wrap = (text.width * view.camera.zoom).max(12.0) as f32;
    let galley = painter.layout(text.text.clone(), font, color, wrap);
    let mut position = view.pos(text.pos);
    position.x += match text.align {
        TextAlign::Left => 0.0,
        TextAlign::Center => (wrap - galley.size().x) * 0.5,
        TextAlign::Right => wrap - galley.size().x,
    };
    let angle = text.rotation as f32;
    if angle.abs() > 1.0e-5 {
        let center = view.pos(text.rotation_center);
        let relative = position - center;
        let (sin, cos) = angle.sin_cos();
        position = center
            + vec2(
                relative.x * cos - relative.y * sin,
                relative.x * sin + relative.y * cos,
            );
    }
    let mut shape = egui::epaint::TextShape::new(position, galley, color);
    shape.angle = if angle.abs() > 1.0e-5 { angle } else { 0.0 };
    painter.add(shape);
}

fn draw_image(
    app: &mut DrawCraftApp,
    ctx: &egui::Context,
    painter: &egui::Painter,
    id: &ShapeId,
    image: &drawcraft_render::ImageItem,
    dim: f64,
    view: View,
) {
    let Some(texture) = ensure_texture(app, ctx, id, &image.href) else {
        let rect = view.rect(image.rect);
        painter.rect_stroke(
            rect,
            CornerRadius::same(2),
            Stroke::new(1.0, theme::palette(app.session.editor.view.dark).accent),
            StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "image",
            FontId::proportional(12.0),
            theme::palette(app.session.editor.view.dark).accent,
        );
        return;
    };
    let rect = view.rect(image.rect);
    let uv = EguiRect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let tint = Color32::WHITE.gamma_multiply(dim as f32);
    painter.image(texture.id(), rect, uv, tint);
}

fn ensure_texture(
    app: &mut DrawCraftApp,
    ctx: &egui::Context,
    id: &ShapeId,
    href: &str,
) -> Option<TextureHandle> {
    if let Some(texture) = app.state.textures.get(id) {
        return Some(texture.clone());
    }
    let bytes = decode_image_source(href)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return None;
    }
    let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
    if image.width() > 8192 || image.height() > 8192 {
        return None;
    }
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    );
    let texture = ctx.load_texture(
        format!("drawcraft-image-{id}"),
        image,
        TextureOptions::LINEAR,
    );
    app.state.textures.insert(id.clone(), texture.clone());
    Some(texture)
}

fn decode_image_source(href: &str) -> Option<Vec<u8>> {
    if let Some(rest) = href.strip_prefix("data:") {
        let (_, payload) = rest.split_once(',')?;
        return base64::engine::general_purpose::STANDARD
            .decode(payload)
            .ok();
    }
    std::fs::read(href).ok()
}

fn draw_handles(painter: &egui::Painter, bounds: EguiRect, colors: theme::CanvasColors) {
    let radius = 3.5;
    for handle in ResizeHandle::ALL {
        let point = handle_position(handle, bounds);
        let rect = EguiRect::from_center_size(point, vec2(8.0, 8.0));
        painter.rect_filled(rect, CornerRadius::same(1), colors.handle_fill);
        painter.rect_stroke(
            rect,
            CornerRadius::same(1),
            Stroke::new(1.2, colors.selection),
            StrokeKind::Inside,
        );
        let _ = radius;
    }
    let rotation = rotation_handle(bounds);
    let top = pos2(bounds.center().x, bounds.top());
    painter.line_segment(
        [top, rotation],
        Stroke::new(1.0, colors.selection.gamma_multiply(0.8)),
    );
    painter.circle_filled(rotation, 5.0, colors.handle_fill);
    painter.circle_stroke(rotation, 5.0, Stroke::new(1.2, colors.selection));
}

fn handle_position(handle: ResizeHandle, bounds: EguiRect) -> Pos2 {
    let center = bounds.center();
    match handle {
        ResizeHandle::NorthWest => bounds.left_top(),
        ResizeHandle::North => pos2(center.x, bounds.top()),
        ResizeHandle::NorthEast => bounds.right_top(),
        ResizeHandle::East => pos2(bounds.right(), center.y),
        ResizeHandle::SouthEast => bounds.right_bottom(),
        ResizeHandle::South => pos2(center.x, bounds.bottom()),
        ResizeHandle::SouthWest => bounds.left_bottom(),
        ResizeHandle::West => pos2(bounds.left(), center.y),
    }
}

fn hit_handle(pointer: Pos2, bounds: EguiRect) -> Option<ResizeHandle> {
    ResizeHandle::ALL
        .into_iter()
        .find(|handle| handle_position(*handle, bounds).distance(pointer) <= 7.0)
}

fn rotation_handle(bounds: EguiRect) -> Pos2 {
    pos2(bounds.center().x, bounds.top() - 22.0)
}

fn hit_rotation_handle(pointer: Pos2, bounds: EguiRect) -> bool {
    rotation_handle(bounds).distance(pointer) <= 9.0
}

fn paint_grid(painter: &egui::Painter, _rect: EguiRect, view: View, colors: theme::CanvasColors) {
    let mut spacing = 20.0_f64;
    while spacing * view.camera.zoom < 12.0 {
        spacing *= 5.0;
    }
    while spacing * view.camera.zoom > 120.0 {
        spacing /= 5.0;
    }
    let visible = view.camera.visible_world_rect(view.viewport);
    let start_x = (visible.min.x / spacing).floor() * spacing;
    let start_y = (visible.min.y / spacing).floor() * spacing;
    let mut x = start_x;
    while x <= visible.max.x {
        let screen_x = view.pos(Vec2::new(x, 0.0)).x;
        let mut y = start_y;
        while y <= visible.max.y {
            let screen_y = view.pos(Vec2::new(0.0, y)).y;
            let major =
                (x / spacing).round() as i64 % 5 == 0 && (y / spacing).round() as i64 % 5 == 0;
            let color = if major {
                colors.grid_major
            } else {
                colors.grid
            };
            painter.circle_filled(
                pos2(screen_x, screen_y),
                if major { 1.4 } else { 0.8 },
                color,
            );
            y += spacing;
        }
        x += spacing;
    }
}
