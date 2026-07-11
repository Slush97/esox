use std::cell::Cell;

use esox_ui::frame_core::{
    Axis, CommittedScene, DeterministicMeasurer, Element, FrameCore, GridTrack, LogicalPoint,
    LogicalRect, LogicalSize, NullSceneConsumer, PaintPrimitive, PointerEventKind,
    SemanticProperties, SemanticRole, SemanticSnapshot, WidgetId,
};

const ROOT: WidgetId = WidgetId(1);
const SIDEBAR: WidgetId = WidgetId(2);
const CONTENT: WidgetId = WidgetId(3);
const GRID: WidgetId = WidgetId(4);
const TEXT: WidgetId = WidgetId(5);
const IMAGE: WidgetId = WidgetId(6);
const INSERTED: WidgetId = WidgetId(7);
const SCROLL: WidgetId = WidgetId(8);
const SCROLL_CONTENT: WidgetId = WidgetId(9);
const ROW_A: WidgetId = WidgetId(10);
const ROW_B: WidgetId = WidgetId(11);
const OVERLAY: WidgetId = WidgetId(12);
const OVERLAY_ACTION: WidgetId = WidgetId(13);

fn representative_scene() -> Element {
    Element::flex(ROOT, Axis::Row, 0.0)
        .without_paint()
        .with_children(vec![
            Element::fixed(SIDEBAR, 80.0, 120.0).interactive(),
            Element::flex(CONTENT, Axis::Column, 0.0)
                .without_paint()
                .with_flex_grow(1.0)
                .with_children(vec![Element::grid(
                    GRID,
                    vec![GridTrack::Points(72.0), GridTrack::Fraction(1.0)],
                    0.0,
                )
                .without_paint()
                .with_children(vec![
                    Element::image(IMAGE, 7),
                    Element::text(TEXT, "current frame contract text").interactive(),
                ])]),
        ])
}

fn without_generation(mut scene: CommittedScene) -> CommittedScene {
    scene.generation = 0;
    scene
}

#[test]
fn unchanged_first_and_second_frames_match() {
    let declarations = Cell::new(0);
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(7, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(320.0, 120.0));

    for _ in 0..2 {
        core.run_frame(&measurer, &mut consumer, |_| {
            declarations.set(declarations.get() + 1);
            representative_scene()
        })
        .unwrap();
    }

    assert_eq!(declarations.get(), 2);
    assert_eq!(consumer.scenes().len(), 2);
    assert_eq!(
        without_generation(consumer.scenes()[0].clone()),
        without_generation(consumer.scenes()[1].clone())
    );
}

#[test]
fn application_closure_is_never_replayed() {
    let declarations = Cell::new(0);
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(7, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(320.0, 120.0));

    core.run_frame(&measurer, &mut consumer, |state| {
        declarations.set(declarations.get() + 1);
        assert_eq!(state.insert(ROOT, 1), None);
        representative_scene()
    })
    .unwrap();

    assert_eq!(declarations.get(), 1);
    assert_eq!(core.committed_scene().unwrap().generation, 1);
    assert_eq!(consumer.scenes().len(), 1);
}

#[test]
fn resize_is_correct_in_first_post_resize_frame() {
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(7, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(320.0, 120.0));

    core.run_frame(&measurer, &mut consumer, |_| representative_scene())
        .unwrap();
    core.resize(LogicalSize::new(200.0, 120.0));
    core.run_frame(&measurer, &mut consumer, |_| representative_scene())
        .unwrap();

    let scene = core.committed_scene().unwrap();
    let content = scene.node(CONTENT).unwrap();
    let grid = scene.node(GRID).unwrap();
    let text = scene.node(TEXT).unwrap();
    assert_eq!(scene.generation, 2);
    assert_eq!(scene.viewport, LogicalSize::new(200.0, 120.0));
    assert_eq!(content.bounds.width, 120.0);
    assert_eq!(grid.bounds.width, 120.0);
    assert_eq!(text.bounds.width, 48.0);
    assert_eq!(text.bounds.height, 90.0);
    assert_eq!(text.paint_bounds, Some(text.bounds));
    assert_eq!(text.hit_bounds, Some(text.bounds));
    assert_eq!(text.semantic_bounds, Some(text.bounds));
    assert_eq!(text.current_damage_bounds, Some(text.bounds));
    assert_eq!(text.effective_clip.unwrap().width, 200.0);
}

#[test]
fn structural_changes_have_no_stale_geometry() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(240.0, 60.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Row, 0.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(SIDEBAR, 40.0, 60.0).interactive(),
                Element::fixed(CONTENT, 80.0, 60.0).interactive(),
                Element::fixed(TEXT, 120.0, 60.0).interactive(),
            ])
    })
    .unwrap();

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Row, 0.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(TEXT, 120.0, 60.0).interactive(),
                Element::fixed(INSERTED, 40.0, 60.0).interactive(),
                Element::fixed(SIDEBAR, 80.0, 60.0).interactive(),
            ])
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    assert_eq!(scene.generation, 2);
    assert!(scene.node(CONTENT).is_none());
    assert_eq!(
        scene.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
        vec![ROOT, TEXT, INSERTED, SIDEBAR]
    );

    let text = scene.node(TEXT).unwrap();
    let inserted = scene.node(INSERTED).unwrap();
    let sidebar = scene.node(SIDEBAR).unwrap();
    assert_eq!(text.bounds.x, 0.0);
    assert_eq!(inserted.bounds.x, 120.0);
    assert_eq!(sidebar.bounds.x, 160.0);
    for node in [text, inserted, sidebar] {
        assert_eq!(node.paint_bounds, Some(node.bounds));
        assert_eq!(node.hit_bounds, Some(node.bounds));
        assert_eq!(node.semantic_bounds, Some(node.bounds));
        assert_eq!(node.current_damage_bounds, Some(node.bounds));
        assert_eq!(
            node.effective_clip,
            Some(LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 240.0,
                height: 60.0,
            })
        );
    }
}

#[test]
fn metric_changes_reflow_and_paint_together() {
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 120.0));

    core.run_frame(
        &DeterministicMeasurer::new(4.0, 10.0),
        &mut consumer,
        |_| {
            Element::flex(ROOT, Axis::Column, 0.0)
                .without_paint()
                .with_children(vec![Element::text(TEXT, "metric change").interactive()])
        },
    )
    .unwrap();
    core.run_frame(
        &DeterministicMeasurer::new(8.0, 20.0),
        &mut consumer,
        |_| {
            Element::flex(ROOT, Axis::Column, 0.0)
                .without_paint()
                .with_children(vec![Element::text(TEXT, "metric change").interactive()])
        },
    )
    .unwrap();

    let first = consumer.scenes()[0].node(TEXT).unwrap();
    let second = consumer.scenes()[1].node(TEXT).unwrap();
    assert_eq!(
        first.bounds,
        LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 10.0,
        }
    );
    assert_eq!(
        second.bounds,
        LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 40.0,
        }
    );
    assert_eq!(second.paint_bounds, Some(second.bounds));
    assert_eq!(second.hit_bounds, Some(second.bounds));
    assert_eq!(second.semantic_bounds, Some(second.bounds));
    assert_eq!(second.current_damage_bounds, Some(second.bounds));
}

#[test]
fn one_node_supplies_all_resolved_bounds() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(90.0, 30.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::fixed(ROOT, 90.0, 30.0)
            .interactive()
            .clip_children()
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    let node = scene.node(ROOT).unwrap();
    assert_eq!(scene.generation, 1);
    assert_eq!(node.paint_bounds, Some(node.bounds));
    assert_eq!(node.hit_bounds, Some(node.bounds));
    assert_eq!(node.semantic_bounds, Some(node.bounds));
    assert_eq!(node.effective_clip, Some(node.bounds));
    assert_eq!(node.current_damage_bounds, Some(node.bounds));

    let paint = &scene.display_list[0];
    assert_eq!(paint.id, ROOT);
    assert_eq!(paint.primitive, PaintPrimitive::Box);
    assert_eq!(paint.bounds, node.bounds);
    assert_eq!(paint.effective_clip, node.effective_clip);

    let hit = &scene.hit_index[0];
    assert_eq!(hit.id, ROOT);
    assert_eq!(hit.bounds, node.bounds);
    assert_eq!(hit.effective_clip, node.effective_clip);

    let semantic = scene.semantics.node(ROOT).unwrap();
    assert_eq!(scene.semantics.roots, vec![ROOT]);
    assert_eq!(semantic.properties.role, SemanticRole::Generic);
    assert_eq!(semantic.bounds, node.bounds);
    assert_eq!(semantic.effective_clip, node.effective_clip);

    let damage = &scene.damage[0];
    assert_eq!(damage.id, ROOT);
    assert_eq!(damage.current_bounds, node.bounds);
    assert_eq!(damage.effective_clip, node.effective_clip);

    fn assert_serializable<T: serde::Serialize + for<'de> serde::Deserialize<'de>>() {}
    assert_serializable::<SemanticSnapshot>();
}

#[test]
fn scroll_offset_is_current_frame_state() {
    fn scene(scroll_y: f32) -> Element {
        Element::fixed(ROOT, 100.0, 60.0)
            .without_paint()
            .with_children(vec![Element::fixed(SCROLL, 100.0, 40.0)
                .without_paint()
                .clip_children()
                .with_scroll_offset(0.0, scroll_y)
                .with_children(vec![Element::flex(SCROLL_CONTENT, Axis::Column, 0.0)
                    .without_paint()
                    .with_children(vec![
                        Element::fixed(ROW_A, 100.0, 30.0).interactive(),
                        Element::fixed(ROW_B, 100.0, 30.0).interactive(),
                    ])])])
    }

    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    core.run_frame(&measurer, &mut consumer, |_| scene(5.0))
        .unwrap();
    core.run_frame(&measurer, &mut consumer, |_| scene(25.0))
        .unwrap();

    let first_row = consumer.scenes()[0].node(ROW_B).unwrap();
    let second_scene = &consumer.scenes()[1];
    let second_row = second_scene.node(ROW_B).unwrap();
    assert_eq!(first_row.bounds.y, 25.0);
    assert_eq!(second_row.bounds.y, 5.0);
    assert_eq!(second_row.paint_bounds, Some(second_row.bounds));
    assert_eq!(second_row.hit_bounds, Some(second_row.bounds));
    assert_eq!(second_row.semantic_bounds, Some(second_row.bounds));
    assert_eq!(second_row.current_damage_bounds, Some(second_row.bounds));
    assert_eq!(
        second_row.effective_clip,
        second_scene.node(SCROLL).map(|node| node.bounds)
    );
    assert_eq!(
        second_scene
            .hit_test(LogicalPoint::new(10.0, 10.0))
            .map(|node| node.id),
        Some(ROW_B)
    );
    assert_eq!(
        second_scene
            .hit_test(LogicalPoint::new(10.0, 45.0))
            .map(|node| node.id),
        None
    );
}

#[test]
fn blocking_overlay_owns_topmost_hit() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 120.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::fixed(ROOT, 200.0, 120.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(CONTENT, 200.0, 120.0).interactive(),
                Element::fixed(OVERLAY, 100.0, 60.0)
                    .with_absolute_position(20.0, 20.0)
                    .blocking_overlay()
                    .clip_children()
                    .with_children(vec![Element::fixed(OVERLAY_ACTION, 40.0, 20.0)
                        .interactive()
                        .with_semantics(
                            SemanticProperties::new(SemanticRole::Button).with_label("Action"),
                        )]),
            ])
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    let overlay = scene.node(OVERLAY).unwrap();
    assert_eq!(
        scene
            .hit_test(LogicalPoint::new(90.0, 50.0))
            .map(|node| node.id),
        Some(OVERLAY)
    );
    assert!(overlay.blocks_input);
    assert_eq!(overlay.semantic_bounds, Some(overlay.bounds));
    assert_eq!(
        overlay.effective_clip.unwrap(),
        scene.node(ROOT).unwrap().bounds
    );
    assert_eq!(scene.focus_order, vec![OVERLAY, OVERLAY_ACTION]);
    assert_eq!(scene.focus_scopes.len(), 1);
    assert_eq!(scene.focus_scopes[0].owner, OVERLAY);
    assert_eq!(scene.focus_scopes[0].members, scene.focus_order);

    let overlay_semantics = scene.semantics.node(OVERLAY).unwrap();
    let action_semantics = scene.semantics.node(OVERLAY_ACTION).unwrap();
    assert_eq!(overlay_semantics.children, vec![OVERLAY_ACTION]);
    assert_eq!(action_semantics.parent, Some(OVERLAY));
    assert_eq!(action_semantics.properties.role, SemanticRole::Button);
    assert_eq!(action_semantics.properties.label.as_deref(), Some("Action"));
    assert_eq!(
        action_semantics.bounds,
        scene.node(OVERLAY_ACTION).unwrap().bounds
    );
}

#[test]
fn input_targets_committed_generation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Row, 0.0)
            .without_paint()
            .with_children(vec![Element::fixed(CONTENT, 50.0, 40.0).interactive()])
    })
    .unwrap();

    core.queue_pointer_press(LogicalPoint::new(25.0, 20.0));
    core.queue_pointer_press(LogicalPoint::new(75.0, 20.0));
    core.run_frame(&measurer, &mut consumer, |state| {
        let response = state.take_response(CONTENT).unwrap();
        assert_eq!(response.target, CONTENT);
        assert_eq!(response.committed_generation, 1);
        assert_eq!(response.target_bounds.width, 50.0);
        assert_eq!(state.take_response(CONTENT), None);
        assert_eq!(state.take_response(INSERTED), None);
        Element::flex(ROOT, Axis::Row, 0.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(CONTENT, 50.0, 40.0).interactive(),
                Element::fixed(INSERTED, 50.0, 40.0).interactive(),
            ])
    })
    .unwrap();

    core.queue_pointer_press(LogicalPoint::new(75.0, 20.0));
    core.run_frame(&measurer, &mut consumer, |state| {
        let response = state.take_response(INSERTED).unwrap();
        assert_eq!(response.target, INSERTED);
        assert_eq!(response.committed_generation, 2);
        assert_eq!(state.take_response(INSERTED), None);
        Element::flex(ROOT, Axis::Row, 0.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(CONTENT, 50.0, 40.0).interactive(),
                Element::fixed(INSERTED, 50.0, 40.0).interactive(),
            ])
    })
    .unwrap();

    assert_eq!(core.committed_scene().unwrap().generation, 3);
}

#[test]
fn removed_capture_is_cancelled() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 120.0));

    core.run_frame(&measurer, &mut consumer, |state| {
        state.request_keyboard_focus(CONTENT);
        Element::fixed(ROOT, 200.0, 120.0)
            .without_paint()
            .with_children(vec![Element::fixed(CONTENT, 200.0, 120.0).interactive()])
    })
    .unwrap();
    assert_eq!(core.keyboard_focus(), Some(CONTENT));

    core.run_frame(&measurer, &mut consumer, |state| {
        state.request_keyboard_focus(OVERLAY_ACTION);
        state.request_pointer_capture(7, OVERLAY_ACTION);
        Element::fixed(ROOT, 200.0, 120.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(CONTENT, 200.0, 120.0).interactive(),
                Element::fixed(OVERLAY, 100.0, 60.0)
                    .with_absolute_position(20.0, 20.0)
                    .blocking_overlay()
                    .with_children(vec![
                        Element::fixed(OVERLAY_ACTION, 40.0, 20.0).interactive()
                    ]),
            ])
    })
    .unwrap();
    assert_eq!(core.keyboard_focus(), Some(OVERLAY_ACTION));
    assert_eq!(core.pointer_capture(7), Some(OVERLAY_ACTION));

    core.queue_pointer_event(PointerEventKind::Move, 7, LogicalPoint::new(190.0, 110.0));
    core.run_frame(&measurer, &mut consumer, |state| {
        let captured_move = state.take_response(OVERLAY_ACTION).unwrap();
        assert_eq!(captured_move.kind, PointerEventKind::Move);
        assert_eq!(captured_move.committed_generation, 2);
        assert_eq!(captured_move.position, LogicalPoint::new(190.0, 110.0));
        Element::fixed(ROOT, 200.0, 120.0)
            .without_paint()
            .with_children(vec![Element::fixed(CONTENT, 200.0, 120.0).interactive()])
    })
    .unwrap();

    assert_eq!(core.pointer_capture(7), None);
    assert_eq!(core.keyboard_focus(), Some(CONTENT));
    let cancellation = core.take_cancellation().unwrap();
    assert_eq!(cancellation.kind, PointerEventKind::Cancel);
    assert_eq!(cancellation.pointer, 7);
    assert_eq!(cancellation.target, OVERLAY_ACTION);
    assert_eq!(cancellation.committed_generation, 2);
    assert_eq!(cancellation.target_bounds.width, 40.0);
    assert_eq!(core.take_cancellation(), None);
}

#[test]
fn headless_pipeline_has_no_platform_or_gpu() {
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(7, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(320.0, 120.0));

    core.run_frame(&measurer, &mut consumer, |_| representative_scene())
        .unwrap();

    assert_eq!(consumer.scenes().len(), 1);
    assert_eq!(consumer.scenes()[0].generation, 1);
    assert_eq!(consumer.scenes()[0].viewport.width, 320.0);
}

#[test]
fn two_window_contexts_are_isolated() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut left_consumer = NullSceneConsumer::default();
    let mut right_consumer = NullSceneConsumer::default();
    let mut left = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut right = FrameCore::new(LogicalSize::new(240.0, 80.0));

    left.run_frame(&measurer, &mut left_consumer, |state| {
        state.insert(CONTENT, 11);
        state.request_keyboard_focus(CONTENT);
        state.request_pointer_capture(1, CONTENT);
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();
    right
        .run_frame(&measurer, &mut right_consumer, |state| {
            state.insert(CONTENT, 22);
            state.request_keyboard_focus(CONTENT);
            Element::fixed(CONTENT, 240.0, 80.0).interactive()
        })
        .unwrap();
    left.run_frame(&measurer, &mut left_consumer, |state| {
        assert_eq!(state.get(CONTENT), Some(11));
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();

    assert_eq!(left.committed_scene().unwrap().generation, 2);
    assert_eq!(right.committed_scene().unwrap().generation, 1);
    assert_eq!(left.committed_scene().unwrap().viewport.width, 100.0);
    assert_eq!(right.committed_scene().unwrap().viewport.width, 240.0);
    assert_eq!(left.pointer_capture(1), Some(CONTENT));
    assert_eq!(right.pointer_capture(1), None);
    assert_eq!(left.keyboard_focus(), Some(CONTENT));
    assert_eq!(right.keyboard_focus(), Some(CONTENT));
    assert_eq!(left_consumer.scenes().len(), 2);
    assert_eq!(right_consumer.scenes().len(), 1);
}
