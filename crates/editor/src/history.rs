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
    /// consume undo steps. Returns `true` when a snapshot was actually stored,
    /// so callers can tell whether [`Self::rollback`] will undo their change.
    #[must_use]
    pub fn record(&mut self, document: &Document) -> bool {
        if self.undo.last() == Some(document) {
            return false;
        }
        self.undo.push(document.clone());
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    /// Discard the most recent snapshot and restore it.
    ///
    /// This is for cancelling an in-progress gesture: the document goes back to
    /// its pre-gesture state and no undo/redo step is left behind.
    pub fn rollback(&mut self, current: &mut Document) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        *current = previous;
        true
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

#[cfg(test)]
mod tests {
    use super::*;

    fn document_with_marker(marker: &str) -> Document {
        let mut document = Document::new();
        document
            .meta
            .insert("marker".to_owned(), serde_json::json!(marker));
        document
    }

    #[test]
    fn record_reports_whether_a_snapshot_was_stored() {
        let mut history = History::new(10);
        let mut document = document_with_marker("one");
        assert!(history.record(&document));
        assert!(!history.record(&document), "duplicate states are ignored");
        document
            .meta
            .insert("marker".to_owned(), serde_json::json!("two"));
        assert!(history.record(&document));
    }

    #[test]
    fn rollback_restores_the_snapshot_without_adding_redo() {
        let mut history = History::new(10);
        let mut document = document_with_marker("before");
        assert!(history.record(&document));
        document
            .meta
            .insert("marker".to_owned(), serde_json::json!("after"));

        assert!(history.rollback(&mut document));
        assert_eq!(
            document.meta.get("marker"),
            Some(&serde_json::json!("before"))
        );
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn rollback_without_snapshot_is_a_no_op() {
        let mut history = History::new(10);
        let mut document = document_with_marker("unchanged");
        assert!(!history.rollback(&mut document));
        assert_eq!(
            document.meta.get("marker"),
            Some(&serde_json::json!("unchanged"))
        );
    }
}
