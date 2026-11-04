use std::collections::{BTreeMap, HashSet};

use drawcraft_geometry::Rect;
use serde::{Deserialize, Serialize};

use crate::{DocumentError, FORMAT_VERSION, GroupId, PageId, Result, ShapeId, ShapeRecord};

/// Hard cap on pages in one document.
pub const MAX_PAGES: usize = 256;
/// Hard cap on shapes in one page.
pub const MAX_SHAPES_PER_PAGE: usize = 100_000;
/// Hard cap on points in one freehand/line record.
pub const MAX_POINTS_PER_SHAPE: usize = 100_000;

/// A named page containing an ordered list of shapes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub shapes: Vec<ShapeRecord>,
    /// Unknown page keys are preserved round-trip.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Page {
    #[must_use]
    pub fn new(id: PageId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            shapes: Vec::new(),
            extra: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn shapes_sorted_by_z(&self) -> Vec<&ShapeRecord> {
        let mut shapes: Vec<&ShapeRecord> = self.shapes.iter().collect();
        shapes.sort_by_key(|shape| (shape.z, &shape.id));
        shapes
    }

    /// Bounds of all shapes on the page, if any.
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        self.shapes
            .iter()
            .map(ShapeRecord::world_bounds)
            .reduce(Rect::union)
    }

    #[must_use]
    pub fn shape(&self, id: &ShapeId) -> Option<&ShapeRecord> {
        self.shapes.iter().find(|shape| &shape.id == id)
    }

    pub fn shape_mut(&mut self, id: &ShapeId) -> Option<&mut ShapeRecord> {
        self.shapes.iter_mut().find(|shape| &shape.id == id)
    }

    /// Insert or replace a shape by id.
    pub fn upsert_shape(&mut self, shape: ShapeRecord) {
        if let Some(existing) = self.shape_mut(&shape.id) {
            *existing = shape;
        } else {
            self.shapes.push(shape);
        }
    }

    pub fn remove_shape(&mut self, id: &ShapeId) -> Option<ShapeRecord> {
        let index = self.shapes.iter().position(|shape| &shape.id == id)?;
        Some(self.shapes.remove(index))
    }

    /// Next free shape id. Scans once, so it is safe to call in loops.
    #[must_use]
    pub fn next_shape_id(&self) -> ShapeId {
        let existing: HashSet<&str> = self.shapes.iter().map(|shape| shape.id.as_str()).collect();
        let mut index = self.shapes.len() as u64 + 1;
        loop {
            let candidate = ShapeId::generated(index);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            index = index.saturating_add(1);
        }
    }
}

/// The root document: a list of pages plus document-level metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub active_page: PageId,
    #[serde(default)]
    pub meta: BTreeMap<String, serde_json::Value>,
    /// Unknown document-level keys are preserved round-trip.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

fn default_version() -> u32 {
    FORMAT_VERSION
}

impl Document {
    #[must_use]
    pub fn new() -> Self {
        let page = Page::new(PageId::from("page-1"), "Page 1");
        Self {
            version: FORMAT_VERSION,
            active_page: page.id.clone(),
            pages: vec![page],
            meta: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    #[must_use]
    pub fn shape_count(&self) -> usize {
        self.pages.iter().map(|page| page.shapes.len()).sum()
    }

    pub fn active(&self) -> Option<&Page> {
        self.page(&self.active_page)
    }

    pub fn active_mut(&mut self) -> Option<&mut Page> {
        let active = self.active_page.clone();
        self.page_mut(&active)
    }

    pub fn page(&self, id: &PageId) -> Option<&Page> {
        self.pages.iter().find(|page| &page.id == id)
    }

    pub fn page_mut(&mut self, id: &PageId) -> Option<&mut Page> {
        self.pages.iter_mut().find(|page| &page.id == id)
    }

    pub fn set_active_page(&mut self, id: &PageId) -> Result<()> {
        if self.page(id).is_none() {
            return Err(DocumentError::MissingActivePage(id.clone()));
        }
        self.active_page = id.clone();
        Ok(())
    }

    pub fn add_page(&mut self, name: impl Into<String>) -> Result<PageId> {
        if self.pages.len() >= MAX_PAGES {
            return Err(DocumentError::TooManyPages {
                actual: self.pages.len() + 1,
                limit: MAX_PAGES,
            });
        }
        let id = self.next_page_id();
        let fallback = format!("Page {}", self.pages.len() + 1);
        let name = name.into();
        let name = if name.trim().is_empty() {
            fallback
        } else {
            name
        };
        self.pages.push(Page::new(id.clone(), name));
        Ok(id)
    }

    /// Insert a page, assigning a fresh id if the incoming id already exists.
    pub fn insert_page(&mut self, mut page: Page) -> Result<PageId> {
        if self.pages.len() >= MAX_PAGES {
            return Err(DocumentError::TooManyPages {
                actual: self.pages.len() + 1,
                limit: MAX_PAGES,
            });
        }
        if self.page(&page.id).is_some() {
            page.id = self.next_page_id();
        }
        let id = page.id.clone();
        self.pages.push(page);
        Ok(id)
    }

    pub fn remove_page(&mut self, id: &PageId) -> Result<()> {
        if self.pages.len() <= 1 {
            return Err(DocumentError::Invalid(
                "a document must keep at least one page".into(),
            ));
        }
        let index = self
            .pages
            .iter()
            .position(|page| &page.id == id)
            .ok_or_else(|| DocumentError::MissingActivePage(id.clone()))?;
        self.pages.remove(index);
        if &self.active_page == id {
            let next = index.min(self.pages.len() - 1);
            self.active_page = self.pages[next].id.clone();
        }
        Ok(())
    }

    pub fn duplicate_page(&mut self, id: &PageId, name: impl Into<String>) -> Result<PageId> {
        let source = self
            .page(id)
            .cloned()
            .ok_or_else(|| DocumentError::MissingActivePage(id.clone()))?;
        let new_id = self.next_page_id();
        let mut page = source;
        page.id = new_id.clone();
        let name = name.into();
        page.name = if name.trim().is_empty() {
            format!("{} copy", page.name)
        } else {
            name
        };
        // Reassign shape ids so both pages stay independently addressable, then
        // remap arrow bindings and frame parents to the copied ids.
        let mut used: HashSet<String> = page
            .shapes
            .iter()
            .map(|shape| shape.id.as_str().to_owned())
            .collect();
        let mut id_map: BTreeMap<ShapeId, ShapeId> = BTreeMap::new();
        let mut group_map: BTreeMap<GroupId, GroupId> = BTreeMap::new();
        let mut index = page.shapes.len() as u64;
        for shape in &mut page.shapes {
            index = index.saturating_add(1);
            let mut candidate = ShapeId::generated(index);
            while used.contains(candidate.as_str()) {
                index = index.saturating_add(1);
                candidate = ShapeId::generated(index);
            }
            used.insert(candidate.as_str().to_owned());
            id_map.insert(shape.id.clone(), candidate.clone());
            shape.id = candidate;
            if let Some(group) = &shape.group_id {
                let count = group_map.len() as u64;
                let mapped = group_map
                    .entry(group.clone())
                    .or_insert_with(|| GroupId::from(format!("{}-copy-{count}", new_id.as_str())));
                shape.group_id = Some(mapped.clone());
            }
        }
        for shape in &mut page.shapes {
            if let Some(start) = &shape.arrow_start {
                shape.arrow_start = id_map.get(start).cloned();
            }
            if let Some(end) = &shape.arrow_end {
                shape.arrow_end = id_map.get(end).cloned();
            }
            if let Some(parent) = &shape.parent_id {
                shape.parent_id = id_map.get(parent).cloned();
            }
        }
        self.pages.push(page);
        Ok(new_id)
    }

    pub fn rename_page(&mut self, id: &PageId, name: impl Into<String>) -> Result<()> {
        let page = self
            .page_mut(id)
            .ok_or_else(|| DocumentError::MissingActivePage(id.clone()))?;
        let name = name.into();
        page.name = if name.trim().is_empty() {
            "Untitled page".into()
        } else {
            name
        };
        Ok(())
    }

    pub fn move_page(&mut self, id: &PageId, offset: isize) -> Result<()> {
        let index = self
            .pages
            .iter()
            .position(|page| &page.id == id)
            .ok_or_else(|| DocumentError::MissingActivePage(id.clone()))?;
        let target = (index as isize + offset).clamp(0, self.pages.len() as isize - 1) as usize;
        if target != index {
            let page = self.pages.remove(index);
            self.pages.insert(target, page);
        }
        Ok(())
    }

    pub fn shape(&self, id: &ShapeId) -> Option<&ShapeRecord> {
        self.active().and_then(|page| page.shape(id))
    }

    pub fn shape_mut(&mut self, id: &ShapeId) -> Option<&mut ShapeRecord> {
        self.active_mut().and_then(|page| page.shape_mut(id))
    }

    pub fn shapes(&self) -> &[ShapeRecord] {
        self.active().map_or(&[], |page| page.shapes.as_slice())
    }

    pub fn shapes_mut(&mut self) -> &mut [ShapeRecord] {
        self.active_mut()
            .map_or(&mut [], |page| page.shapes.as_mut_slice())
    }

    pub fn add_shape(&mut self, shape: ShapeRecord) -> Result<ShapeId> {
        let page_id = self.active_page.clone();
        let page = self
            .page_mut(&page_id)
            .ok_or_else(|| DocumentError::MissingActivePage(page_id.clone()))?;
        if page.shapes.len() >= MAX_SHAPES_PER_PAGE {
            return Err(DocumentError::TooManyShapes {
                page: page_id,
                actual: page.shapes.len() + 1,
                limit: MAX_SHAPES_PER_PAGE,
            });
        }
        let id = shape.id.clone();
        if page.shape(&id).is_some() {
            return Err(DocumentError::DuplicateShape(id));
        }
        page.upsert_shape(shape);
        Ok(id)
    }

    pub fn remove_shape(&mut self, id: &ShapeId) -> Option<ShapeRecord> {
        self.active_mut().and_then(|page| page.remove_shape(id))
    }

    #[must_use]
    pub fn next_shape_id(&self) -> ShapeId {
        self.active()
            .map_or_else(|| ShapeId::from("shape-1"), Page::next_shape_id)
    }

    #[must_use]
    pub fn selection_bounds(&self, ids: &[ShapeId]) -> Option<Rect> {
        let page = self.active()?;
        ids.iter()
            .filter_map(|id| page.shape(id))
            .map(ShapeRecord::world_bounds)
            .reduce(Rect::union)
    }

    /// Shapes belonging to a group.
    #[must_use]
    pub fn group_members(&self, group: &GroupId) -> Vec<&ShapeRecord> {
        self.shapes()
            .iter()
            .filter(|shape| shape.group_id.as_ref() == Some(group))
            .collect()
    }

    fn next_page_id(&self) -> PageId {
        let existing: HashSet<&str> = self.pages.iter().map(|page| page.id.as_str()).collect();
        let mut index = self.pages.len() as u64 + 1;
        loop {
            let candidate = PageId::generated(index);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            index = index.saturating_add(1);
        }
    }

    /// Structural and numeric validation for untrusted documents.
    pub fn validate(&self) -> Result<()> {
        if self.version != FORMAT_VERSION {
            return Err(DocumentError::UnsupportedVersion(self.version));
        }
        if self.pages.is_empty() {
            return Err(DocumentError::NoPages);
        }
        if self.pages.len() > MAX_PAGES {
            return Err(DocumentError::TooManyPages {
                actual: self.pages.len(),
                limit: MAX_PAGES,
            });
        }

        let mut page_ids: HashSet<&str> = HashSet::with_capacity(self.pages.len());
        for page in &self.pages {
            if page.id.is_empty() {
                return Err(DocumentError::InvalidPage {
                    id: page.id.clone(),
                    reason: "page id must not be empty".into(),
                });
            }
            if !page_ids.insert(page.id.as_str()) {
                return Err(DocumentError::DuplicatePage(page.id.clone()));
            }
            if page.name.len() > 512 {
                return Err(DocumentError::InvalidPage {
                    id: page.id.clone(),
                    reason: "page name is too long".into(),
                });
            }
            if page.shapes.len() > MAX_SHAPES_PER_PAGE {
                return Err(DocumentError::TooManyShapes {
                    page: page.id.clone(),
                    actual: page.shapes.len(),
                    limit: MAX_SHAPES_PER_PAGE,
                });
            }

            let mut shape_ids: HashSet<&str> = HashSet::with_capacity(page.shapes.len());
            for shape in &page.shapes {
                if !shape_ids.insert(shape.id.as_str()) {
                    return Err(DocumentError::DuplicateShape(shape.id.clone()));
                }
                shape
                    .validate()
                    .map_err(|reason| DocumentError::InvalidShape {
                        id: shape.id.clone(),
                        reason,
                    })?;
                if shape.points.len() > MAX_POINTS_PER_SHAPE {
                    return Err(DocumentError::TooManyPoints {
                        shape: shape.id.clone(),
                        actual: shape.points.len(),
                        limit: MAX_POINTS_PER_SHAPE,
                    });
                }
            }

            // References must resolve inside the same page.
            for shape in &page.shapes {
                for binding in [shape.arrow_start.as_ref(), shape.arrow_end.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    if !shape_ids.contains(binding.as_str()) {
                        return Err(DocumentError::InvalidShape {
                            id: shape.id.clone(),
                            reason: format!("bound shape {binding} does not exist on this page"),
                        });
                    }
                }
                if let Some(parent) = &shape.parent_id
                    && !shape_ids.contains(parent.as_str())
                {
                    return Err(DocumentError::InvalidShape {
                        id: shape.id.clone(),
                        reason: format!("parent shape {parent} does not exist on this page"),
                    });
                }
            }
        }

        if self.page(&self.active_page).is_none() {
            return Err(DocumentError::MissingActivePage(self.active_page.clone()));
        }
        Ok(())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ShapeKind, Vec2};

    fn shape(id: &str) -> ShapeRecord {
        ShapeRecord::new(
            ShapeId::from(id),
            ShapeKind::Rectangle,
            Vec2::ZERO,
            Vec2::new(10.0, 10.0),
        )
    }

    #[test]
    fn new_document_is_valid() {
        let document = Document::new();
        assert!(document.validate().is_ok());
        assert_eq!(document.page_count(), 1);
    }

    #[test]
    fn pages_can_be_added_and_removed() {
        let mut document = Document::new();
        let id = document
            .add_page("Second")
            .unwrap_or_else(|_| PageId::from("x"));
        assert_eq!(document.page_count(), 2);
        assert!(document.remove_page(&id).is_ok());
        assert_eq!(document.page_count(), 1);
        assert!(document.remove_page(&document.active_page.clone()).is_err());
    }

    #[test]
    fn duplicate_page_reassigns_shape_ids() {
        let mut document = Document::new();
        document.add_shape(shape("shape-1")).unwrap_or_default();
        let copy = document
            .duplicate_page(&document.active_page.clone(), "Copy")
            .unwrap_or_else(|_| PageId::from("copy"));
        let duplicated = document.page(&copy).map(|page| &page.shapes[0]);
        assert_eq!(
            duplicated.map(|shape| shape.id.clone()),
            Some(ShapeId::from("shape-2"))
        );
    }

    #[test]
    fn duplicate_page_remaps_arrow_bindings() {
        let mut document = Document::new();
        let left = shape("shape-1");
        let mut right = shape("shape-2");
        right.pos = Vec2::new(100.0, 0.0);
        let mut arrow = shape("shape-3");
        arrow.kind = ShapeKind::Arrow;
        arrow.arrow_start = Some(left.id.clone());
        arrow.arrow_end = Some(right.id.clone());
        let _ = document.add_shape(left);
        let _ = document.add_shape(right);
        let _ = document.add_shape(arrow);
        let page_id = document.active_page.clone();
        let copy = document
            .duplicate_page(&page_id, "Copy")
            .unwrap_or_else(|_| PageId::from("copy"));
        let copy = document
            .page(&copy)
            .unwrap_or_else(|| panic!("copy missing"));
        let arrow = copy
            .shapes
            .iter()
            .find(|shape| shape.kind == ShapeKind::Arrow)
            .unwrap_or_else(|| panic!("arrow missing"));
        assert!(
            arrow
                .arrow_start
                .as_ref()
                .is_some_and(|id| copy.shape(id).is_some())
        );
        assert!(
            arrow
                .arrow_end
                .as_ref()
                .is_some_and(|id| copy.shape(id).is_some())
        );
        assert!(document.validate().is_ok());
    }

    #[test]
    fn validation_rejects_dangling_binding() {
        let mut document = Document::new();
        let mut arrow = shape("shape-1");
        arrow.kind = ShapeKind::Arrow;
        arrow.arrow_end = Some(ShapeId::from("missing"));
        let _ = document.add_shape(arrow);
        assert!(document.validate().is_err());
    }

    #[test]
    fn validation_rejects_duplicate_ids() {
        let mut document = Document::new();
        if let Some(page) = document.active_mut() {
            page.shapes.push(shape("shape-1"));
            page.shapes.push(shape("shape-1"));
        }
        assert!(matches!(
            document.validate(),
            Err(DocumentError::DuplicateShape(_))
        ));
    }

    #[test]
    fn next_shape_id_skips_existing() {
        let mut document = Document::new();
        let _ = document.add_shape(shape("shape-2"));
        assert_eq!(document.next_shape_id(), ShapeId::from("shape-3"));
    }
}
