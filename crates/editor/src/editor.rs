use std::collections::{BTreeMap, HashSet};

use drawcraft_command::Registry;
use drawcraft_document::{
    ArrowHead, ArrowStyle, Document, Fill, GroupId, PageId, Rect, ShapeId, ShapeKind, ShapeRecord,
    Style, Vec2, rect_anchor,
};
use drawcraft_geometry::{Camera, rect_in_polygon};

use crate::{EditError, History, Result, Selection, Tool};

/// Persistent view preferences.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub dark: bool,
    pub grid: bool,
    pub focus: bool,
    pub minimap: bool,
    pub grid_size: f64,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            dark: false,
            grid: true,
            focus: false,
            minimap: false,
            grid_size: 20.0,
        }
    }
}

/// Incremental style patch used by the properties panel and commands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleUpdate {
    pub stroke: Option<String>,
    pub fill_color: Option<String>,
    pub stroke_width: Option<f64>,
    pub fill: Option<Fill>,
    pub dash: Option<drawcraft_document::Dash>,
    pub opacity: Option<f64>,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
}

impl StyleUpdate {
    pub fn apply(&self, style: &mut Style) {
        if let Some(value) = &self.stroke {
            style.stroke = value.clone();
        }
        if let Some(value) = &self.fill_color {
            style.fill_color = value.clone();
        }
        if let Some(value) = self.stroke_width {
            style.stroke_width = value;
        }
        if let Some(value) = self.fill {
            style.fill = value;
        }
        if let Some(value) = self.dash {
            style.dash = value;
        }
        if let Some(value) = self.opacity {
            style.opacity = value;
        }
        if let Some(value) = self.font_size {
            style.font_size = value;
        }
        if let Some(value) = &self.font_family {
            style.font_family = value.clone();
        }
    }
}

/// The headless editor.
#[derive(Clone, Debug)]
pub struct Editor {
    pub doc: Document,
    pub camera: Camera,
    pub selection: Selection,
    pub history: History,
    pub clipboard: Vec<ShapeRecord>,
    pub tool: Tool,
    pub view: ViewState,
    pub registry: Registry,
    /// Canvas size in logical pixels; command-driven zoom uses this.
    pub viewport: Vec2,
    /// Set after loading so the first real viewport can fit the page.
    pub fit_requested: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            doc: Document::new(),
            camera: Camera::new(),
            selection: Selection::new(),
            history: History::new(200),
            clipboard: Vec::new(),
            tool: Tool::Select,
            view: ViewState::default(),
            registry: Registry::new(),
            viewport: Vec2::new(1280.0, 800.0),
            fit_requested: false,
        }
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn undo(&mut self) -> bool {
        let changed = self.history.undo(&mut self.doc);
        if changed {
            self.prune_selection();
        }
        changed
    }

    pub fn redo(&mut self) -> bool {
        let changed = self.history.redo(&mut self.doc);
        if changed {
            self.prune_selection();
        }
        changed
    }

    /// Discard the current document and start over.
    pub fn reset(&mut self) {
        self.doc = Document::new();
        self.camera = Camera::new();
        self.selection.clear();
        self.history.clear();
        self.clipboard.clear();
        self.tool = Tool::Select;
    }

    /// Replace the document (used by file open). Resets undo history.
    pub fn replace_document(&mut self, document: Document) -> Result<()> {
        document.validate()?;
        self.doc = document;
        self.selection.clear();
        self.history.clear();
        self.camera = Camera::new();
        self.fit_requested = true;
        Ok(())
    }

    pub fn snapshot(&mut self) {
        self.history.record(&self.doc);
    }

    pub fn next_shape_id(&self) -> ShapeId {
        self.doc.next_shape_id()
    }

    fn next_group_id(&self) -> GroupId {
        let used: HashSet<&str> = self
            .doc
            .shapes()
            .iter()
            .filter_map(|shape| shape.group_id.as_ref().map(GroupId::as_str))
            .collect();
        let mut index = self.doc.shape_count() as u64 + 1;
        loop {
            let candidate = GroupId::generated(index);
            if !used.contains(candidate.as_str()) {
                return candidate;
            }
            index = index.saturating_add(1);
        }
    }

    fn next_z(&self) -> i32 {
        self.doc
            .shapes()
            .iter()
            .map(|shape| shape.z)
            .max()
            .unwrap_or(-1)
            .saturating_add(1)
    }

    // ---- Shape creation -------------------------------------------------

    pub fn add_shape(
        &mut self,
        kind: ShapeKind,
        rect: Rect,
        text: impl Into<String>,
    ) -> Result<ShapeId> {
        self.snapshot();
        let id = self.next_shape_id();
        let mut shape = ShapeRecord::new(id.clone(), kind, rect.min, rect.size());
        shape.z = self.next_z();
        shape.text = text.into();
        shape.style = default_style_for(kind, self.view.dark);
        if kind.is_linear() {
            shape.points = vec![rect.min, Vec2::new(rect.max.x, rect.max.y)];
            shape.sync_rect_from_points();
        }
        self.doc.add_shape(shape)?;
        self.selection.select(id.clone());
        Ok(id)
    }

    pub fn insert_shape(&mut self, mut shape: ShapeRecord) -> Result<ShapeId> {
        self.snapshot();
        if shape.id.is_empty() || self.doc.shape(&shape.id).is_some() {
            shape.id = self.next_shape_id();
        }
        if shape.z == 0 {
            shape.z = self.next_z();
        }
        let id = shape.id.clone();
        self.doc.add_shape(shape)?;
        self.selection.select(id.clone());
        Ok(id)
    }

    /// Add an arrow and optionally bind each endpoint to the topmost bindable shape.
    pub fn add_arrow_between(&mut self, start: Vec2, end: Vec2, bind: bool) -> Result<ShapeId> {
        let start_target = bind.then(|| self.topmost_bindable_at(start, 1.0)).flatten();
        let end_target = bind.then(|| self.topmost_bindable_at(end, 1.0)).flatten();
        let id = self.next_shape_id();
        let mut shape = ShapeRecord::new(
            id.clone(),
            ShapeKind::Arrow,
            start,
            Vec2::new(
                (end.x - start.x).abs().max(1.0),
                (end.y - start.y).abs().max(1.0),
            ),
        );
        shape.points = vec![start, end];
        shape.sync_rect_from_points();
        shape.z = self.next_z();
        shape.style = default_style_for(ShapeKind::Arrow, self.view.dark);
        shape.arrow_start = start_target;
        shape.arrow_end = end_target;
        self.snapshot();
        self.doc.add_shape(shape)?;
        self.selection.select(id.clone());
        self.refresh_arrows();
        Ok(id)
    }

    // ---- Selection ------------------------------------------------------

    pub fn selected_shapes(&self) -> Vec<&ShapeRecord> {
        let page = match self.doc.active() {
            Some(page) => page,
            None => return Vec::new(),
        };
        self.selection
            .iter()
            .filter_map(|id| page.shape(id))
            .collect()
    }

    pub fn selection_bounds(&self) -> Option<Rect> {
        self.doc.selection_bounds(self.selection.ids())
    }

    pub fn select_all(&mut self) {
        if let Some(page) = self.doc.active() {
            self.selection.set_existing(
                page.shapes
                    .iter()
                    .filter(|shape| !shape.locked)
                    .map(|shape| shape.id.clone()),
                &page
                    .shapes
                    .iter()
                    .map(|shape| shape.id.clone())
                    .collect::<Vec<_>>(),
            );
        }
    }

    pub fn select_none(&mut self) {
        self.selection.clear();
    }

    pub fn prune_selection(&mut self) {
        let existing: Vec<ShapeId> = self
            .doc
            .shapes()
            .iter()
            .map(|shape| shape.id.clone())
            .collect();
        self.selection.retain_existing(&existing);
    }

    /// Topmost unlocked shape under a world-space point.
    #[must_use]
    pub fn topmost_at(&self, point: Vec2, tolerance: f64) -> Option<ShapeId> {
        self.doc
            .active()
            .into_iter()
            .flat_map(|page| page.shapes.iter())
            .filter(|shape| !shape.locked)
            .filter(|shape| shape.hit_test(point, tolerance))
            .max_by_key(|shape| (shape.z, &shape.id))
            .map(|shape| shape.id.clone())
    }

    /// Topmost unlocked non-linear shape under a point (arrow bind / snap target).
    ///
    /// Linear shapes (arrows, lines, draw strokes, polylines) are skipped so
    /// preview snap and commit binding agree: binding uses edge hit-tests that
    /// rarely land on a linear bbox edge the preview would otherwise snap to.
    #[must_use]
    pub fn topmost_bindable_at(&self, point: Vec2, tolerance: f64) -> Option<ShapeId> {
        self.doc
            .active()
            .into_iter()
            .flat_map(|page| page.shapes.iter())
            .filter(|shape| !shape.locked)
            .filter(|shape| !shape.kind.is_linear())
            .filter(|shape| shape.hit_test(point, tolerance))
            .max_by_key(|shape| (shape.z, &shape.id))
            .map(|shape| shape.id.clone())
    }

    /// Snap arrow drag endpoints the same way [`Self::refresh_arrows`] does once
    /// bindings are known (center-to-center when both ends bind).
    #[must_use]
    pub fn arrow_endpoints_for_drag(&self, start: Vec2, end: Vec2, tolerance: f64) -> (Vec2, Vec2) {
        let start_rect = self
            .topmost_bindable_at(start, tolerance)
            .and_then(|id| self.doc.shape(&id).map(ShapeRecord::rect));
        let end_rect = self
            .topmost_bindable_at(end, tolerance)
            .and_then(|id| self.doc.shape(&id).map(ShapeRecord::rect));
        resolve_arrow_endpoints(start, end, start_rect, end_rect)
    }

    /// Select what is under the pointer, expanding to the whole group.
    ///
    /// Additive (shift) toggles the hit unit at group level: grouped shapes are
    /// an atomic unit, so shift-clicking a selected group deselects every member.
    pub fn select_at(&mut self, point: Vec2, additive: bool, tolerance: f64) -> Option<ShapeId> {
        let Some(id) = self.topmost_at(point, tolerance) else {
            if !additive {
                self.selection.clear();
            }
            return None;
        };
        self.apply_region_selection(std::slice::from_ref(&id), additive);
        Some(id)
    }

    fn all_ids(&self) -> Vec<ShapeId> {
        self.doc
            .shapes()
            .iter()
            .map(|shape| shape.id.clone())
            .collect()
    }

    /// Expand any touched group id to every member. Ungrouped ids pass through.
    ///
    /// Used by click, marquee, lasso, and `edit.select` so groups stay atomic.
    #[must_use]
    pub fn expand_groups(&self, ids: &[ShapeId]) -> Vec<ShapeId> {
        let mut out = Vec::new();
        let mut seen_ids = HashSet::new();
        let mut seen_groups = HashSet::new();
        for id in ids {
            let Some(shape) = self.doc.shape(id) else {
                continue;
            };
            if let Some(group) = &shape.group_id {
                if !seen_groups.insert(group.clone()) {
                    continue;
                }
                for member in self.doc.group_members(group) {
                    if seen_ids.insert(member.id.clone()) {
                        out.push(member.id.clone());
                    }
                }
            } else if seen_ids.insert(id.clone()) {
                out.push(id.clone());
            }
        }
        out
    }

    /// Apply a region hit as either replace-with-expanded or group-level toggle.
    fn apply_region_selection(&mut self, raw_ids: &[ShapeId], additive: bool) {
        if !additive {
            let expanded = self.expand_groups(raw_ids);
            let all = self.all_ids();
            self.selection.set_existing(expanded, &all);
            return;
        }
        self.toggle_units(raw_ids);
    }

    /// Toggle each atomic unit (group or ungrouped shape) among `raw_ids`.
    fn toggle_units(&mut self, raw_ids: &[ShapeId]) {
        let mut seen_groups = HashSet::new();
        let mut seen_singles = HashSet::new();
        for id in raw_ids {
            let Some(shape) = self.doc.shape(id) else {
                continue;
            };
            if let Some(group) = &shape.group_id {
                if !seen_groups.insert(group.clone()) {
                    continue;
                }
                let members: Vec<ShapeId> = self
                    .doc
                    .group_members(group)
                    .into_iter()
                    .map(|shape| shape.id.clone())
                    .collect();
                let all_selected = members.iter().all(|member| self.selection.contains(member));
                if all_selected {
                    for member in &members {
                        self.selection.remove(member);
                    }
                } else {
                    self.selection.extend(members);
                }
            } else if seen_singles.insert(id.clone()) {
                self.selection.toggle(id.clone());
            }
        }
    }

    /// Marquee / brush selection. Touch-to-select; expands groups; shift toggles units.
    pub fn brush_select(&mut self, rect: Rect, additive: bool) -> usize {
        let ids: Vec<ShapeId> = self
            .doc
            .shapes()
            .iter()
            .filter(|shape| !shape.locked && rect.intersects(shape.world_bounds()))
            .map(|shape| shape.id.clone())
            .collect();
        self.apply_region_selection(&ids, additive);
        self.selection.len()
    }

    /// Lasso selection. Fully-contained bounds only; expands groups; shift toggles units.
    pub fn lasso_select(&mut self, points: &[Vec2], additive: bool) -> usize {
        if points.len() < 3 {
            if !additive {
                self.selection.clear();
            }
            return self.selection.len();
        }
        let ids: Vec<ShapeId> = self
            .doc
            .shapes()
            .iter()
            .filter(|shape| !shape.locked && rect_in_polygon(shape.world_bounds(), points))
            .map(|shape| shape.id.clone())
            .collect();
        self.apply_region_selection(&ids, additive);
        self.selection.len()
    }

    /// Select the bottommost unlocked shape by z-order (then expand its group).
    pub fn select_first(&mut self) {
        let id = self
            .doc
            .shapes()
            .iter()
            .filter(|shape| !shape.locked)
            .min_by_key(|shape| (shape.z, &shape.id))
            .map(|shape| shape.id.clone());
        match id {
            Some(id) => self.apply_region_selection(std::slice::from_ref(&id), false),
            None => self.selection.clear(),
        }
    }

    /// Select the topmost unlocked shape by z-order (then expand its group).
    pub fn select_last(&mut self) {
        let id = self
            .doc
            .shapes()
            .iter()
            .filter(|shape| !shape.locked)
            .max_by_key(|shape| (shape.z, &shape.id))
            .map(|shape| shape.id.clone());
        match id {
            Some(id) => self.apply_region_selection(std::slice::from_ref(&id), false),
            None => self.selection.clear(),
        }
    }

    // ---- Editing operations --------------------------------------------

    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.snapshot();
        let removed: HashSet<ShapeId> = self.selection.iter().cloned().collect();
        let page = match self.doc.active_mut() {
            Some(page) => page,
            None => return,
        };
        page.shapes.retain(|shape| !removed.contains(&shape.id));
        for shape in &mut page.shapes {
            if let Some(frame) = &shape.parent_id
                && removed.contains(frame)
            {
                shape.parent_id = None;
            }
        }
        self.ungroup_orphans();
        self.clear_dangling_arrows(&removed);
        self.selection.clear();
        self.refresh_arrows();
    }

    fn clear_dangling_arrows(&mut self, removed: &HashSet<ShapeId>) {
        let Some(page) = self.doc.active_mut() else {
            return;
        };
        for shape in &mut page.shapes {
            if shape
                .arrow_start
                .as_ref()
                .is_some_and(|id| removed.contains(id))
            {
                shape.arrow_start = None;
            }
            if shape
                .arrow_end
                .as_ref()
                .is_some_and(|id| removed.contains(id))
            {
                shape.arrow_end = None;
            }
        }
    }

    fn ungroup_orphans(&mut self) {
        let Some(page) = self.doc.active() else {
            return;
        };
        let mut counts: BTreeMap<GroupId, usize> = BTreeMap::new();
        for shape in &page.shapes {
            if let Some(group) = &shape.group_id {
                *counts.entry(group.clone()).or_insert(0) += 1;
            }
        }
        let singleton: HashSet<GroupId> = counts
            .into_iter()
            .filter_map(|(group, count)| (count < 2).then_some(group))
            .collect();
        if singleton.is_empty() {
            return;
        }
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if shape
                    .group_id
                    .as_ref()
                    .is_some_and(|group| singleton.contains(group))
                {
                    shape.group_id = None;
                }
            }
        }
    }

    pub fn duplicate_selection(&mut self, offset: Vec2) -> Vec<ShapeId> {
        let selected: Vec<ShapeRecord> = self.selected_shapes().into_iter().cloned().collect();
        self.clone_shapes(selected, offset, false)
    }

    pub fn copy_selection(&mut self) {
        self.clipboard = self.selected_shapes().into_iter().cloned().collect();
    }

    pub fn cut_selection(&mut self) {
        self.copy_selection();
        self.delete_selection();
    }

    pub fn paste(&mut self, offset: Vec2) -> Vec<ShapeId> {
        let clipboard = self.clipboard.clone();
        // Pasting from the clipboard must not retain bindings to ids that are
        // no longer on this page; duplicated shapes may keep valid originals.
        self.clone_shapes(clipboard, offset, true)
    }

    fn clone_shapes(
        &mut self,
        selected: Vec<ShapeRecord>,
        offset: Vec2,
        clear_missing_bindings: bool,
    ) -> Vec<ShapeId> {
        if selected.is_empty() {
            return Vec::new();
        }
        self.snapshot();
        let mut used: HashSet<String> = self
            .doc
            .shapes()
            .iter()
            .map(|shape| shape.id.as_str().to_owned())
            .collect();
        let mut id_map: BTreeMap<ShapeId, ShapeId> = BTreeMap::new();
        let mut planned: Vec<(ShapeId, ShapeRecord)> = Vec::with_capacity(selected.len());
        let mut next = self.doc.shape_count() as u64 + 1;
        for source in selected {
            let id = loop {
                let candidate = ShapeId::generated(next);
                next = next.saturating_add(1);
                if !used.contains(candidate.as_str()) {
                    used.insert(candidate.as_str().to_owned());
                    break candidate;
                }
            };
            id_map.insert(source.id.clone(), id.clone());
            planned.push((id, source));
        }

        let mut group_map: BTreeMap<GroupId, GroupId> = BTreeMap::new();
        let mut ids = Vec::with_capacity(planned.len());
        let mut z = self.next_z();
        for (id, mut shape) in planned {
            shape.id = id.clone();
            if let Some(group) = &shape.group_id {
                let mapped = group_map
                    .entry(group.clone())
                    .or_insert_with(|| self.next_group_id());
                shape.group_id = Some(mapped.clone());
            }
            if let Some(start) = &shape.arrow_start {
                shape.arrow_start = id_map.get(start).cloned().or_else(|| {
                    (!clear_missing_bindings && self.doc.shape(start).is_some())
                        .then(|| start.clone())
                });
            }
            if let Some(end) = &shape.arrow_end {
                shape.arrow_end = id_map.get(end).cloned().or_else(|| {
                    (!clear_missing_bindings && self.doc.shape(end).is_some()).then(|| end.clone())
                });
            }
            if let Some(parent) = &shape.parent_id {
                shape.parent_id = id_map.get(parent).cloned().or_else(|| {
                    (!clear_missing_bindings && self.doc.shape(parent).is_some())
                        .then(|| parent.clone())
                });
            }
            shape.apply_translation(offset);
            shape.z = z;
            z = z.saturating_add(1);
            if self.doc.add_shape(shape).is_ok() {
                ids.push(id);
            }
        }
        self.selection.set_existing(ids.clone(), &self.all_ids());
        self.refresh_arrows();
        ids
    }

    pub fn move_selection(&mut self, delta: Vec2) {
        if self.selection.is_empty()
            || (delta.x.abs() < f64::EPSILON && delta.y.abs() < f64::EPSILON)
        {
            return;
        }
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if ids.contains(&shape.id) {
                    shape.apply_translation(delta);
                }
            }
        }
        self.refresh_arrows();
    }

    pub fn resize_selection(&mut self, width: f64, height: f64) {
        let Some(bounds) = self.selection_bounds() else {
            return;
        };
        if bounds.is_empty() || !width.is_finite() || !height.is_finite() {
            return;
        }
        let scale = Vec2::new(
            (width.max(1.0) / bounds.width()).max(0.01),
            (height.max(1.0) / bounds.height()).max(0.01),
        );
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if ids.contains(&shape.id) {
                    shape.scale_from(bounds.min, scale);
                }
            }
        }
        self.refresh_arrows();
    }

    pub fn rotate_selection(&mut self, radians: f64) {
        if self.selection.is_empty() || !radians.is_finite() {
            return;
        }
        let Some(bounds) = self.selection_bounds() else {
            return;
        };
        let pivot = bounds.center();
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if !ids.contains(&shape.id) {
                    continue;
                }
                let center = shape.center();
                let (sin, cos) = radians.sin_cos();
                let delta = center - pivot;
                let rotated = Vec2::new(
                    pivot.x + delta.x * cos - delta.y * sin,
                    pivot.y + delta.x * sin + delta.y * cos,
                );
                shape.apply_translation(rotated - center);
                shape.rotate_by(radians);
            }
        }
        self.refresh_arrows();
    }

    pub fn flip_selection(&mut self, horizontal: bool) {
        let Some(bounds) = self.selection_bounds() else {
            return;
        };
        let pivot = bounds.center();
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if !ids.contains(&shape.id) {
                    continue;
                }
                let center = shape.center();
                let mirrored = Vec2::new(
                    if horizontal {
                        2.0 * pivot.x - center.x
                    } else {
                        center.x
                    },
                    if horizontal {
                        center.y
                    } else {
                        2.0 * pivot.y - center.y
                    },
                );
                shape.apply_translation(mirrored - center);
                shape.flip_axis(horizontal);
            }
        }
        self.refresh_arrows();
    }

    pub fn align_selection(&mut self, edge: &str) -> Result<()> {
        let Some(bounds) = self.selection_bounds() else {
            return Ok(());
        };
        let ids = self.selection.ids().to_vec();
        self.snapshot();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if !ids.contains(&shape.id) {
                    continue;
                }
                let current = shape.world_bounds();
                let delta = match edge {
                    "left" => Vec2::new(bounds.min.x - current.min.x, 0.0),
                    "center" | "centre" => Vec2::new(bounds.center().x - current.center().x, 0.0),
                    "right" => Vec2::new(bounds.max.x - current.max.x, 0.0),
                    "top" => Vec2::new(0.0, bounds.min.y - current.min.y),
                    "middle" | "center_v" => Vec2::new(0.0, bounds.center().y - current.center().y),
                    "bottom" => Vec2::new(0.0, bounds.max.y - current.max.y),
                    other => {
                        return Err(EditError::InvalidParam {
                            name: "edge",
                            reason: format!("unknown edge {other}"),
                        });
                    }
                };
                shape.apply_translation(delta);
            }
        }
        self.refresh_arrows();
        Ok(())
    }

    pub fn distribute_selection(&mut self, axis: &str) -> Result<()> {
        let mut entries: Vec<(ShapeId, Rect)> = self
            .selected_shapes()
            .into_iter()
            .map(|shape| (shape.id.clone(), shape.world_bounds()))
            .collect();
        if entries.len() < 3 {
            return Ok(());
        }
        match axis {
            "horizontal" => entries.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x)),
            "vertical" => entries.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y)),
            other => {
                return Err(EditError::InvalidParam {
                    name: "axis",
                    reason: format!("unknown axis {other}"),
                });
            }
        }
        let horizontal = axis == "horizontal";
        let start = if horizontal {
            entries[0].1.min.x
        } else {
            entries[0].1.min.y
        };
        let end = if horizontal {
            entries[entries.len() - 1].1.max.x
        } else {
            entries[entries.len() - 1].1.max.y
        };
        let total_size: f64 = entries
            .iter()
            .map(|(_, rect)| {
                if horizontal {
                    rect.width()
                } else {
                    rect.height()
                }
            })
            .sum();
        let gap = (end - start - total_size) / (entries.len() - 1) as f64;
        self.snapshot();
        let mut cursor = start;
        if let Some(page) = self.doc.active_mut() {
            for (id, rect) in &entries {
                if let Some(shape) = page.shape_mut(id) {
                    let current = if horizontal { rect.min.x } else { rect.min.y };
                    let delta = cursor - current;
                    shape.apply_translation(if horizontal {
                        Vec2::new(delta, 0.0)
                    } else {
                        Vec2::new(0.0, delta)
                    });
                }
                cursor += if horizontal {
                    rect.width()
                } else {
                    rect.height()
                } + gap;
            }
        }
        self.refresh_arrows();
        Ok(())
    }

    pub fn zorder_selection(&mut self, action: &str) -> Result<()> {
        let Some(page) = self.doc.active() else {
            return Ok(());
        };
        let selected: HashSet<ShapeId> = self.selection.iter().cloned().collect();
        if selected.is_empty() {
            return Ok(());
        }
        let mut order: Vec<ShapeId> = page
            .shapes_sorted_by_z()
            .iter()
            .map(|shape| shape.id.clone())
            .collect();
        match action {
            "front" => {
                let moved: Vec<ShapeId> = order
                    .iter()
                    .filter(|id| selected.contains(*id))
                    .cloned()
                    .collect();
                order.retain(|id| !selected.contains(id));
                order.extend(moved);
            }
            "back" => {
                let moved: Vec<ShapeId> = order
                    .iter()
                    .filter(|id| selected.contains(*id))
                    .cloned()
                    .collect();
                order.retain(|id| !selected.contains(id));
                for (index, id) in moved.into_iter().enumerate() {
                    order.insert(index, id);
                }
            }
            "forward" => {
                for index in (0..order.len().saturating_sub(1)).rev() {
                    if selected.contains(&order[index]) && !selected.contains(&order[index + 1]) {
                        order.swap(index, index + 1);
                    }
                }
            }
            "backward" => {
                for index in 1..order.len() {
                    if selected.contains(&order[index]) && !selected.contains(&order[index - 1]) {
                        order.swap(index, index - 1);
                    }
                }
            }
            other => {
                return Err(EditError::InvalidParam {
                    name: "action",
                    reason: format!("unknown z-order action {other}"),
                });
            }
        }
        self.snapshot();
        if let Some(page) = self.doc.active_mut() {
            for (index, id) in order.iter().enumerate() {
                if let Some(shape) = page.shape_mut(id) {
                    shape.z = index as i32;
                }
            }
        }
        Ok(())
    }

    pub fn lock_selection(&mut self, locked: bool) {
        if self.selection.is_empty() {
            return;
        }
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if ids.contains(&shape.id) {
                    shape.locked = locked;
                }
            }
        }
        if locked {
            self.selection.clear();
        }
    }

    pub fn group_selection(&mut self) -> Option<GroupId> {
        if self.selection.len() < 2 {
            return None;
        }
        let group = self.next_group_id();
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if ids.contains(&shape.id) {
                    shape.group_id = Some(group.clone());
                }
            }
        }
        Some(group)
    }

    pub fn ungroup_selection(&mut self) {
        let groups: HashSet<GroupId> = self
            .selected_shapes()
            .into_iter()
            .filter_map(|shape| shape.group_id.clone())
            .collect();
        if groups.is_empty() {
            return;
        }
        self.snapshot();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if shape
                    .group_id
                    .as_ref()
                    .is_some_and(|group| groups.contains(group))
                {
                    shape.group_id = None;
                }
            }
        }
    }

    pub fn set_text(&mut self, id: &ShapeId, text: impl Into<String>) -> Result<()> {
        let text = text.into();
        let text = text.chars().take(100_000).collect::<String>();
        self.snapshot();
        let shape = self
            .doc
            .shape_mut(id)
            .ok_or_else(|| EditError::NotFound(id.to_string()))?;
        shape.text = text;
        Ok(())
    }

    pub fn set_label(&mut self, id: &ShapeId, label: impl Into<String>) -> Result<()> {
        let label = label.into();
        let label = label.chars().take(1_000).collect::<String>();
        self.snapshot();
        let shape = self
            .doc
            .shape_mut(id)
            .ok_or_else(|| EditError::NotFound(id.to_string()))?;
        shape.label = label;
        Ok(())
    }

    pub fn set_style(&mut self, id: &ShapeId, update: &StyleUpdate) -> Result<()> {
        self.snapshot();
        let shape = self
            .doc
            .shape_mut(id)
            .ok_or_else(|| EditError::NotFound(id.to_string()))?;
        let mut style = shape.style.clone();
        update.apply(&mut style);
        style.validate().map_err(|reason| EditError::InvalidParam {
            name: "style",
            reason,
        })?;
        shape.style = style;
        Ok(())
    }

    pub fn set_style_selection(&mut self, update: &StyleUpdate) {
        if self.selection.is_empty() {
            return;
        }
        self.snapshot();
        let ids = self.selection.ids().to_vec();
        if let Some(page) = self.doc.active_mut() {
            for shape in &mut page.shapes {
                if ids.contains(&shape.id) {
                    update.apply(&mut shape.style);
                }
            }
        }
    }

    // ---- Single-shape operations ---------------------------------------

    pub fn move_shape(&mut self, id: &ShapeId, delta: Vec2) -> Result<()> {
        if self.doc.shape(id).is_none() {
            return Err(EditError::NotFound(id.to_string()));
        }
        self.snapshot();
        if let Some(shape) = self.doc.shape_mut(id) {
            shape.apply_translation(delta);
        }
        self.refresh_arrows();
        Ok(())
    }

    pub fn resize_shape(&mut self, id: &ShapeId, width: f64, height: f64) -> Result<()> {
        let shape = self
            .doc
            .shape(id)
            .ok_or_else(|| EditError::NotFound(id.to_string()))?;
        let current = shape.rect();
        if current.is_empty() || !width.is_finite() || !height.is_finite() {
            return Err(EditError::InvalidParam {
                name: "size",
                reason: "width and height must be positive finite numbers".into(),
            });
        }
        let scale = Vec2::new(
            (width.max(1.0) / current.width()).max(0.001),
            (height.max(1.0) / current.height()).max(0.001),
        );
        self.snapshot();
        if let Some(shape) = self.doc.shape_mut(id) {
            shape.scale_from(current.min, scale);
        }
        self.refresh_arrows();
        Ok(())
    }

    pub fn rotate_shape(&mut self, id: &ShapeId, radians: f64) -> Result<()> {
        if self.doc.shape(id).is_none() {
            return Err(EditError::NotFound(id.to_string()));
        }
        self.snapshot();
        if let Some(shape) = self.doc.shape_mut(id) {
            shape.rotate_by(radians);
        }
        self.refresh_arrows();
        Ok(())
    }

    pub fn flip_shape(&mut self, id: &ShapeId, horizontal: bool) -> Result<()> {
        if self.doc.shape(id).is_none() {
            return Err(EditError::NotFound(id.to_string()));
        }
        self.snapshot();
        if let Some(shape) = self.doc.shape_mut(id) {
            shape.flip_axis(horizontal);
        }
        self.refresh_arrows();
        Ok(())
    }

    pub fn lock_shape(&mut self, id: &ShapeId, locked: bool) -> Result<()> {
        if self.doc.shape(id).is_none() {
            return Err(EditError::NotFound(id.to_string()));
        }
        self.snapshot();
        if let Some(shape) = self.doc.shape_mut(id) {
            shape.locked = locked;
        }
        if locked {
            self.selection.remove(id);
        }
        Ok(())
    }

    pub fn delete_shape(&mut self, id: &ShapeId) -> Result<()> {
        if self.doc.shape(id).is_none() {
            return Err(EditError::NotFound(id.to_string()));
        }
        self.selection.select(id.clone());
        self.delete_selection();
        Ok(())
    }

    // ---- Arrow binding --------------------------------------------------

    /// Recompute bound arrow endpoints and routing. Call after moving,
    /// resizing, rotating, or deleting shapes.
    pub fn refresh_arrows(&mut self) {
        let Some(page) = self.doc.active() else {
            return;
        };
        let anchors: BTreeMap<ShapeId, (Rect, Vec2)> = page
            .shapes
            .iter()
            .map(|shape| (shape.id.clone(), (shape.rect(), shape.center())))
            .collect();
        let mut updates = Vec::new();
        for shape in &page.shapes {
            if shape.kind != ShapeKind::Arrow {
                continue;
            }
            let (mut start, mut end) = shape.endpoints();
            if shape.points.len() >= 2 {
                start = shape.points[0];
                end = shape.points[shape.points.len() - 1];
            }
            let start_rect = shape
                .arrow_start
                .as_ref()
                .and_then(|id| anchors.get(id))
                .map(|(rect, _)| *rect);
            let end_rect = shape
                .arrow_end
                .as_ref()
                .and_then(|id| anchors.get(id))
                .map(|(rect, _)| *rect);
            (start, end) = resolve_arrow_endpoints(start, end, start_rect, end_rect);
            let points = match shape.arrow_style() {
                ArrowStyle::Straight => vec![start, end],
                ArrowStyle::Bent => {
                    let mid = (start + end) * 0.5;
                    let offset =
                        (end - start).perp().normalized() * ((end - start).length() * 0.18);
                    vec![start, mid + offset, end]
                }
                ArrowStyle::Elbow => {
                    let dx = (end.x - start.x).abs();
                    let dy = (end.y - start.y).abs();
                    if dx >= dy {
                        vec![start, Vec2::new(end.x, start.y), end]
                    } else {
                        vec![start, Vec2::new(start.x, end.y), end]
                    }
                }
            };
            updates.push((shape.id.clone(), points));
        }
        if let Some(page) = self.doc.active_mut() {
            for (id, points) in updates {
                if let Some(shape) = page.shape_mut(&id) {
                    shape.set_points(points);
                }
            }
        }
    }

    pub fn bind_arrow(
        &mut self,
        id: &ShapeId,
        start: Option<ShapeId>,
        end: Option<ShapeId>,
    ) -> Result<()> {
        let exists = |editor: &Self, binding: &Option<ShapeId>| {
            binding
                .as_ref()
                .is_none_or(|binding| editor.doc.shape(binding).is_some())
        };
        if !exists(self, &start) {
            return Err(EditError::NotFound(
                start.map(|id| id.to_string()).unwrap_or_default(),
            ));
        }
        if !exists(self, &end) {
            return Err(EditError::NotFound(
                end.map(|id| id.to_string()).unwrap_or_default(),
            ));
        }
        self.snapshot();
        let shape = self
            .doc
            .shape_mut(id)
            .ok_or_else(|| EditError::NotFound(id.to_string()))?;
        if shape.kind != ShapeKind::Arrow {
            return Err(EditError::InvalidState("shape is not an arrow".into()));
        }
        shape.arrow_start = start;
        shape.arrow_end = end;
        self.refresh_arrows();
        Ok(())
    }

    pub fn set_arrow_style(
        &mut self,
        id: &ShapeId,
        style: Option<ArrowStyle>,
        start_head: Option<ArrowHead>,
        end_head: Option<ArrowHead>,
    ) -> Result<()> {
        self.snapshot();
        let shape = self
            .doc
            .shape_mut(id)
            .ok_or_else(|| EditError::NotFound(id.to_string()))?;
        if shape.kind != ShapeKind::Arrow {
            return Err(EditError::InvalidState("shape is not an arrow".into()));
        }
        if let Some(style) = style {
            shape.arrow_style = style.as_str().into();
        }
        if let Some(head) = start_head {
            shape.arrow_start_head = head.as_str().into();
        }
        if let Some(head) = end_head {
            shape.arrow_end_head = head.as_str().into();
        }
        self.refresh_arrows();
        Ok(())
    }

    // ---- Pages ----------------------------------------------------------

    pub fn add_page(&mut self, name: impl Into<String>) -> Result<PageId> {
        self.snapshot();
        let id = self.doc.add_page(name)?;
        self.doc.set_active_page(&id)?;
        self.selection.clear();
        self.zoom_to_fit();
        Ok(id)
    }

    pub fn delete_page(&mut self, id: &PageId) -> Result<()> {
        self.snapshot();
        self.doc.remove_page(id)?;
        self.selection.clear();
        self.zoom_to_fit();
        Ok(())
    }

    pub fn rename_page(&mut self, id: &PageId, name: impl Into<String>) -> Result<()> {
        self.snapshot();
        self.doc.rename_page(id, name)?;
        Ok(())
    }

    pub fn set_active_page(&mut self, id: &PageId) -> Result<()> {
        self.doc.set_active_page(id)?;
        self.selection.clear();
        self.zoom_to_fit();
        Ok(())
    }

    pub fn duplicate_page(&mut self, id: &PageId, name: impl Into<String>) -> Result<PageId> {
        self.snapshot();
        let new_id = self.doc.duplicate_page(id, name)?;
        self.doc.set_active_page(&new_id)?;
        self.selection.clear();
        self.zoom_to_fit();
        Ok(new_id)
    }

    pub fn move_page(&mut self, id: &PageId, offset: isize) -> Result<()> {
        self.snapshot();
        self.doc.move_page(id, offset)?;
        Ok(())
    }

    // ---- Camera ---------------------------------------------------------

    pub fn set_viewport(&mut self, viewport: Vec2) {
        if viewport.x > 1.0 && viewport.y > 1.0 && viewport.is_finite() {
            self.viewport = viewport;
            if self.fit_requested {
                self.fit_requested = false;
                self.zoom_to_fit();
            }
        }
    }

    pub fn zoom_to_fit(&mut self) {
        self.fit_requested = false;
        let bounds = self
            .doc
            .active()
            .and_then(|page| page.bounds())
            .unwrap_or_else(|| Rect::from_xywh(-200.0, -150.0, 400.0, 300.0));
        self.camera.zoom_to_fit(bounds, self.viewport, 0.1);
    }

    pub fn zoom_to_selection(&mut self) {
        if let Some(bounds) = self.selection_bounds() {
            self.camera.zoom_to_fit(bounds, self.viewport, 0.2);
        } else {
            self.zoom_to_fit();
        }
    }

    pub fn zoom_in(&mut self) {
        let center = self.viewport * 0.5;
        self.camera.zoom_at(center, self.viewport, 1.25);
    }

    pub fn zoom_out(&mut self) {
        let center = self.viewport * 0.5;
        self.camera.zoom_at(center, self.viewport, 0.8);
    }

    pub fn set_zoom(&mut self, zoom: f64) {
        let center = self.viewport * 0.5;
        self.camera.set_zoom_at(center, self.viewport, zoom);
    }

    /// Snap a selection rectangle to the grid and other shapes' edges.
    #[must_use]
    pub fn snap_offset(&self, rect: Rect, threshold_world: f64, ignore: &[ShapeId]) -> Vec2 {
        if !threshold_world.is_finite() || threshold_world <= 0.0 {
            return Vec2::ZERO;
        }
        let mut best_x = threshold_world;
        let mut best_y = threshold_world;
        let mut offset = Vec2::ZERO;

        let mut consider = |targets_x: &[f64], targets_y: &[f64]| {
            for target in targets_x {
                for candidate in [rect.min.x, rect.center().x, rect.max.x] {
                    let delta = target - candidate;
                    if delta.abs() < best_x {
                        best_x = delta.abs();
                        offset.x = delta;
                    }
                }
            }
            for target in targets_y {
                for candidate in [rect.min.y, rect.center().y, rect.max.y] {
                    let delta = target - candidate;
                    if delta.abs() < best_y {
                        best_y = delta.abs();
                        offset.y = delta;
                    }
                }
            }
        };

        if self.view.grid && self.view.grid_size > 0.0 {
            let grid = self.view.grid_size;
            let gx = (rect.min.x / grid).round() * grid;
            let gy = (rect.min.y / grid).round() * grid;
            consider(&[gx], &[gy]);
        }
        for shape in self.doc.shapes() {
            if ignore.contains(&shape.id) {
                continue;
            }
            let bounds = shape.world_bounds();
            consider(
                &[bounds.min.x, bounds.center().x, bounds.max.x],
                &[bounds.min.y, bounds.center().y, bounds.max.y],
            );
        }
        offset
    }

    /// Fill the viewport with the current page, used after loading.
    pub fn frame_page(&mut self) {
        self.zoom_to_fit();
    }
}

/// Minimum drag length (world units) required to commit an Arrow/Line create.
pub const LINEAR_CREATE_MIN_LENGTH: f64 = 3.0;

/// Whether an Arrow/Line create drag is long enough to commit.
///
/// Gates on drag length rather than bounding-box width AND height so
/// axis-aligned (horizontal/vertical) drags are not dropped.
#[must_use]
pub fn linear_create_drag_commits(start: Vec2, current: Vec2) -> bool {
    start.distance(current) >= LINEAR_CREATE_MIN_LENGTH
}

/// Resolve arrow endpoints the same way bound arrows are refreshed.
///
/// When both ends have target rects, each end anchors toward the *other*
/// rect's center (center-to-center). With only one bound end, that end
/// anchors toward the free endpoint.
#[must_use]
pub fn resolve_arrow_endpoints(
    start: Vec2,
    end: Vec2,
    start_rect: Option<Rect>,
    end_rect: Option<Rect>,
) -> (Vec2, Vec2) {
    let mut start = start;
    let mut end = end;
    if let Some(rect) = start_rect {
        let toward = end_rect.map_or(end, Rect::center);
        start = rect_anchor(rect, toward);
    }
    if let Some(rect) = end_rect {
        let toward = start_rect.map_or(start, Rect::center);
        end = rect_anchor(rect, toward);
    }
    (start, end)
}

/// Theme-aware default ink for newly created shapes.
#[must_use]
pub fn default_style_for(kind: ShapeKind, dark: bool) -> Style {
    let mut style = default_style(kind);
    if !dark {
        return style;
    }
    match kind {
        ShapeKind::Sticky => {
            style.fill_color = "#4a3a14".into();
            style.stroke = "#ffdb8d".into();
        }
        ShapeKind::Highlighter => {
            style.stroke = "#ffdb8d".into();
            style.opacity = 0.55;
        }
        ShapeKind::Laser => {
            style.stroke = "#f7768e".into();
        }
        ShapeKind::Frame | ShapeKind::Image => {
            style.stroke = "#767676".into();
        }
        _ => {
            style.stroke = "#e1e1e1".into();
            if kind.is_geometric() {
                style.fill_color = "#3b2a54".into();
            }
        }
    }
    style
}

pub fn default_style(kind: ShapeKind) -> Style {
    let mut style = Style::default();
    match kind {
        ShapeKind::Sticky => {
            style.fill = Fill::Full;
            style.fill_color = "#ffdb8d".into();
            style.stroke = "#a8780a".into();
            style.dash = drawcraft_document::Dash::None;
            style.font_size = 20.0;
        }
        ShapeKind::Text => {
            style.fill = Fill::None;
            style.dash = drawcraft_document::Dash::None;
            style.font_size = 22.0;
        }
        ShapeKind::Frame => {
            style.fill = Fill::None;
            style.dash = drawcraft_document::Dash::Dashed;
            style.stroke = "#767676".into();
        }
        ShapeKind::Highlighter => {
            style.fill = Fill::None;
            style.stroke = "#ffdb8d".into();
            style.stroke_width = 14.0;
            style.opacity = 0.55;
        }
        ShapeKind::Draw => {
            style.fill = Fill::None;
            style.stroke_width = 3.0;
        }
        ShapeKind::Line | ShapeKind::Polyline | ShapeKind::Arrow => {
            style.fill = Fill::None;
        }
        ShapeKind::Laser => {
            style.fill = Fill::None;
            style.stroke = "#cd3048".into();
            style.opacity = 0.65;
        }
        ShapeKind::Image => {
            style.fill = Fill::None;
            style.stroke = "#767676".into();
        }
        _ => {}
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style_is_kind_specific() {
        assert_eq!(default_style(ShapeKind::Sticky).fill, Fill::Full);
        assert_eq!(default_style(ShapeKind::Arrow).fill, Fill::None);
        assert!(default_style(ShapeKind::Highlighter).opacity < 1.0);
    }

    #[test]
    fn linear_create_gate_accepts_axis_aligned_drags() {
        let start = Vec2::new(0.0, 50.0);
        let horizontal = Vec2::new(100.0, 50.0);
        let vertical = Vec2::new(0.0, 150.0);
        let tiny = Vec2::new(1.0, 50.0);

        assert!(linear_create_drag_commits(start, horizontal));
        assert!(linear_create_drag_commits(start, vertical));
        assert!(!linear_create_drag_commits(start, tiny));
        assert!(!linear_create_drag_commits(start, start));

        // Old width&&height gate would reject these (height or width ~0).
        let hz = Rect::new(start, horizontal);
        let vt = Rect::new(start, vertical);
        assert!(hz.height() < LINEAR_CREATE_MIN_LENGTH);
        assert!(vt.width() < LINEAR_CREATE_MIN_LENGTH);
        assert!(
            !(hz.width() >= LINEAR_CREATE_MIN_LENGTH && hz.height() >= LINEAR_CREATE_MIN_LENGTH),
            "old width&&height gate must reject the horizontal drag this length gate accepts"
        );
        assert!(
            !(vt.width() >= LINEAR_CREATE_MIN_LENGTH && vt.height() >= LINEAR_CREATE_MIN_LENGTH),
            "old width&&height gate must reject the vertical drag this length gate accepts"
        );
    }

    #[test]
    fn both_bound_arrow_endpoints_are_center_to_center() {
        let left = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let right = Rect::from_xywh(300.0, 0.0, 100.0, 100.0);
        // Raw pointer hits near opposite corners; resolver must still go center-to-center.
        let (start, end) = resolve_arrow_endpoints(
            Vec2::new(90.0, 10.0),
            Vec2::new(310.0, 90.0),
            Some(left),
            Some(right),
        );
        let expected_start = rect_anchor(left, right.center());
        let expected_end = rect_anchor(right, left.center());
        assert!((start - expected_start).length() < 1e-9);
        assert!((end - expected_end).length() < 1e-9);
    }

    #[test]
    fn horizontal_arrow_create_path_commits_and_binds() {
        let mut editor = Editor::new();
        let left = editor
            .add_shape(
                ShapeKind::Rectangle,
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                "",
            )
            .expect("left");
        let right = editor
            .add_shape(
                ShapeKind::Rectangle,
                Rect::from_xywh(300.0, 0.0, 100.0, 100.0),
                "",
            )
            .expect("right");
        let start = Vec2::new(100.0, 50.0);
        let end = Vec2::new(300.0, 50.0);
        assert!(
            linear_create_drag_commits(start, end),
            "axis-aligned arrow drag must pass the create gate"
        );
        let preview = editor.arrow_endpoints_for_drag(start, end, 12.0);
        let arrow = editor
            .add_arrow_between(preview.0, preview.1, true)
            .expect("arrow");
        let shape = editor.doc.shape(&arrow).expect("arrow shape");
        assert_eq!(shape.arrow_start.as_ref(), Some(&left));
        assert_eq!(shape.arrow_end.as_ref(), Some(&right));
        let committed = shape.endpoints();
        assert!(
            (committed.0 - preview.0).length() < 1e-6,
            "preview start must match committed start; preview={preview:?} committed={committed:?}"
        );
        assert!(
            (committed.1 - preview.1).length() < 1e-6,
            "preview end must match committed end; preview={preview:?} committed={committed:?}"
        );
    }

    #[test]
    fn arrow_snap_skips_linear_shapes() {
        let mut editor = Editor::new();
        let rect = editor
            .add_shape(
                ShapeKind::Rectangle,
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                "",
            )
            .expect("rect");
        let _line = editor
            .add_shape(
                ShapeKind::Line,
                Rect::from_xywh(200.0, 40.0, 100.0, 20.0),
                "",
            )
            .expect("line");
        // Point on the line's bbox; bindable lookup must ignore the line.
        let on_line = Vec2::new(250.0, 50.0);
        assert!(editor.topmost_at(on_line, 12.0).is_some());
        assert!(editor.topmost_bindable_at(on_line, 12.0).is_none());
        let on_rect = Vec2::new(50.0, 50.0);
        assert_eq!(
            editor.topmost_bindable_at(on_rect, 12.0).as_ref(),
            Some(&rect)
        );
    }
}
