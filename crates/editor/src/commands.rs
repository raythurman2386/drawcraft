use drawcraft_command::ids;
use drawcraft_document::{ArrowHead, ArrowStyle, Dash, Fill, Rect, ShapeKind, Vec2};
use serde_json::{Value, json};

use crate::{EditError, Editor, Params, Result, StyleUpdate, Tool};

impl Editor {
    /// Execute a command by its registry id.
    ///
    /// This is the single entry point used by the UI, CLI, JSON channel, MCP
    /// server, and tests. Unknown ids are rejected explicitly so registry
    /// coverage can be asserted in tests.
    pub fn dispatch(&mut self, id: &str, params: &Value) -> Result<Value> {
        let params = Params::new(params)?;
        match id {
            // ---- History -------------------------------------------------
            ids::EDIT_UNDO => Ok(json!({"undone": self.undo()})),
            ids::EDIT_REDO => Ok(json!({"redone": self.redo()})),
            ids::EDIT_COPY => {
                self.copy_selection();
                Ok(json!({"count": self.clipboard.len()}))
            }
            ids::EDIT_CUT => {
                self.cut_selection();
                Ok(json!({"count": self.clipboard.len()}))
            }
            ids::EDIT_PASTE => {
                let ids = self.paste(Vec2::new(24.0, 24.0));
                Ok(json!({"ids": ids}))
            }
            ids::EDIT_DUPLICATE => {
                let ids = self.duplicate_selection(Vec2::new(24.0, 24.0));
                Ok(json!({"ids": ids}))
            }
            ids::EDIT_DELETE => {
                self.delete_selection();
                Ok(json!({"ok": true}))
            }
            ids::EDIT_SELECT => {
                let mut ids: Vec<drawcraft_document::ShapeId> = params
                    .get("ids")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(drawcraft_document::ShapeId::from)
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(id) = params.optional_shape_id("id") {
                    ids.push(id);
                }
                let existing = self
                    .doc
                    .shapes()
                    .iter()
                    .map(|shape| shape.id.clone())
                    .collect::<Vec<_>>();
                // Filter nonexistent ids on both paths, then expand groups.
                let filtered: Vec<_> = ids
                    .into_iter()
                    .filter(|id| existing.iter().any(|candidate| candidate == id))
                    .collect();
                let expanded = self.expand_groups(&filtered);
                if params.boolean_or("additive", false) {
                    self.selection.extend(expanded);
                } else {
                    self.selection.set_existing(expanded, &existing);
                }
                Ok(json!({"count": self.selection.len()}))
            }
            ids::EDIT_SELECT_ALL => {
                self.select_all();
                Ok(json!({"count": self.selection.len()}))
            }
            ids::EDIT_SELECT_NONE => {
                self.select_none();
                Ok(json!({"ok": true}))
            }
            ids::EDIT_SELECT_LASSO => {
                let points = parse_points_array(params.get("points"))?;
                let additive = params.boolean_or("additive", false);
                let count = self.lasso_select(&points, additive);
                Ok(json!({"count": count}))
            }
            ids::EDIT_SELECT_FIRST => {
                self.select_first();
                Ok(json!({"count": self.selection.len()}))
            }
            ids::EDIT_SELECT_LAST => {
                self.select_last();
                Ok(json!({"count": self.selection.len()}))
            }

            // ---- Transforms ---------------------------------------------
            ids::EDIT_MOVE => {
                let delta = Vec2::new(params.number("dx")?, params.number("dy")?);
                self.move_selection(delta);
                Ok(json!({"ok": true}))
            }
            ids::EDIT_RESIZE => {
                self.resize_selection(params.number("width")?, params.number("height")?);
                Ok(json!({"ok": true}))
            }
            ids::EDIT_ROTATE => {
                let degrees = params.number("degrees")?;
                self.rotate_selection(degrees.to_radians());
                Ok(json!({"ok": true}))
            }
            ids::EDIT_FLIP => {
                let axis = params.string("axis")?;
                let horizontal = parse_axis(axis)?;
                self.flip_selection(horizontal);
                Ok(json!({"ok": true}))
            }
            ids::EDIT_ALIGN => {
                self.align_selection(params.string("edge")?)?;
                Ok(json!({"ok": true}))
            }
            ids::EDIT_DISTRIBUTE => {
                self.distribute_selection(params.string("axis")?)?;
                Ok(json!({"ok": true}))
            }
            ids::EDIT_ZORDER => {
                self.zorder_selection(params.string("action")?)?;
                Ok(json!({"ok": true}))
            }
            ids::EDIT_LOCK => {
                self.lock_selection(params.boolean_or("locked", true));
                Ok(json!({"ok": true}))
            }
            ids::EDIT_GROUP => {
                let group = self.group_selection();
                Ok(json!({"group": group}))
            }
            ids::EDIT_UNGROUP => {
                self.ungroup_selection();
                Ok(json!({"ok": true}))
            }

            // ---- Shapes -------------------------------------------------
            ids::SHAPE_ADD => self.command_add_shape(&params),
            ids::SHAPE_MOVE => {
                let id = params.shape_id("id")?;
                let delta = Vec2::new(params.number("dx")?, params.number("dy")?);
                self.move_shape(&id, delta)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_RESIZE => {
                let id = params.shape_id("id")?;
                self.resize_shape(&id, params.number("width")?, params.number("height")?)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_ROTATE => {
                let id = params.shape_id("id")?;
                self.rotate_shape(&id, params.number("degrees")?.to_radians())?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_FLIP => {
                let id = params.shape_id("id")?;
                let horizontal = parse_axis(params.string("axis")?)?;
                self.flip_shape(&id, horizontal)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_SET_TEXT => {
                let id = params.shape_id("id")?;
                self.set_text(&id, params.string("text")?)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_SET_LABEL => {
                let id = params.shape_id("id")?;
                self.set_label(&id, params.string("label")?)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_SET_STYLE => {
                let id = params.shape_id("id")?;
                let update = parse_style_update(&params)?;
                self.set_style(&id, &update)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_DELETE => {
                let id = params.shape_id("id")?;
                self.delete_shape(&id)?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_LOCK => {
                let id = params.shape_id("id")?;
                self.lock_shape(&id, params.boolean_or("locked", true))?;
                Ok(json!({"ok": true}))
            }
            ids::SHAPE_BIND => self.command_bind_arrow(&params),

            // ---- Pages --------------------------------------------------
            ids::PAGE_ADD => {
                let id = self.add_page(params.optional_string("name").unwrap_or(""))?;
                Ok(json!({"id": id}))
            }
            ids::PAGE_DELETE => {
                let id = params.page_id("id")?;
                self.delete_page(&id)?;
                Ok(json!({"ok": true}))
            }
            ids::PAGE_RENAME => {
                let id = params.page_id("id")?;
                self.rename_page(&id, params.string("name")?)?;
                Ok(json!({"ok": true}))
            }
            ids::PAGE_SET_ACTIVE => {
                let id = params.page_id("id")?;
                self.set_active_page(&id)?;
                Ok(json!({"ok": true}))
            }
            ids::PAGE_DUPLICATE => {
                let id = params.page_id("id")?;
                let new_id = self.duplicate_page(&id, params.string_or("name", ""))?;
                Ok(json!({"id": new_id}))
            }
            ids::PAGE_MOVE => {
                let id = params.page_id("id")?;
                let offset = params.integer("offset")?;
                self.move_page(&id, offset as isize)?;
                Ok(json!({"ok": true}))
            }
            ids::PAGE_LIST => Ok(json!({
                "active": self.doc.active_page,
                "pages": self.doc.pages.iter().map(|page| json!({
                    "id": page.id,
                    "name": page.name,
                    "shapes": page.shapes.len(),
                })).collect::<Vec<_>>()
            })),

            // ---- View ---------------------------------------------------
            ids::VIEW_ZOOM_TO_FIT => {
                self.zoom_to_fit();
                Ok(json!({"zoom": self.camera.zoom, "center": self.camera.center}))
            }
            ids::VIEW_ZOOM_TO_SELECTION => {
                self.zoom_to_selection();
                Ok(json!({"zoom": self.camera.zoom, "center": self.camera.center}))
            }
            ids::VIEW_ZOOM_IN => {
                self.zoom_in();
                Ok(json!({"zoom": self.camera.zoom}))
            }
            ids::VIEW_ZOOM_OUT => {
                self.zoom_out();
                Ok(json!({"zoom": self.camera.zoom}))
            }
            ids::VIEW_SET_ZOOM => {
                self.set_zoom(params.number("zoom")?);
                Ok(json!({"zoom": self.camera.zoom}))
            }
            ids::VIEW_SET_DARK => {
                self.view.dark = params.get("dark").and_then(Value::as_bool).unwrap_or(false);
                Ok(json!({"dark": self.view.dark}))
            }
            ids::VIEW_SET_GRID => {
                self.view.grid = params.get("grid").and_then(Value::as_bool).unwrap_or(false);
                Ok(json!({"grid": self.view.grid}))
            }
            ids::VIEW_SET_FOCUS => {
                self.view.focus = params
                    .get("focus")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Ok(json!({"focus": self.view.focus}))
            }
            ids::VIEW_SET_MINIMAP => {
                self.view.minimap = params
                    .get("minimap")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Ok(json!({"minimap": self.view.minimap}))
            }

            // ---- Tools --------------------------------------------------
            ids::TOOL_SET => {
                let name = params.string("tool")?;
                let tool = Tool::parse(name).ok_or_else(|| EditError::InvalidParam {
                    name: "tool",
                    reason: format!("unknown tool {name}"),
                })?;
                self.tool = tool;
                Ok(json!({"tool": tool.as_str()}))
            }

            other => Err(EditError::UnknownCommand(other.to_owned())),
        }
    }

    fn command_add_shape(&mut self, params: &Params<'_>) -> Result<Value> {
        let kind_name = params.string("kind")?;
        let kind = ShapeKind::parse(kind_name).ok_or_else(|| EditError::InvalidParam {
            name: "kind",
            reason: format!("unknown shape kind {kind_name}"),
        })?;
        let x = params.number_or("x", 0.0);
        let y = params.number_or("y", 0.0);
        let width = params.number_or("width", 120.0).max(1.0);
        let height = params.number_or("height", 80.0).max(1.0);
        let text = params.optional_string("text").unwrap_or("");
        let id = self.add_shape(kind, Rect::from_xywh(x, y, width, height), text)?;
        if let Some(shape) = self.doc.shape_mut(&id) {
            if let Some(stroke) = params.optional_string("stroke") {
                shape.style.stroke = stroke.to_owned();
            }
            if let Some(fill) = params.optional_string("fill") {
                shape.style.fill_color = fill.to_owned();
                shape.style.fill = Fill::Solid;
            }
        }
        Ok(json!({"id": id}))
    }

    fn command_bind_arrow(&mut self, params: &Params<'_>) -> Result<Value> {
        let id = params.shape_id("id")?;
        if self.doc.shape(&id).is_none() {
            return Err(EditError::NotFound(id.to_string()));
        }
        let start = if params.get("start").is_some() {
            let value = params.optional_shape_id("start");
            if let Some(binding) = &value
                && self.doc.shape(binding).is_none()
            {
                return Err(EditError::NotFound(binding.to_string()));
            }
            Some(value)
        } else {
            None
        };
        let end = if params.get("end").is_some() {
            let value = params.optional_shape_id("end");
            if let Some(binding) = &value
                && self.doc.shape(binding).is_none()
            {
                return Err(EditError::NotFound(binding.to_string()));
            }
            Some(value)
        } else {
            None
        };
        let style = match params.optional_string("style") {
            Some(name) => Some(
                ArrowStyle::parse(name).ok_or_else(|| EditError::InvalidParam {
                    name: "style",
                    reason: format!("unknown arrow style {name}"),
                })?,
            ),
            None => None,
        };
        let start_head = match params.optional_string("start_head") {
            Some(name) => Some(
                ArrowHead::parse(name).ok_or_else(|| EditError::InvalidParam {
                    name: "start_head",
                    reason: format!("unknown arrow head {name}"),
                })?,
            ),
            None => None,
        };
        let end_head = match params.optional_string("end_head") {
            Some(name) => Some(
                ArrowHead::parse(name).ok_or_else(|| EditError::InvalidParam {
                    name: "end_head",
                    reason: format!("unknown arrow head {name}"),
                })?,
            ),
            None => None,
        };

        self.snapshot();
        {
            let shape = self
                .doc
                .shape_mut(&id)
                .ok_or_else(|| EditError::NotFound(id.to_string()))?;
            if shape.kind != ShapeKind::Arrow {
                return Err(EditError::InvalidState("shape is not an arrow".into()));
            }
            if let Some(start) = start {
                shape.arrow_start = start;
            }
            if let Some(end) = end {
                shape.arrow_end = end;
            }
            if let Some(style) = style {
                shape.arrow_style = style.as_str().to_owned();
            }
            if let Some(head) = start_head {
                shape.arrow_start_head = head.as_str().to_owned();
            }
            if let Some(head) = end_head {
                shape.arrow_end_head = head.as_str().to_owned();
            }
        }
        self.refresh_arrows();
        Ok(json!({"ok": true}))
    }
}

fn parse_points_array(value: Option<&Value>) -> Result<Vec<Vec2>> {
    let Some(value) = value else {
        return Err(EditError::MissingParam("points"));
    };
    let array = value.as_array().ok_or_else(|| EditError::InvalidParam {
        name: "points",
        reason: "expected an array of [x, y] pairs".into(),
    })?;
    let mut points = Vec::with_capacity(array.len());
    for entry in array {
        let pair = entry.as_array().ok_or_else(|| EditError::InvalidParam {
            name: "points",
            reason: "each point must be a [x, y] array".into(),
        })?;
        if pair.len() < 2 {
            return Err(EditError::InvalidParam {
                name: "points",
                reason: "each point needs at least two numbers".into(),
            });
        }
        let x = pair[0].as_f64().ok_or_else(|| EditError::InvalidParam {
            name: "points",
            reason: "point x must be a number".into(),
        })?;
        let y = pair[1].as_f64().ok_or_else(|| EditError::InvalidParam {
            name: "points",
            reason: "point y must be a number".into(),
        })?;
        if !x.is_finite() || !y.is_finite() {
            return Err(EditError::InvalidParam {
                name: "points",
                reason: "point coordinates must be finite".into(),
            });
        }
        points.push(Vec2::new(x, y));
    }
    Ok(points)
}

fn parse_axis(value: &str) -> Result<bool> {
    match value {
        "horizontal" | "x" => Ok(true),
        "vertical" | "y" => Ok(false),
        other => Err(EditError::InvalidParam {
            name: "axis",
            reason: format!("expected horizontal or vertical, got {other}"),
        }),
    }
}

fn parse_style_update(params: &Params<'_>) -> Result<StyleUpdate> {
    let mut update = StyleUpdate::default();
    if let Some(value) = params.optional_string("stroke") {
        update.stroke = Some(value.to_owned());
    }
    if let Some(value) = params.optional_string("fill_color") {
        update.fill_color = Some(value.to_owned());
    }
    if let Some(value) = params.get("stroke_width").and_then(Value::as_f64) {
        update.stroke_width = Some(value);
    }
    if let Some(value) = params.optional_string("fill") {
        update.fill = Some(Fill::parse(value).ok_or_else(|| EditError::InvalidParam {
            name: "fill",
            reason: format!("unknown fill {value}"),
        })?);
    }
    if let Some(value) = params.optional_string("dash") {
        update.dash = Some(Dash::parse(value).ok_or_else(|| EditError::InvalidParam {
            name: "dash",
            reason: format!("unknown dash {value}"),
        })?);
    }
    if let Some(value) = params.get("opacity").and_then(Value::as_f64) {
        update.opacity = Some(value);
    }
    if let Some(value) = params.get("font_size").and_then(Value::as_f64) {
        update.font_size = Some(value);
    }
    Ok(update)
}
