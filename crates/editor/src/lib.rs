//! Headless DrawCraft editor.
//!
//! The editor owns the document, camera, selection, tool, and history. It has
//! no windowing or rendering dependency, so every interaction can be tested and
//! driven by an agent through [`Editor::dispatch`].

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

mod commands;
mod editor;
mod error;
mod history;
mod params;
mod selection;
mod tool;

pub use drawcraft_document::{Document, ShapeId, ShapeKind, ShapeRecord, Style};
pub use drawcraft_geometry::{Camera, Rect, Vec2};
pub use editor::{Editor, StyleUpdate, ViewState, default_style, default_style_for};
pub use error::EditError;
pub use history::History;
pub use params::Params;
pub use selection::Selection;
pub use tool::Tool;

pub type Result<T> = std::result::Result<T, EditError>;

#[cfg(test)]
mod tests {
    use super::*;
    use drawcraft_command::{Category, Registry, Scope, ids};
    use serde_json::{Value, json};

    fn editor() -> Editor {
        Editor::new()
    }

    fn add_rect(editor: &mut Editor, x: f64, y: f64, width: f64, height: f64) -> ShapeId {
        let value = editor
            .dispatch(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":x,"y":y,"width":width,"height":height}),
            )
            .unwrap_or_default();
        value
            .get("id")
            .and_then(|id| id.as_str())
            .map(ShapeId::from)
            .unwrap_or_else(|| ShapeId::from("missing"))
    }

    #[test]
    fn add_move_and_undo() {
        let mut editor = editor();
        let id = add_rect(&mut editor, 10.0, 20.0, 100.0, 50.0);
        assert!(editor.doc.shape(&id).is_some());

        editor
            .dispatch(ids::EDIT_MOVE, &json!({"dx":5.0,"dy":7.0}))
            .unwrap_or_default();
        let position = editor.doc.shape(&id).map(|shape| shape.pos);
        assert_eq!(position, Some(Vec2::new(15.0, 27.0)));

        assert!(editor.undo());
        let position = editor.doc.shape(&id).map(|shape| shape.pos);
        assert_eq!(position, Some(Vec2::new(10.0, 20.0)));
        assert!(editor.redo());
    }

    #[test]
    fn duplicate_and_paste_create_new_ids() {
        let mut editor = editor();
        let id = add_rect(&mut editor, 0.0, 0.0, 10.0, 10.0);
        editor
            .dispatch(ids::EDIT_SELECT_ALL, &json!({}))
            .unwrap_or_default();
        let duplicate = editor
            .dispatch(ids::EDIT_DUPLICATE, &json!({}))
            .unwrap_or_default();
        let duplicate_id = duplicate
            .get("ids")
            .and_then(|ids| ids.get(0))
            .and_then(|id| id.as_str())
            .unwrap_or_default()
            .to_owned();
        assert_ne!(id.as_str(), duplicate_id);
        assert_eq!(editor.doc.shapes().len(), 2);

        editor
            .dispatch(ids::EDIT_COPY, &json!({}))
            .unwrap_or_default();
        editor
            .dispatch(ids::EDIT_PASTE, &json!({}))
            .unwrap_or_default();
        assert_eq!(editor.doc.shapes().len(), 3);
    }

    #[test]
    fn align_and_distribute_are_deterministic() {
        let mut editor = editor();
        let first = add_rect(&mut editor, 0.0, 0.0, 10.0, 10.0);
        let second = add_rect(&mut editor, 50.0, 20.0, 10.0, 10.0);
        let third = add_rect(&mut editor, 100.0, 40.0, 10.0, 10.0);
        editor.selection.select(first.clone());
        editor.selection.extend([second.clone(), third.clone()]);
        editor
            .dispatch(ids::EDIT_ALIGN, &json!({"edge":"left"}))
            .unwrap_or_default();
        let lefts: Vec<f64> = editor
            .doc
            .shapes()
            .iter()
            .map(|shape| shape.pos.x)
            .collect();
        assert!(
            lefts
                .windows(2)
                .all(|pair| (pair[0] - pair[1]).abs() < 1.0e-9)
        );

        editor
            .dispatch(ids::EDIT_DISTRIBUTE, &json!({"axis":"vertical"}))
            .unwrap_or_default();
        assert!(editor.doc.shapes().len() == 3);
    }

    #[test]
    fn select_command_can_target_shape_ids() {
        let mut editor = editor();
        let first = add_rect(&mut editor, 0.0, 0.0, 10.0, 10.0);
        let second = add_rect(&mut editor, 40.0, 0.0, 10.0, 10.0);
        let value = editor
            .dispatch(
                ids::EDIT_SELECT,
                &json!({"ids": [first.as_str(), second.as_str()]}),
            )
            .unwrap_or_default();
        assert_eq!(value.get("count").and_then(Value::as_u64), Some(2));
        editor
            .dispatch(ids::EDIT_SELECT, &json!({"additive": false}))
            .unwrap_or_default();
        assert!(editor.selection.is_empty());
    }

    #[test]
    fn page_commands_work() {
        let mut editor = editor();
        let value = editor
            .dispatch(ids::PAGE_ADD, &json!({"name":"Second"}))
            .unwrap_or_default();
        let page_id = value
            .get("id")
            .and_then(|id| id.as_str())
            .unwrap_or_default()
            .to_owned();
        assert!(page_id.starts_with("page-"));
        let pages = editor
            .dispatch(ids::PAGE_LIST, &json!({}))
            .unwrap_or_default();
        assert_eq!(
            pages
                .get("pages")
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn arrow_binding_follows_moved_shape() {
        let mut editor = editor();
        let left = add_rect(&mut editor, 0.0, 0.0, 100.0, 100.0);
        let right = add_rect(&mut editor, 300.0, 0.0, 100.0, 100.0);
        let arrow = editor
            .add_arrow_between(Vec2::new(100.0, 50.0), Vec2::new(300.0, 50.0), true)
            .unwrap_or_else(|_| ShapeId::from("arrow"));
        editor
            .dispatch(
                ids::SHAPE_BIND,
                &json!({"id":arrow.as_str(),"start":left.as_str(),"end":right.as_str()}),
            )
            .unwrap_or_default();
        let before = editor
            .doc
            .shape(&arrow)
            .map(|shape| shape.endpoints())
            .unwrap_or_default();
        editor.selection.select(left.clone());
        editor
            .dispatch(ids::EDIT_MOVE, &json!({"dx":-40.0,"dy":0.0}))
            .unwrap_or_default();
        let after = editor
            .doc
            .shape(&arrow)
            .map(|shape| shape.endpoints())
            .unwrap_or_default();
        assert!((before.0.x - after.0.x).abs() > 1.0);
    }

    #[test]
    fn duplicating_a_bound_arrow_remaps_to_the_copied_targets() {
        let mut editor = editor();
        let left = add_rect(&mut editor, 0.0, 0.0, 80.0, 80.0);
        let right = add_rect(&mut editor, 200.0, 0.0, 80.0, 80.0);
        let arrow = editor
            .add_arrow_between(Vec2::new(80.0, 40.0), Vec2::new(200.0, 40.0), false)
            .unwrap_or_else(|_| ShapeId::from("arrow"));
        editor
            .dispatch(
                ids::SHAPE_BIND,
                &json!({"id": arrow.as_str(), "start": left.as_str(), "end": right.as_str()}),
            )
            .unwrap_or_default();
        editor
            .dispatch(ids::EDIT_SELECT_ALL, &json!({}))
            .unwrap_or_default();
        let value = editor
            .dispatch(ids::EDIT_DUPLICATE, &json!({}))
            .unwrap_or_default();
        let duplicated: Vec<ShapeId> = value
            .get("ids")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ShapeId::from)
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(duplicated.len(), 3);
        let copied_arrow = duplicated
            .iter()
            .filter_map(|id| editor.doc.shape(id))
            .find(|shape| shape.kind == ShapeKind::Arrow)
            .unwrap_or_else(|| panic!("no copied arrow"));
        let start = copied_arrow.arrow_start.clone().unwrap_or_default();
        let end = copied_arrow.arrow_end.clone().unwrap_or_default();
        assert!(duplicated.contains(&start));
        assert!(duplicated.contains(&end));
        assert!(editor.doc.validate().is_ok());
    }

    #[test]
    fn pasting_an_arrow_without_its_target_clears_the_binding() {
        let mut editor = editor();
        let left = add_rect(&mut editor, 0.0, 0.0, 80.0, 80.0);
        let right = add_rect(&mut editor, 200.0, 0.0, 80.0, 80.0);
        let arrow = editor
            .add_arrow_between(Vec2::new(80.0, 40.0), Vec2::new(200.0, 40.0), false)
            .unwrap_or_else(|_| ShapeId::from("arrow"));
        editor
            .dispatch(
                ids::SHAPE_BIND,
                &json!({"id": arrow.as_str(), "start": left.as_str(), "end": right.as_str()}),
            )
            .unwrap_or_default();
        editor.selection.select(arrow.clone());
        editor
            .dispatch(ids::EDIT_COPY, &json!({}))
            .unwrap_or_default();
        let value = editor
            .dispatch(ids::EDIT_PASTE, &json!({}))
            .unwrap_or_default();
        let pasted = value
            .get("ids")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .map(ShapeId::from)
            .unwrap_or_default();
        let shape = editor
            .doc
            .shape(&pasted)
            .unwrap_or_else(|| panic!("pasted arrow missing"));
        assert!(shape.arrow_start.is_none());
        assert!(shape.arrow_end.is_none());
        assert!(editor.doc.validate().is_ok());
    }

    #[test]
    fn shift_click_toggles_ungrouped_and_grouped_shapes() {
        let mut editor = editor();
        let a = add_rect(&mut editor, 0.0, 0.0, 40.0, 40.0);
        let b = add_rect(&mut editor, 80.0, 0.0, 40.0, 40.0);
        let c = add_rect(&mut editor, 160.0, 0.0, 40.0, 40.0);

        editor.select_at(Vec2::new(20.0, 20.0), false, 1.0);
        assert_eq!(editor.selection.ids(), std::slice::from_ref(&a));
        editor.select_at(Vec2::new(20.0, 20.0), true, 1.0);
        assert!(
            editor.selection.is_empty(),
            "shift-click toggles ungrouped off"
        );

        editor.selection.set_existing(
            [a.clone(), b.clone()],
            &editor
                .doc
                .shapes()
                .iter()
                .map(|s| s.id.clone())
                .collect::<Vec<_>>(),
        );
        editor
            .dispatch(ids::EDIT_GROUP, &json!({}))
            .unwrap_or_default();
        assert_eq!(editor.selection.len(), 2);
        // Shift-clicking one group member should deselect the whole group.
        editor.select_at(Vec2::new(20.0, 20.0), true, 1.0);
        assert!(
            editor.selection.is_empty(),
            "shift-click toggles grouped unit off"
        );
        // Shift-click again selects the whole group.
        editor.select_at(Vec2::new(20.0, 20.0), true, 1.0);
        assert!(editor.selection.contains(&a));
        assert!(editor.selection.contains(&b));
        assert!(!editor.selection.contains(&c));
    }

    #[test]
    fn marquee_expands_groups_and_shift_toggles_units() {
        let mut editor = editor();
        let a = add_rect(&mut editor, 0.0, 0.0, 40.0, 40.0);
        let b = add_rect(&mut editor, 80.0, 0.0, 40.0, 40.0);
        let c = add_rect(&mut editor, 200.0, 0.0, 40.0, 40.0);
        // add_shape selects the newest shape; replace selection before grouping.
        editor.selection.set_existing(
            [a.clone(), b.clone()],
            &editor
                .doc
                .shapes()
                .iter()
                .map(|s| s.id.clone())
                .collect::<Vec<_>>(),
        );
        editor
            .dispatch(ids::EDIT_GROUP, &json!({}))
            .unwrap_or_default();
        editor.select_none();

        // Touch only shape A — whole group should select.
        editor.brush_select(Rect::from_xywh(0.0, 0.0, 30.0, 30.0), false);
        assert!(editor.selection.contains(&a));
        assert!(editor.selection.contains(&b));
        assert!(!editor.selection.contains(&c));

        // Additive toggle of the same unit deselects the group.
        editor.brush_select(Rect::from_xywh(0.0, 0.0, 30.0, 30.0), true);
        assert!(editor.selection.is_empty());
    }

    #[test]
    fn lasso_selects_only_fully_contained_bounds() {
        let mut editor = editor();
        let inside = add_rect(&mut editor, 20.0, 20.0, 30.0, 30.0);
        let overhang = add_rect(&mut editor, 80.0, 80.0, 40.0, 40.0);
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 100.0),
        ];
        editor.lasso_select(&points, false);
        assert!(editor.selection.contains(&inside));
        assert!(
            !editor.selection.contains(&overhang),
            "lasso must require full containment"
        );

        // Touch marquee still selects overhang.
        editor.brush_select(Rect::from_xywh(90.0, 90.0, 20.0, 20.0), false);
        assert!(editor.selection.contains(&overhang));
    }

    #[test]
    fn lasso_rejects_concave_notch_into_shape() {
        let mut editor = editor();
        // A sits under the notch; B sits clear of it on the right.
        let a = add_rect(&mut editor, 20.0, 20.0, 60.0, 40.0);
        let b = add_rect(&mut editor, 120.0, 20.0, 40.0, 40.0);
        let notch = [
            Vec2::new(0.0, 0.0),
            Vec2::new(45.0, 0.0),
            Vec2::new(50.0, 40.0),
            Vec2::new(55.0, 0.0),
            Vec2::new(200.0, 0.0),
            Vec2::new(200.0, 100.0),
            Vec2::new(0.0, 100.0),
        ];
        let count = editor.lasso_select(&notch, false);
        assert_eq!(count, 1);
        assert!(
            !editor.selection.contains(&a),
            "notch bites into A, so A is not fully contained"
        );
        assert!(editor.selection.contains(&b));
    }

    #[test]
    fn select_first_and_last_follow_z_order() {
        let mut editor = editor();
        let bottom = add_rect(&mut editor, 0.0, 0.0, 20.0, 20.0);
        let mid = add_rect(&mut editor, 40.0, 0.0, 20.0, 20.0);
        let top = add_rect(&mut editor, 80.0, 0.0, 20.0, 20.0);
        // Raise top to front explicitly via zorder.
        editor.selection.select(top.clone());
        editor
            .dispatch(ids::EDIT_ZORDER, &json!({"action": "front"}))
            .unwrap_or_default();
        editor.selection.select(bottom.clone());
        editor
            .dispatch(ids::EDIT_ZORDER, &json!({"action": "back"}))
            .unwrap_or_default();

        editor
            .dispatch(ids::EDIT_SELECT_FIRST, &json!({}))
            .unwrap_or_default();
        assert_eq!(editor.selection.ids(), std::slice::from_ref(&bottom));
        editor
            .dispatch(ids::EDIT_SELECT_LAST, &json!({}))
            .unwrap_or_default();
        assert_eq!(editor.selection.ids(), std::slice::from_ref(&top));
        let _ = mid;
    }

    #[test]
    fn edit_select_additive_filters_nonexistent_ids_and_expands_groups() {
        let mut editor = editor();
        let a = add_rect(&mut editor, 0.0, 0.0, 20.0, 20.0);
        let b = add_rect(&mut editor, 40.0, 0.0, 20.0, 20.0);
        editor.selection.set_existing(
            [a.clone(), b.clone()],
            &editor
                .doc
                .shapes()
                .iter()
                .map(|s| s.id.clone())
                .collect::<Vec<_>>(),
        );
        editor
            .dispatch(ids::EDIT_GROUP, &json!({}))
            .unwrap_or_default();
        editor.select_none();

        editor
            .dispatch(
                ids::EDIT_SELECT,
                &json!({"ids": [a.as_str(), "shape-does-not-exist"], "additive": true}),
            )
            .unwrap_or_default();
        assert!(editor.selection.contains(&a));
        assert!(
            editor.selection.contains(&b),
            "group expands on edit.select"
        );
        assert_eq!(editor.selection.len(), 2);
        assert!(
            !editor
                .selection
                .ids()
                .iter()
                .any(|id| id.as_str() == "shape-does-not-exist")
        );
    }

    #[test]
    fn edit_select_lasso_command_works() {
        let mut editor = editor();
        let id = add_rect(&mut editor, 10.0, 10.0, 20.0, 20.0);
        let value = editor
            .dispatch(
                ids::EDIT_SELECT_LASSO,
                &json!({"points": [[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]]}),
            )
            .unwrap_or_default();
        assert_eq!(value.get("count").and_then(Value::as_u64), Some(1));
        assert!(editor.selection.contains(&id));
    }

    #[test]
    fn edit_select_lasso_command_rejects_concave_notch() {
        let mut editor = editor();
        let a = add_rect(&mut editor, 20.0, 20.0, 60.0, 40.0);
        let value = editor
            .dispatch(
                ids::EDIT_SELECT_LASSO,
                &json!({"points": [
                    [0.0, 0.0], [45.0, 0.0], [50.0, 40.0], [55.0, 0.0],
                    [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]
                ]}),
            )
            .unwrap_or_default();
        assert_eq!(value.get("count").and_then(Value::as_u64), Some(0));
        assert!(!editor.selection.contains(&a));
    }

    #[test]
    fn every_editor_command_is_handled_by_dispatch() {
        let mut editor = editor();
        let registry = Registry::new();
        for command in registry
            .all()
            .iter()
            .filter(|command| command.scope == Scope::Editor)
        {
            if let Err(error) = editor.dispatch(command.id, &json!({})) {
                assert!(
                    !matches!(error, EditError::UnknownCommand(_)),
                    "registered command {} is not handled",
                    command.id
                );
            }
        }
    }

    #[test]
    fn categories_are_covered_by_registry() {
        let registry = Registry::new();
        for category in Category::ALL {
            assert!(registry.by_category(category).next().is_some());
        }
    }
}
