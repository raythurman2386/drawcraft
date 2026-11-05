//! Pure decision logic for the inline canvas text editor.
//!
//! Kept free of egui so the commit/cancel rules can be unit tested; the canvas
//! code only translates egui input into these types and acts on the result.

/// A key press the inline editor cares about.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InlineKey {
    /// Enter, with or without Shift held.
    Enter { shift: bool },
    /// Escape.
    Escape,
}

/// What the inline editor should do this frame.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InlineEditAction {
    /// Keep editing.
    None,
    /// Let the multiline widget insert a newline (Shift+Enter in Sticky/Note).
    Newline,
    /// Close the editor and apply the text (if it changed).
    Commit,
    /// Close the editor and discard the edit.
    Cancel,
}

impl InlineEditAction {
    /// True when the editor should close this frame.
    pub fn closes(self) -> bool {
        matches!(self, Self::Commit | Self::Cancel)
    }
}

/// Decide what the inline editor does.
///
/// * `multiline` – Sticky/Note editors (Shift+Enter inserts a newline).
/// * `owns_keyboard` – the editor holds keyboard focus (or no other widget
///   does); Enter/Escape are ignored otherwise so they never hijack another
///   focused field such as an inspector input.
/// * `key` – the first Enter/Escape press this frame, if any.
/// * `lost_focus` – the text field lost focus this frame (e.g. click-away).
///
/// Escape wins over a simultaneous focus loss (egui drops focus on Escape
/// before widgets run), so cancelling never commits.
pub fn decide(
    multiline: bool,
    owns_keyboard: bool,
    key: Option<InlineKey>,
    lost_focus: bool,
) -> InlineEditAction {
    if owns_keyboard {
        match key {
            Some(InlineKey::Escape) => return InlineEditAction::Cancel,
            Some(InlineKey::Enter { shift: true }) if multiline => {
                return InlineEditAction::Newline;
            }
            Some(InlineKey::Enter { .. }) => return InlineEditAction::Commit,
            None => {}
        }
    }
    if lost_focus {
        InlineEditAction::Commit
    } else {
        InlineEditAction::None
    }
}

/// The text to write back on commit, or `None` when nothing changed (so an
/// unchanged commit creates no undo entry and does not dirty the document).
pub fn committed_text(current: &str, edited: &str) -> Option<String> {
    (current != edited).then(|| edited.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use InlineEditAction as A;

    const ENTER: Option<InlineKey> = Some(InlineKey::Enter { shift: false });
    const SHIFT_ENTER: Option<InlineKey> = Some(InlineKey::Enter { shift: true });
    const ESC: Option<InlineKey> = Some(InlineKey::Escape);

    #[test]
    fn enter_commits_single_and_multiline() {
        assert_eq!(decide(false, true, ENTER, false), A::Commit);
        assert_eq!(decide(true, true, ENTER, false), A::Commit);
    }

    #[test]
    fn shift_enter_is_newline_only_for_multiline() {
        assert_eq!(decide(true, true, SHIFT_ENTER, false), A::Newline);
        assert_eq!(decide(false, true, SHIFT_ENTER, false), A::Commit);
    }

    #[test]
    fn escape_cancels_even_when_focus_was_dropped() {
        assert_eq!(decide(false, true, ESC, false), A::Cancel);
        assert_eq!(decide(true, true, ESC, true), A::Cancel);
    }

    #[test]
    fn lost_focus_commits() {
        assert_eq!(decide(false, true, None, true), A::Commit);
        assert_eq!(decide(true, false, None, true), A::Commit);
    }

    #[test]
    fn keys_ignored_without_keyboard_ownership() {
        assert_eq!(decide(false, false, ENTER, false), A::None);
        assert_eq!(decide(true, false, ESC, false), A::None);
        assert_eq!(decide(true, false, SHIFT_ENTER, false), A::None);
    }

    #[test]
    fn idle_frame_keeps_editing() {
        assert_eq!(decide(true, true, None, false), A::None);
        assert!(!A::None.closes() && !A::Newline.closes());
        assert!(A::Commit.closes() && A::Cancel.closes());
    }

    #[test]
    fn unchanged_text_is_skipped() {
        assert_eq!(committed_text("Hello", "Hello"), Option::None);
        assert_eq!(
            committed_text("Hello", "Hello world"),
            Some("Hello world".into())
        );
        assert_eq!(committed_text("a", "a\nb"), Some("a\nb".into()));
    }
}
