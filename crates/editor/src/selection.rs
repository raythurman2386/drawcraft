use drawcraft_document::ShapeId;

/// Ordered selection of shape IDs.
///
/// Order is insertion order, which makes command output deterministic and
/// keeps group operations stable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    ids: Vec<ShapeId>,
}

impl Selection {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    #[must_use]
    pub fn contains(&self, id: &ShapeId) -> bool {
        self.ids.iter().any(|candidate| candidate == id)
    }

    #[must_use]
    pub fn first(&self) -> Option<&ShapeId> {
        self.ids.first()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ShapeId> {
        self.ids.iter()
    }

    #[must_use]
    pub fn ids(&self) -> &[ShapeId] {
        &self.ids
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn select(&mut self, id: ShapeId) {
        self.ids.clear();
        self.ids.push(id);
    }

    /// Add `id` if missing, otherwise remove it (shift-click behaviour).
    pub fn toggle(&mut self, id: ShapeId) {
        if let Some(index) = self.ids.iter().position(|candidate| candidate == &id) {
            self.ids.remove(index);
        } else {
            self.ids.push(id);
        }
    }

    pub fn extend(&mut self, ids: impl IntoIterator<Item = ShapeId>) {
        for id in ids {
            if !self.contains(&id) {
                self.ids.push(id);
            }
        }
    }

    pub fn remove(&mut self, id: &ShapeId) {
        self.ids.retain(|candidate| candidate != id);
    }

    /// Drop IDs that no longer exist in the current page.
    pub fn retain_existing(&mut self, existing: &[ShapeId]) {
        self.ids
            .retain(|id| existing.iter().any(|candidate| candidate == id));
    }

    /// Replace the selection with IDs that exist in `existing`.
    pub fn set_existing(&mut self, ids: impl IntoIterator<Item = ShapeId>, existing: &[ShapeId]) {
        self.ids = ids
            .into_iter()
            .filter(|id| existing.iter().any(|candidate| candidate == id))
            .collect();
    }
}
