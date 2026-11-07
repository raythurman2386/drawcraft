//! Synthetic egui event integration tests, not physical touch/palm hardware proof.
#![allow(clippy::unwrap_used, clippy::print_stdout)]
use crate::app::DrawCraftApp;
use drawcraft_document::{Document, Rect as DocRect, ShapeKind, Vec2};
use drawcraft_editor::{History, Tool};
use egui::{
    Event, Modifiers, PointerButton, Pos2, RawInput, Rect, TouchDeviceId, TouchId, TouchPhase,
    pos2, vec2,
};

struct CanvasInput {
    app: DrawCraftApp,
    ctx: egui::Context,
    time: f64,
}

impl CanvasInput {
    fn new(tool: Tool) -> Self {
        let mut app = DrawCraftApp::new(None);
        app.session.editor.tool = tool;
        app.session.editor.view.touch_draw = true;
        let mut input = Self {
            app,
            ctx: egui::Context::default(),
            time: 0.0,
        };
        input.frame(vec![]);
        input
    }
    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(raw, |ui| self.app.canvas(ui));
        output.textures_delta.clear();
    }
    fn shape(&mut self, x: f64) {
        let editor = &mut self.app.session.editor;
        let world = editor
            .camera
            .screen_to_world(Vec2::new(x, 300.0), Vec2::new(800.0, 600.0));
        editor
            .add_shape(
                ShapeKind::Rectangle,
                DocRect::from_xywh(world.x - 40.0, world.y - 40.0, 80.0, 80.0),
                "",
            )
            .unwrap();
        editor.selection.clear();
    }
    fn start(&mut self, pos: Pos2) {
        self.frame(vec![
            touch(1, TouchPhase::Start, pos),
            Event::PointerMoved(pos),
            button(pos, true),
        ]);
    }
    fn move_to(&mut self, pos: Pos2) {
        self.frame(vec![
            touch(1, TouchPhase::Move, pos),
            Event::PointerMoved(pos),
        ]);
    }
    fn release(&mut self, pos: Pos2) {
        self.frame(vec![
            touch(1, TouchPhase::End, pos),
            button(pos, false),
            Event::PointerGone,
        ]);
        self.frame(vec![]);
    }
    fn cancel_with_second_finger(&mut self, pos: Pos2) {
        let second = pos2(400.0, 500.0);
        self.frame(vec![
            touch(2, TouchPhase::Start, second),
            touch(1, TouchPhase::Move, pos),
            Event::PointerMoved(pos),
        ]);
        assert!(
            self.app.state.drag.is_none(),
            "real canvas touch navigation must cancel the drag"
        );
        self.frame(vec![
            touch(1, TouchPhase::End, pos),
            touch(2, TouchPhase::End, second),
            button(pos, false),
            Event::PointerGone,
        ]);
        self.frame(vec![]);
    }
    fn erase_three(&mut self) -> Pos2 {
        self.start(pos2(100.0, 300.0));
        for x in (106..=700).step_by(6) {
            self.move_to(pos2(x as f32, 300.0));
        }
        assert!(self.app.state.drag.is_some());
        assert_eq!(
            self.app.session.editor.doc.shape_count(),
            0,
            "precondition: all three shapes erased mid-gesture"
        );
        pos2(700.0, 300.0)
    }
}
fn touch(id: u64, phase: TouchPhase, pos: Pos2) -> Event {
    Event::Touch {
        device_id: TouchDeviceId(0),
        id: TouchId(id),
        phase,
        pos,
        force: None,
    }
}
fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    }
}
fn history_states(history: &History, doc: &Document) -> (Vec<Document>, Vec<Document>) {
    let (mut undo, mut redo) = (Vec::new(), Vec::new());
    let (mut h, mut d) = (history.clone(), doc.clone());
    while h.undo(&mut d) {
        undo.push(d.clone());
    }
    let (mut h, mut d) = (history.clone(), doc.clone());
    while h.redo(&mut d) {
        redo.push(d.clone());
    }
    (undo, redo)
}
fn assert_restored(input: &CanvasInput, before: &Document, history: &History) {
    let editor = &input.app.session.editor;
    println!(
        "cancel before shapes={} undo={} redo={}; after shapes={} undo={} redo={}",
        before.shape_count(),
        history.undo_len(),
        history.redo_len(),
        editor.doc.shape_count(),
        editor.history.undo_len(),
        editor.history.redo_len()
    );
    assert_eq!(
        &editor.doc, before,
        "cancel must restore the entire pre-gesture document"
    );
    assert_eq!(
        (editor.history.undo_len(), editor.history.redo_len()),
        (history.undo_len(), history.redo_len()),
        "cancel must restore exact history depths"
    );
    assert_eq!(
        history_states(&editor.history, &editor.doc),
        history_states(history, before),
        "every preexisting undo/redo state must survive, not just lengths"
    );
}

#[test]
fn multi_shape_eraser_cancel_restores_document_and_existing_undo_redo() {
    let mut input = CanvasInput::new(Tool::Eraser);
    for x in [200.0, 400.0, 600.0, 750.0] {
        input.shape(x);
    }
    input.app.session.editor.undo(); // leave a real redo branch before the gesture
    let before = input.app.session.editor.doc.clone();
    let history = input.app.session.editor.history.clone();
    assert_eq!((history.undo_len(), history.redo_len()), (3, 1));
    let end = input.erase_three();
    input.cancel_with_second_finger(end);
    assert_restored(&input, &before, &history);
}

#[test]
fn duplicate_snapshot_move_cancel_restores_document_and_history() {
    let mut input = CanvasInput::new(Tool::Select);
    input.shape(400.0);
    assert!(input.app.session.editor.snapshot());
    assert!(
        !input.app.session.editor.snapshot(),
        "force deduplicated gesture snapshot case"
    );
    let before = input.app.session.editor.doc.clone();
    let history = input.app.session.editor.history.clone();
    input.start(pos2(400.0, 300.0));
    for x in (410..=460).step_by(10) {
        input.move_to(pos2(x as f32, 300.0));
    }
    assert_ne!(
        input.app.session.editor.doc, before,
        "precondition: move really mutated the document"
    );
    input.cancel_with_second_finger(pos2(460.0, 300.0));
    assert_restored(&input, &before, &history);
}

#[test]
fn move_cancel_preserves_existing_redo() {
    let mut input = CanvasInput::new(Tool::Select);
    input.shape(400.0);
    input.shape(600.0);
    input.app.session.editor.undo();
    let before = input.app.session.editor.doc.clone();
    let history = input.app.session.editor.history.clone();
    assert_eq!(history.redo_len(), 1);
    input.start(pos2(400.0, 300.0));
    for x in (410..=460).step_by(10) {
        input.move_to(pos2(x as f32, 300.0));
    }
    assert_ne!(input.app.session.editor.doc, before);
    input.cancel_with_second_finger(pos2(460.0, 300.0));
    assert_restored(&input, &before, &history);
}

#[test]
fn committed_multi_erase_keeps_existing_per_delete_undo_behavior() {
    let mut input = CanvasInput::new(Tool::Eraser);
    for x in [200.0, 400.0, 600.0] {
        input.shape(x);
    }
    let before = input.app.session.editor.doc.clone();
    let depth = input.app.session.editor.history.undo_len();
    let end = input.erase_three();
    input.release(end);
    assert_eq!(input.app.session.editor.doc.shape_count(), 0);
    assert_eq!(input.app.session.editor.history.undo_len(), depth + 3);
    for count in 1..=3 {
        input.app.session.editor.undo();
        assert_eq!(input.app.session.editor.doc.shape_count(), count);
    }
    assert_eq!(input.app.session.editor.doc, before);
    for count in (0..3).rev() {
        input.app.session.editor.redo();
        assert_eq!(input.app.session.editor.doc.shape_count(), count);
    }
}

#[test]
fn committed_move_keeps_single_undo_and_clears_redo() {
    let mut input = CanvasInput::new(Tool::Select);
    input.shape(400.0);
    input.shape(600.0);
    input.app.session.editor.undo();
    let before = input.app.session.editor.doc.clone();
    let depth = input.app.session.editor.history.undo_len();
    input.start(pos2(400.0, 300.0));
    for x in (410..=460).step_by(10) {
        input.move_to(pos2(x as f32, 300.0));
    }
    input.release(pos2(460.0, 300.0));
    let moved = input.app.session.editor.doc.clone();
    assert_ne!(moved, before);
    assert_eq!(
        (
            input.app.session.editor.history.undo_len(),
            input.app.session.editor.history.redo_len()
        ),
        (depth + 1, 0)
    );
    input.app.session.editor.undo();
    assert_eq!(input.app.session.editor.doc, before);
    input.app.session.editor.redo();
    assert_eq!(input.app.session.editor.doc, moved);
}

#[test]
fn multi_shape_eraser_cancel_restores_history_entries_evicted_during_gesture() {
    let mut input = CanvasInput::new(Tool::Eraser);
    input.app.session.editor.history = History::new(2);
    for x in [200.0, 400.0, 600.0] {
        input.shape(x);
    }
    let before = input.app.session.editor.doc.clone();
    let history = input.app.session.editor.history.clone();
    assert_eq!(history.undo_len(), 2);
    let end = input.erase_three();
    input.cancel_with_second_finger(end);
    assert_restored(&input, &before, &history);
}
