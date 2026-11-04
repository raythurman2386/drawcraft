use drawcraft_document::Document;

/// Bounded undo/redo stack of document snapshots.
#[derive(Clone, Debug)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
    limit: usize,
}

impl History {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Record the current document before a mutation.
    ///
    /// Duplicate consecutive states are ignored so that no-op commands do not
    /// consume undo steps.
    pub fn record(&mut self, document: &Document) {
        if self.undo.last() == Some(document) {
            return;
        }
        self.undo.push(document.clone());
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: &mut Document) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(current, previous));
        true
    }

    pub fn redo(&mut self, current: &mut Document) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(current, next));
        true
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(200)
    }
}
