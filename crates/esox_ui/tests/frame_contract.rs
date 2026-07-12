use std::cell::Cell;

use esox_ui::frame_core::{
    AvailableLength, Axis, Color, CommittedScene, CrossAxisAlignment, DeterministicMeasurer,
    Element, FrameCore, FrameError, GridDeclarationError, GridTrack, ImageMeasureRequest,
    IntrinsicMeasurer, LogicalPoint, LogicalRect, LogicalSize, MainAxisAlignment,
    NullSceneConsumer, PaintPrimitive, PointerEventKind, SemanticProperties, SemanticRole,
    SemanticSnapshot, TextDirection, TextMeasureRequest, TextProperties, WidgetId,
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
const VISIBILITY_CONTAINER: WidgetId = WidgetId(14);
const VISIBILITY_GRID: WidgetId = WidgetId(15);
const VISIBILITY_ACTION: WidgetId = WidgetId(16);
const VISIBILITY_SIBLING: WidgetId = WidgetId(17);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn representative_text_properties() -> TextProperties {
    TextProperties {
        font_family: Some("Esox Sans".into()),
        font_size: 16.0,
        font_weight: 500,
        locale: Some("en-US".into()),
        direction: TextDirection::LeftToRight,
    }
}

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
                    vec![GridTrack::Fixed(72.0), GridTrack::Fraction(1.0)],
                    0.0,
                )
                .without_paint()
                .with_children(vec![
                    Element::image(IMAGE, 7),
                    Element::text_with_properties(
                        TEXT,
                        "current frame contract text",
                        representative_text_properties(),
                    )
                    .interactive(),
                ])]),
        ])
}

fn without_generation(mut scene: CommittedScene) -> CommittedScene {
    scene.generation = 0;
    scene
}

fn participation_scene(hidden: bool, disabled: bool) -> Element {
    Element::flex(ROOT, Axis::Row, 0.0)
        .without_paint()
        .with_children(vec![
            Element::flex(VISIBILITY_CONTAINER, Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(60.0), Some(40.0))
                .with_hidden(hidden)
                .with_disabled(disabled)
                .with_children(vec![Element::grid(
                    VISIBILITY_GRID,
                    vec![GridTrack::Fraction(1.0)],
                    0.0,
                )
                .without_paint()
                .with_children(vec![Element::fixed(VISIBILITY_ACTION, 60.0, 40.0)
                    .with_paint(PaintPrimitive::SolidRect {
                        color: Color::rgba(0.2, 0.4, 0.6, 1.0),
                    })
                    .with_semantics(
                        SemanticProperties::new(SemanticRole::Button).with_label("Action"),
                    )
                    .interactive()])]),
            Element::fixed(VISIBILITY_SIBLING, 40.0, 40.0),
        ])
}

fn retained_scroll_scene(request: Option<LogicalPoint>, content: LogicalSize) -> Element {
    let viewport = Element::flex(SCROLL, Axis::Column, 0.0)
        .without_paint()
        .with_size(Some(100.0), Some(40.0))
        .clip_children()
        .with_children(vec![Element::fixed(
            SCROLL_CONTENT,
            content.width,
            content.height,
        )]);
    let viewport = match request {
        Some(offset) => viewport.with_scroll_offset(offset.x, offset.y),
        None => viewport.scrollable(),
    };
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![viewport])
}

fn retained_nested_scroll_scene() -> Element {
    let inner = Element::flex(SCROLL_CONTENT, Axis::Column, 0.0)
        .without_paint()
        .with_size(Some(100.0), Some(30.0))
        .clip_children()
        .scrollable()
        .with_children(vec![Element::fixed(ROW_A, 100.0, 70.0)]);
    let outer = Element::flex(SCROLL, Axis::Column, 0.0)
        .without_paint()
        .with_size(Some(100.0), Some(60.0))
        .clip_children()
        .scrollable()
        .with_children(vec![inner, Element::fixed(ROW_B, 100.0, 80.0)]);
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![outer])
}

fn retained_scroll_participation_scene(hidden: bool, disabled: bool) -> Element {
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(40.0))
            .clip_children()
            .scrollable()
            .with_hidden(hidden)
            .with_disabled(disabled)
            .with_children(vec![Element::fixed(SCROLL_CONTENT, 100.0, 100.0)])])
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
fn hidden_collapses_nested_layout_and_restores_on_the_first_frame() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let visible = core
        .run_frame(&measurer, &mut consumer, |_| {
            participation_scene(false, false)
        })
        .unwrap()
        .clone();
    assert_eq!(
        visible.node(VISIBILITY_ACTION).unwrap().bounds,
        LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 40.0,
        }
    );
    assert_eq!(visible.node(VISIBILITY_SIBLING).unwrap().bounds.x, 60.0);
    assert!(visible
        .node(VISIBILITY_ACTION)
        .unwrap()
        .hit_bounds
        .is_some());

    let hidden = core
        .run_frame(&measurer, &mut consumer, |_| {
            participation_scene(true, false)
        })
        .unwrap()
        .clone();
    for id in [VISIBILITY_CONTAINER, VISIBILITY_GRID, VISIBILITY_ACTION] {
        let node = hidden.node(id).unwrap();
        assert!(node.effective_hidden);
        assert!(!node.effective_disabled);
        assert_eq!(node.bounds, LogicalRect::default());
        assert_eq!(node.paint_bounds, None);
        assert_eq!(node.hit_bounds, None);
        assert_eq!(node.semantic_bounds, None);
        assert_eq!(node.current_damage_bounds, None);
        assert_eq!(node.effective_clip, None);
        assert!(!hidden.display_list.iter().any(|record| record.id == id));
        assert!(!hidden.hit_index.iter().any(|record| record.id == id));
        assert!(hidden.semantics.node(id).is_none());
        assert!(!hidden.damage.iter().any(|record| record.id == id));
        assert!(!hidden.focus_order.contains(&id));
    }
    assert_eq!(hidden.node(VISIBILITY_SIBLING).unwrap().bounds.x, 0.0);

    let restored = core
        .run_frame(&measurer, &mut consumer, |_| {
            participation_scene(false, false)
        })
        .unwrap();
    assert_eq!(
        restored.node(VISIBILITY_ACTION).unwrap().bounds,
        visible.node(VISIBILITY_ACTION).unwrap().bounds
    );
    assert_eq!(restored.node(VISIBILITY_SIBLING).unwrap().bounds.x, 60.0);
    assert!(!restored.node(VISIBILITY_ACTION).unwrap().effective_hidden);
    assert!(restored
        .node(VISIBILITY_ACTION)
        .unwrap()
        .hit_bounds
        .is_some());
    assert!(restored.semantics.node(VISIBILITY_ACTION).is_some());
}

#[test]
fn disabled_inherits_without_changing_layout_or_paint_and_restores_immediately() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let enabled = core
        .run_frame(&measurer, &mut consumer, |state| {
            state.request_keyboard_focus(VISIBILITY_ACTION);
            state.request_pointer_capture(9, VISIBILITY_ACTION);
            participation_scene(false, false)
        })
        .unwrap()
        .clone();
    assert_eq!(core.keyboard_focus(), Some(VISIBILITY_ACTION));
    assert_eq!(core.pointer_capture(9), Some(VISIBILITY_ACTION));

    let disabled = core
        .run_frame(&measurer, &mut consumer, |_| {
            participation_scene(false, true)
        })
        .unwrap()
        .clone();
    let enabled_action = enabled.node(VISIBILITY_ACTION).unwrap();
    let disabled_action = disabled.node(VISIBILITY_ACTION).unwrap();
    assert_eq!(disabled_action.bounds, enabled_action.bounds);
    assert_eq!(disabled_action.paint_bounds, enabled_action.paint_bounds);
    assert_eq!(disabled.display_list, enabled.display_list);
    assert!(
        disabled
            .node(VISIBILITY_CONTAINER)
            .unwrap()
            .effective_disabled
    );
    assert!(disabled.node(VISIBILITY_GRID).unwrap().effective_disabled);
    assert!(disabled_action.effective_disabled);
    assert!(!disabled_action.effective_hidden);
    assert_eq!(disabled_action.hit_bounds, None);
    assert!(!disabled.focus_order.contains(&VISIBILITY_ACTION));
    assert!(
        disabled
            .semantics
            .node(VISIBILITY_ACTION)
            .unwrap()
            .properties
            .disabled
    );
    assert_eq!(core.keyboard_focus(), None);
    assert_eq!(core.pointer_capture(9), None);
    let cancellation = core.take_cancellation().unwrap();
    assert_eq!(cancellation.kind, PointerEventKind::Cancel);
    assert_eq!(cancellation.target, VISIBILITY_ACTION);

    let restored = core
        .run_frame(&measurer, &mut consumer, |_| {
            participation_scene(false, false)
        })
        .unwrap();
    assert_eq!(
        restored.node(VISIBILITY_ACTION).unwrap().bounds,
        enabled_action.bounds
    );
    assert!(restored
        .node(VISIBILITY_ACTION)
        .unwrap()
        .hit_bounds
        .is_some());
    assert!(
        !restored
            .semantics
            .node(VISIBILITY_ACTION)
            .unwrap()
            .properties
            .disabled
    );
    assert_eq!(core.keyboard_focus(), Some(VISIBILITY_ACTION));
}

#[test]
fn current_hidden_or_disabled_state_discards_prior_scene_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();

    for (hidden, disabled) in [(true, false), (false, true)] {
        let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
        core.run_frame(&measurer, &mut consumer, |_| {
            participation_scene(false, false)
        })
        .unwrap();
        core.queue_pointer_event(PointerEventKind::Press, 3, LogicalPoint::new(20.0, 20.0));
        core.run_frame(&measurer, &mut consumer, |_| {
            participation_scene(hidden, disabled)
        })
        .unwrap();
        core.run_frame(&measurer, &mut consumer, |state| {
            assert_eq!(state.take_response(VISIBILITY_ACTION), None);
            participation_scene(hidden, disabled)
        })
        .unwrap();
    }
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
    assert_eq!(second_row.bounds.y, 10.0);
    assert_eq!(
        second_scene.node(SCROLL).unwrap().scroll_metrics,
        Some(esox_ui::frame_core::ScrollMetrics {
            viewport_extent: LogicalSize::new(100.0, 40.0),
            content_extent: LogicalSize::new(100.0, 60.0),
            requested_offset: LogicalPoint::new(0.0, 25.0),
            applied_offset: LogicalPoint::new(0.0, 20.0),
            maximum_offset: LogicalPoint::new(0.0, 20.0),
        })
    );
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
fn scroll_metrics_recompute_for_growth_shrink_and_resize_in_the_current_frame() {
    fn scene(content_height: f32) -> Element {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
                .without_paint()
                .with_flex_grow(1.0)
                .clip_children()
                .with_scroll_offset(0.0, 50.0)
                .with_children(vec![Element::fixed(
                    SCROLL_CONTENT,
                    100.0,
                    content_height,
                )
                .interactive()])])
    }

    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    for (content_height, maximum, applied) in
        [(60.0, 20.0, 20.0), (90.0, 50.0, 50.0), (45.0, 5.0, 5.0)]
    {
        let scene = core
            .run_frame(&measurer, &mut consumer, |_| scene(content_height))
            .unwrap();
        let metrics = scene.node(SCROLL).unwrap().scroll_metrics.unwrap();
        assert_eq!(metrics.content_extent.height, content_height);
        assert_eq!(metrics.maximum_offset.y, maximum);
        assert_eq!(metrics.applied_offset.y, applied);
        assert_eq!(scene.node(SCROLL_CONTENT).unwrap().bounds.y, -applied);
    }

    core.resize(LogicalSize::new(100.0, 55.0));
    let resized = core
        .run_frame(&measurer, &mut consumer, |_| scene(45.0))
        .unwrap();
    let metrics = resized.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.viewport_extent, LogicalSize::new(100.0, 55.0));
    assert_eq!(metrics.content_extent, LogicalSize::new(100.0, 55.0));
    assert_eq!(metrics.maximum_offset, LogicalPoint::default());
    assert_eq!(metrics.applied_offset, LogicalPoint::default());
    assert_eq!(resized.node(SCROLL_CONTENT).unwrap().bounds.y, 0.0);
    assert_eq!(consumer.scenes().len(), 4);
}

#[test]
fn scroll_clamps_both_axes_and_sanitizes_non_finite_offsets() {
    fn scene(requested: LogicalPoint) -> Element {
        Element::fixed(ROOT, 100.0, 40.0)
            .without_paint()
            .with_children(vec![Element::fixed(SCROLL, 100.0, 40.0)
                .without_paint()
                .clip_children()
                .with_scroll_offset(requested.x, requested.y)
                .with_children(vec![Element::fixed(SCROLL_CONTENT, 140.0, 70.0)
                    .with_semantics(SemanticProperties::new(SemanticRole::Generic))
                    .interactive()])])
    }

    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let clamped = core
        .run_frame(&measurer, &mut consumer, |_| {
            scene(LogicalPoint::new(25.0, 50.0))
        })
        .unwrap()
        .clone();
    let metrics = clamped.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.maximum_offset, LogicalPoint::new(40.0, 30.0));
    assert_eq!(metrics.applied_offset, LogicalPoint::new(25.0, 30.0));
    let content = clamped.node(SCROLL_CONTENT).unwrap();
    assert_eq!(content.bounds, rect(-25.0, -30.0, 140.0, 70.0));
    assert_eq!(content.paint_bounds, Some(content.bounds));
    assert_eq!(content.hit_bounds, Some(content.bounds));
    assert_eq!(content.semantic_bounds, Some(content.bounds));
    assert_eq!(content.current_damage_bounds, Some(content.bounds));

    let invalid = core
        .run_frame(&measurer, &mut consumer, |_| {
            scene(LogicalPoint::new(f32::NAN, f32::INFINITY))
        })
        .unwrap();
    let metrics = invalid.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert!(metrics.requested_offset.x.is_nan());
    assert_eq!(metrics.applied_offset, LogicalPoint::new(0.0, 30.0));
    let bounds = invalid.node(SCROLL_CONTENT).unwrap().bounds;
    assert!(bounds.x.is_finite());
    assert!(bounds.y.is_finite());

    let negative = core
        .run_frame(&measurer, &mut consumer, |_| {
            scene(LogicalPoint::new(-10.0, f32::NEG_INFINITY))
        })
        .unwrap();
    assert_eq!(
        negative
            .node(SCROLL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset,
        LogicalPoint::default()
    );
}

#[test]
fn fully_clipped_descendant_keeps_an_explicit_empty_clip() {
    const CLIPPED_CONTAINER: WidgetId = WidgetId(18);
    const CLIPPED_CHILD: WidgetId = WidgetId(19);

    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::fixed(ROOT, 100.0, 60.0)
            .without_paint()
            .with_children(vec![Element::fixed(SCROLL, 100.0, 40.0)
                .without_paint()
                .clip_children()
                .with_children(vec![Element::fixed(CLIPPED_CONTAINER, 100.0, 20.0)
                    .without_paint()
                    .with_absolute_position(0.0, 50.0)
                    .clip_children()
                    .with_children(vec![
                        Element::fixed(CLIPPED_CHILD, 100.0, 20.0).interactive()
                    ])])])
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    let child = scene.node(CLIPPED_CHILD).unwrap();
    assert_eq!(child.bounds, rect(0.0, 50.0, 100.0, 20.0));
    assert_eq!(child.effective_clip, Some(rect(0.0, 50.0, 100.0, 0.0)));
    assert_eq!(
        scene
            .display_list
            .iter()
            .find(|record| record.id == CLIPPED_CHILD)
            .unwrap()
            .effective_clip,
        child.effective_clip
    );
    assert_eq!(scene.hit_test(LogicalPoint::new(10.0, 55.0)), None);
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
fn failed_frame_rolls_back_pointer_dispatch_widget_state_and_interaction_requests() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |state| {
        state.insert(CONTENT, 7);
        state.request_keyboard_focus(CONTENT);
        state.request_pointer_capture(3, CONTENT);
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();
    let committed = core.committed_scene().unwrap().clone();
    assert_eq!(core.keyboard_focus(), Some(CONTENT));
    assert_eq!(core.pointer_capture(3), Some(CONTENT));

    core.queue_pointer_event(PointerEventKind::Press, 4, LogicalPoint::new(10.0, 10.0));
    let error = core
        .run_frame(&measurer, &mut consumer, |state| {
            let response = state.take_response(CONTENT).unwrap();
            assert_eq!(response.committed_generation, committed.generation);
            state.insert(CONTENT, 99);
            state.insert(INSERTED, 123);
            state.request_keyboard_focus(INSERTED);
            state.request_pointer_capture(4, INSERTED);
            state.request_pointer_release(3);
            Element::flex(ROOT, Axis::Row, 0.0).with_children(vec![
                Element::fixed(ROW_A, 50.0, 40.0),
                Element::fixed(ROW_A, 50.0, 40.0),
            ])
        })
        .unwrap_err();

    assert_eq!(error, FrameError::DuplicateWidgetId(ROW_A));
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(consumer.scenes().len(), 1);
    assert_eq!(core.keyboard_focus(), Some(CONTENT));
    assert_eq!(core.pointer_capture(3), Some(CONTENT));
    assert_eq!(core.pointer_capture(4), None);
    assert_eq!(core.take_cancellation(), None);

    core.run_frame(&measurer, &mut consumer, |state| {
        assert_eq!(state.get(CONTENT), Some(7));
        assert_eq!(state.get(INSERTED), None);
        let response = state.take_response(CONTENT).unwrap();
        assert_eq!(response.kind, PointerEventKind::Press);
        assert_eq!(response.pointer, 4);
        assert_eq!(response.committed_generation, committed.generation);
        assert_eq!(state.take_response(CONTENT), None);
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();

    core.run_frame(&measurer, &mut consumer, |state| {
        assert_eq!(state.take_response(CONTENT), None);
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();
    assert_eq!(
        core.committed_scene().unwrap().generation,
        committed.generation + 2
    );
}

#[test]
fn queued_pointer_order_survives_failed_frame_and_successful_retry() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();
    for kind in [
        PointerEventKind::Press,
        PointerEventKind::Move,
        PointerEventKind::Release,
    ] {
        core.queue_pointer_event(kind, 8, LogicalPoint::new(10.0, 10.0));
    }

    let error = core
        .run_frame(&measurer, &mut consumer, |_| {
            Element::flex(ROOT, Axis::Row, 0.0).with_children(vec![
                Element::fixed(ROW_A, 50.0, 40.0),
                Element::fixed(ROW_A, 50.0, 40.0),
            ])
        })
        .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(ROW_A));
    assert_eq!(core.committed_scene().unwrap().generation, 1);

    core.run_frame(&measurer, &mut consumer, |state| {
        let observed = [
            state.take_response(CONTENT).unwrap().kind,
            state.take_response(CONTENT).unwrap().kind,
            state.take_response(CONTENT).unwrap().kind,
        ];
        assert_eq!(
            observed,
            [
                PointerEventKind::Press,
                PointerEventKind::Move,
                PointerEventKind::Release,
            ]
        );
        assert_eq!(state.take_response(CONTENT), None);
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();

    core.run_frame(&measurer, &mut consumer, |state| {
        assert_eq!(state.take_response(CONTENT), None);
        Element::fixed(CONTENT, 100.0, 40.0).interactive()
    })
    .unwrap();
}

#[test]
fn failed_capture_owner_removal_visibility_or_disablement_does_not_cancel() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);

    for (hidden, disabled, removed) in [
        (false, false, true),
        (true, false, false),
        (false, true, false),
    ] {
        let mut consumer = NullSceneConsumer::default();
        let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
        core.run_frame(&measurer, &mut consumer, |state| {
            state.request_pointer_capture(7, CONTENT);
            Element::fixed(CONTENT, 100.0, 40.0).interactive()
        })
        .unwrap();
        assert_eq!(core.pointer_capture(7), Some(CONTENT));

        let capture_owner = || {
            Element::fixed(CONTENT, 100.0, 40.0)
                .interactive()
                .with_hidden(hidden)
                .with_disabled(disabled)
        };
        let error = core
            .run_frame(&measurer, &mut consumer, |_| {
                let mut children = Vec::new();
                if !removed {
                    children.push(capture_owner());
                }
                children.extend([
                    Element::fixed(ROW_A, 10.0, 10.0),
                    Element::fixed(ROW_A, 10.0, 10.0),
                ]);
                Element::flex(ROOT, Axis::Column, 0.0).with_children(children)
            })
            .unwrap_err();
        assert_eq!(error, FrameError::DuplicateWidgetId(ROW_A));
        assert_eq!(core.committed_scene().unwrap().generation, 1);
        assert_eq!(core.pointer_capture(7), Some(CONTENT));
        assert_eq!(core.take_cancellation(), None);

        core.run_frame(&measurer, &mut consumer, |_| {
            let children = if removed {
                Vec::new()
            } else {
                vec![capture_owner()]
            };
            Element::flex(ROOT, Axis::Column, 0.0).with_children(children)
        })
        .unwrap();
        assert_eq!(core.pointer_capture(7), None);
        let cancellation = core.take_cancellation().unwrap();
        assert_eq!(cancellation.kind, PointerEventKind::Cancel);
        assert_eq!(cancellation.pointer, 7);
        assert_eq!(cancellation.target, CONTENT);
        assert_eq!(cancellation.committed_generation, 1);
        assert_eq!(core.take_cancellation(), None);
    }
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
    struct HintCheckingMeasurer {
        inner: DeterministicMeasurer,
        saw_text: Cell<bool>,
        saw_width_constraint: Cell<bool>,
    }

    impl IntrinsicMeasurer for HintCheckingMeasurer {
        fn measure_text(&self, request: TextMeasureRequest<'_>) -> LogicalSize {
            assert_eq!(request.properties.font_family.as_deref(), Some("Esox Sans"));
            assert_eq!(request.properties.font_size, 16.0);
            assert_eq!(request.properties.font_weight, 500);
            assert_eq!(request.properties.locale.as_deref(), Some("en-US"));
            assert_eq!(request.properties.direction, TextDirection::LeftToRight);
            self.saw_text.set(true);
            if request.known_dimensions.width.is_some()
                || matches!(request.available_space.width, AvailableLength::Definite(_))
            {
                self.saw_width_constraint.set(true);
            }
            self.inner.measure_text(request)
        }

        fn measure_image(&self, request: ImageMeasureRequest) -> LogicalSize {
            self.inner.measure_image(request)
        }
    }

    let measurer = HintCheckingMeasurer {
        inner: DeterministicMeasurer::new(8.0, 18.0).with_image(7, LogicalSize::new(40.0, 24.0)),
        saw_text: Cell::new(false),
        saw_width_constraint: Cell::new(false),
    };
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(320.0, 120.0));

    core.run_frame(&measurer, &mut consumer, |_| representative_scene())
        .unwrap();

    assert_eq!(consumer.scenes().len(), 1);
    assert_eq!(consumer.scenes()[0].generation, 1);
    assert_eq!(consumer.scenes()[0].viewport.width, 320.0);
    assert!(measurer.saw_text.get());
    assert!(measurer.saw_width_constraint.get());
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

#[test]
fn production_layout_properties_resolve_in_one_pass() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 80.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Row, 5.0)
            .without_paint()
            .with_padding(10.0)
            .with_children(vec![
                Element::fixed(SIDEBAR, 10.0, 10.0)
                    .with_size(Some(20.0), Some(15.0))
                    .with_min_size(Some(30.0), Some(20.0)),
                Element::fixed(CONTENT, 100.0, 100.0).with_max_size(Some(40.0), Some(25.0)),
                Element::fixed(INSERTED, 0.0, 10.0).with_flex_grow(1.0),
            ])
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    assert_eq!(
        scene.node(ROOT).unwrap().bounds,
        LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 80.0,
        }
    );
    assert_eq!(
        scene.node(SIDEBAR).unwrap().bounds,
        LogicalRect {
            x: 10.0,
            y: 10.0,
            width: 30.0,
            height: 20.0,
        }
    );
    assert_eq!(
        scene.node(CONTENT).unwrap().bounds,
        LogicalRect {
            x: 45.0,
            y: 10.0,
            width: 40.0,
            height: 25.0,
        }
    );
    assert_eq!(
        scene.node(INSERTED).unwrap().bounds,
        LogicalRect {
            x: 90.0,
            y: 10.0,
            width: 100.0,
            height: 10.0,
        }
    );
}

#[test]
fn production_paint_primitives_are_backend_neutral_and_styled() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(120.0, 60.0));
    let fill = Color::rgba(0.1, 0.2, 0.3, 1.0);
    let stroke = Color::rgba(0.7, 0.6, 0.5, 1.0);
    let text_color = Color::rgba(0.9, 0.8, 0.1, 1.0);
    let properties = representative_text_properties();

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![
                Element::fixed(SIDEBAR, 120.0, 20.0)
                    .with_paint(PaintPrimitive::SolidRect { color: fill }),
                Element::fixed(CONTENT, 120.0, 20.0).with_paint(PaintPrimitive::Border {
                    color: stroke,
                    width: 2.0,
                }),
                Element::text_with_properties(TEXT, "styled", properties.clone()).with_paint(
                    PaintPrimitive::Text {
                        content: "styled".into(),
                        properties: properties.clone(),
                        color: text_color,
                    },
                ),
            ])
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    assert_eq!(
        scene.display_list[0].primitive,
        PaintPrimitive::SolidRect { color: fill }
    );
    assert_eq!(
        scene.display_list[1].primitive,
        PaintPrimitive::Border {
            color: stroke,
            width: 2.0,
        }
    );
    assert_eq!(
        scene.display_list[2].primitive,
        PaintPrimitive::Text {
            content: "styled".into(),
            properties,
            color: text_color,
        }
    );
    for record in &scene.display_list {
        let node = scene.node(record.id).unwrap();
        assert_eq!(record.bounds, node.bounds);
        assert_eq!(node.paint_bounds, Some(node.bounds));
        assert_eq!(node.current_damage_bounds, Some(node.bounds));
    }
}

#[test]
fn grid_tracks_gaps_and_implicit_rows_resolve_on_first_frame_and_resize() {
    const A: WidgetId = WidgetId(20);
    const B: WidgetId = WidgetId(21);
    const C: WidgetId = WidgetId(22);
    const D: WidgetId = WidgetId(23);
    const E: WidgetId = WidgetId(24);

    fn grid_scene() -> Element {
        Element::grid_with_gaps(
            ROOT,
            vec![
                GridTrack::Fixed(40.0),
                GridTrack::Auto,
                GridTrack::Fraction(1.0),
                GridTrack::Fraction(2.0),
            ],
            6.0,
            8.0,
        )
        .without_paint()
        .with_padding(10.0)
        .with_children(vec![
            Element::flex(A, Axis::Column, 0.0).with_size(None, Some(20.0)),
            Element::fixed(B, 30.0, 20.0),
            Element::flex(C, Axis::Column, 0.0).with_size(None, Some(20.0)),
            Element::flex(D, Axis::Column, 0.0).with_size(None, Some(20.0)),
            Element::flex(E, Axis::Column, 0.0).with_size(None, Some(20.0)),
        ])
    }

    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(300.0, 140.0));
    core.run_frame(&measurer, &mut consumer, |_| grid_scene())
        .unwrap();

    let first = core.committed_scene().unwrap();
    assert_eq!(first.node(A).unwrap().bounds.x, 10.0);
    assert_eq!(first.node(B).unwrap().bounds.x, 56.0);
    assert_eq!(first.node(C).unwrap().bounds.x, 92.0);
    assert_eq!(first.node(C).unwrap().bounds.width, 64.0);
    assert_eq!(first.node(D).unwrap().bounds.x, 162.0);
    assert_eq!(first.node(D).unwrap().bounds.width, 128.0);
    assert_eq!(first.node(E).unwrap().bounds.y, 74.0);

    core.resize(LogicalSize::new(360.0, 140.0));
    core.run_frame(&measurer, &mut consumer, |_| grid_scene())
        .unwrap();
    let resized = core.committed_scene().unwrap();
    assert_eq!(resized.generation, 2);
    assert_eq!(resized.node(C).unwrap().bounds.width, 84.0);
    assert_eq!(resized.node(D).unwrap().bounds.x, 182.0);
    assert_eq!(resized.node(D).unwrap().bounds.width, 168.0);
    assert_eq!(resized.node(E).unwrap().bounds.y, 74.0);
}

#[test]
fn flex_main_and_cross_axis_alignment_compose_with_max_width() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 80.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Row, 10.0)
            .without_paint()
            .with_padding(10.0)
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_children(vec![
                Element::fixed(SIDEBAR, 20.0, 10.0),
                Element::flex(CONTENT, Axis::Column, 0.0)
                    .with_size(Some(100.0), Some(20.0))
                    .with_max_size(Some(40.0), None)
                    .with_children(vec![Element::fixed(TEXT, 10.0, 5.0)]),
            ])
    })
    .unwrap();

    let scene = core.committed_scene().unwrap();
    assert_eq!(scene.node(SIDEBAR).unwrap().bounds.x, 65.0);
    assert_eq!(scene.node(SIDEBAR).unwrap().bounds.y, 35.0);
    assert_eq!(scene.node(CONTENT).unwrap().bounds.x, 95.0);
    assert_eq!(scene.node(CONTENT).unwrap().bounds.y, 30.0);
    assert_eq!(scene.node(CONTENT).unwrap().bounds.width, 40.0);
}

#[test]
fn invalid_grid_declaration_returns_typed_error_without_committing() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| {
        Element::fixed(ROOT, 100.0, 40.0)
    })
    .unwrap();
    let committed = core.committed_scene().unwrap().clone();

    let error = core
        .run_frame(&measurer, &mut consumer, |_| {
            Element::grid_with_gaps(ROOT, vec![GridTrack::Fraction(0.0)], 4.0, 2.0)
        })
        .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidGrid {
            id: ROOT,
            error: GridDeclarationError::InvalidFractionTrack {
                index: 0,
                value: 0.0,
            },
        }
    );
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(consumer.scenes().len(), 1);
}

#[test]
fn retained_scroll_state_accepts_repeated_two_axis_wheel_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::default()),
            LogicalSize::new(160.0, 100.0),
        )
    })
    .unwrap();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.5, 0.25));
    let second = core
        .run_frame(&measurer, &mut consumer, |_| {
            retained_scroll_scene(None, LogicalSize::new(160.0, 100.0))
        })
        .unwrap()
        .clone();
    assert_eq!(
        second
            .node(SCROLL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset,
        LogicalPoint::new(20.0, 10.0)
    );
    assert_eq!(
        second.node(SCROLL_CONTENT).unwrap().bounds,
        rect(-20.0, -10.0, 160.0, 100.0)
    );

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.5, 0.25));
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(160.0, 100.0))
    })
    .unwrap();
    assert_eq!(
        core.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::new(40.0, 20.0)
    );
    assert_eq!(consumer.scenes().len(), 3);
}

#[test]
fn normalized_wheel_events_are_isolated_between_frame_core_owners() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut left_consumer = NullSceneConsumer::default();
    let mut right_consumer = NullSceneConsumer::default();
    let mut left = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut right = FrameCore::new(LogicalSize::new(100.0, 40.0));

    for (core, consumer) in [
        (&mut left, &mut left_consumer),
        (&mut right, &mut right_consumer),
    ] {
        core.run_frame(&measurer, consumer, |_| {
            retained_scroll_scene(
                Some(LogicalPoint::default()),
                LogicalSize::new(160.0, 100.0),
            )
        })
        .unwrap();
    }

    left.queue_wheel_event(esox_input::WheelEvent {
        position: esox_input::WheelPosition::new(10.0, 10.0),
        delta: esox_input::WheelDelta::new(0.5, 0.25),
    });
    left.run_frame(&measurer, &mut left_consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(160.0, 100.0))
    })
    .unwrap();
    right
        .run_frame(&measurer, &mut right_consumer, |_| {
            retained_scroll_scene(None, LogicalSize::new(160.0, 100.0))
        })
        .unwrap();

    assert_eq!(
        left.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::new(20.0, 10.0)
    );
    assert_eq!(
        right.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::default()
    );
}

#[test]
fn wheel_before_first_commit_has_no_target_and_does_not_leak() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.queue_wheel_event(esox_input::WheelEvent {
        position: esox_input::WheelPosition::new(10.0, 10.0),
        delta: esox_input::WheelDelta::new(0.0, 1.0),
    });
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::default()),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();
    assert_eq!(
        core.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::default()
    );

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(100.0, 100.0))
    })
    .unwrap();
    assert_eq!(
        core.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::default()
    );
}

#[test]
fn multiple_queued_wheels_are_applied_in_event_order() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::new(0.0, 20.0)),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, -1.0));
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 1.0));
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(100.0, 100.0))
    })
    .unwrap();

    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 40.0);
    assert_eq!(consumer.scenes().len(), 2);
}

#[test]
fn nested_wheel_routes_each_axis_to_the_deepest_viewport_that_changes() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    core.run_frame(&measurer, &mut consumer, |_| retained_nested_scroll_scene())
        .unwrap();
    for expected_inner in [20.0, 40.0] {
        core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
        core.run_frame(&measurer, &mut consumer, |_| retained_nested_scroll_scene())
            .unwrap();
        assert_eq!(
            core.scroll_offset(SCROLL_CONTENT).unwrap().applied.y,
            expected_inner
        );
        assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 0.0);
    }

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| retained_nested_scroll_scene())
        .unwrap();
    assert_eq!(core.scroll_offset(SCROLL_CONTENT).unwrap().applied.y, 40.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.5, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| retained_nested_scroll_scene())
        .unwrap();
    assert_eq!(core.scroll_offset(SCROLL_CONTENT).unwrap().applied.x, 0.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.x, 0.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 40.0);
}

#[test]
fn explicit_scroll_request_overrides_retained_input_then_retained_mode_resumes_it() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::default()),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 1.0));
    let controlled = core
        .run_frame(&measurer, &mut consumer, |_| {
            retained_scroll_scene(
                Some(LogicalPoint::new(0.0, 5.0)),
                LogicalSize::new(100.0, 100.0),
            )
        })
        .unwrap()
        .clone();
    let metrics = controlled.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.requested_offset, LogicalPoint::new(0.0, 5.0));
    assert_eq!(metrics.applied_offset, LogicalPoint::new(0.0, 5.0));

    let retained = core
        .run_frame(&measurer, &mut consumer, |_| {
            retained_scroll_scene(None, LogicalSize::new(100.0, 100.0))
        })
        .unwrap();
    assert_eq!(
        retained
            .node(SCROLL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .requested_offset,
        LogicalPoint::new(0.0, 5.0)
    );
}

#[test]
fn shrink_resize_removal_and_reintroduction_reconcile_stable_scroll_state() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::new(0.0, 50.0)),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();
    let shrunk = core
        .run_frame(&measurer, &mut consumer, |_| {
            retained_scroll_scene(None, LogicalSize::new(100.0, 45.0))
        })
        .unwrap()
        .clone();
    let metrics = shrunk.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.requested_offset.y, 50.0);
    assert_eq!(metrics.applied_offset.y, 5.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().requested.y, 50.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 5.0);

    core.resize(LogicalSize::new(100.0, 100.0));
    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(100.0), Some(100.0))
                .clip_children()
                .scrollable()
                .with_children(vec![Element::fixed(SCROLL_CONTENT, 100.0, 45.0)])])
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 0.0);

    core.run_frame(&measurer, &mut consumer, |_| {
        Element::fixed(ROOT, 100.0, 40.0)
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL), None);
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(100.0, 100.0))
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 0.0);
}

#[test]
fn invalid_wheel_and_failed_frame_leave_persistent_scroll_state_atomic() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::new(0.0, 20.0)),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();
    for delta in [
        LogicalPoint::new(f32::NAN, 1.0),
        LogicalPoint::new(0.0, f32::INFINITY),
        LogicalPoint::new(f32::NEG_INFINITY, 0.0),
    ] {
        core.queue_wheel(LogicalPoint::new(10.0, 10.0), delta);
    }
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(100.0, 100.0))
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    let committed = core.committed_scene().unwrap().clone();
    let state = core.scroll_offset(SCROLL).unwrap();
    let error = core
        .run_frame(&measurer, &mut consumer, |_| {
            Element::flex(ROOT, Axis::Column, 0.0).with_children(vec![
                Element::fixed(ROW_A, 10.0, 10.0),
                Element::fixed(ROW_A, 20.0, 20.0),
            ])
        })
        .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(ROW_A));
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(core.scroll_offset(SCROLL), Some(state));
    assert_eq!(consumer.scenes().len(), 2);

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(None, LogicalSize::new(100.0, 100.0))
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 40.0);
}

#[test]
fn blocking_non_scrollable_overlay_prevents_obscured_viewport_wheel_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let scene = || {
        Element::fixed(ROOT, 100.0, 40.0)
            .without_paint()
            .with_children(vec![
                Element::flex(SCROLL, Axis::Column, 0.0)
                    .without_paint()
                    .with_size(Some(100.0), Some(40.0))
                    .clip_children()
                    .scrollable()
                    .with_children(vec![Element::fixed(SCROLL_CONTENT, 100.0, 100.0)]),
                Element::fixed(OVERLAY, 100.0, 40.0)
                    .with_absolute_position(0.0, 0.0)
                    .blocking_overlay(),
            ])
    };

    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();
    assert_eq!(
        core.committed_scene()
            .unwrap()
            .hit_test(LogicalPoint::new(50.0, 20.0))
            .map(|node| node.id),
        Some(OVERLAY)
    );
    core.queue_wheel(LogicalPoint::new(50.0, 20.0), LogicalPoint::new(0.0, 1.0));
    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();

    assert_eq!(
        core.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::default()
    );
}

#[test]
fn empty_viewport_space_still_routes_wheel_to_the_viewport() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let scene = || {
        Element::fixed(ROOT, 100.0, 40.0)
            .without_paint()
            .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(100.0), Some(40.0))
                .clip_children()
                .scrollable()
                .with_children(vec![Element::fixed(SCROLL_CONTENT, 40.0, 100.0)])])
    };

    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();
    core.queue_wheel(LogicalPoint::new(80.0, 20.0), LogicalPoint::new(0.0, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();

    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);
}

#[test]
fn diagonal_wheel_routes_x_to_inner_viewport_and_y_to_ancestor() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    let scene = || {
        let inner = Element::flex(SCROLL_CONTENT, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(30.0))
            .clip_children()
            .scrollable()
            .with_children(vec![Element::fixed(ROW_A, 140.0, 30.0)]);
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(100.0), Some(60.0))
                .clip_children()
                .scrollable()
                .with_children(vec![inner, Element::fixed(ROW_B, 100.0, 80.0)])])
    };

    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.5, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();

    assert_eq!(
        core.scroll_offset(SCROLL_CONTENT).unwrap().applied,
        LogicalPoint::new(20.0, 0.0)
    );
    assert_eq!(
        core.scroll_offset(SCROLL).unwrap().applied,
        LogicalPoint::new(0.0, 20.0)
    );
}

#[test]
fn partial_edge_overshoot_is_consumed_without_residual_propagation() {
    fn scene(explicit: bool) -> Element {
        let inner = Element::flex(SCROLL_CONTENT, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(30.0))
            .clip_children()
            .with_children(vec![Element::fixed(ROW_A, 100.0, 70.0)]);
        let inner = if explicit {
            inner.with_scroll_offset(0.0, 30.0)
        } else {
            inner.scrollable()
        };
        let outer = Element::flex(SCROLL, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(60.0))
            .clip_children()
            .with_children(vec![inner, Element::fixed(ROW_B, 100.0, 80.0)]);
        let outer = if explicit {
            outer.with_scroll_offset(0.0, 0.0)
        } else {
            outer.scrollable()
        };
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![outer])
    }

    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));
    core.run_frame(&measurer, &mut consumer, |_| scene(true))
        .unwrap();

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| scene(false))
        .unwrap();
    assert_eq!(core.scroll_offset(SCROLL_CONTENT).unwrap().applied.y, 40.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 0.0);

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| scene(false))
        .unwrap();
    assert_eq!(core.scroll_offset(SCROLL_CONTENT).unwrap().applied.y, 40.0);
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);
}

#[test]
fn hidden_scroll_viewport_retains_offset_round_trip_but_rejects_wheel() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::new(0.0, 20.0)),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_participation_scene(true, false)
    })
    .unwrap();
    assert!(
        core.committed_scene()
            .unwrap()
            .node(SCROLL)
            .unwrap()
            .effective_hidden
    );
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_participation_scene(true, false)
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_participation_scene(false, false)
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);
}

#[test]
fn disabled_scroll_viewport_retains_offset_but_rejects_wheel() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_scene(
            Some(LogicalPoint::new(0.0, 20.0)),
            LogicalSize::new(100.0, 100.0),
        )
    })
    .unwrap();

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_participation_scene(false, true)
    })
    .unwrap();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_participation_scene(false, true)
    })
    .unwrap();
    assert!(
        core.committed_scene()
            .unwrap()
            .node(SCROLL)
            .unwrap()
            .effective_disabled
    );
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);

    core.run_frame(&measurer, &mut consumer, |_| {
        retained_scroll_participation_scene(false, false)
    })
    .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);
}

#[test]
fn logical_pointer_and_wheel_events_share_committed_scene_position() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let scene = || {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(100.0), Some(40.0))
                .clip_children()
                .scrollable()
                .with_children(vec![
                    Element::fixed(SCROLL_CONTENT, 100.0, 100.0).interactive()
                ])])
    };
    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();

    let position = esox_input::LogicalPosition::new(20.0, 10.0);
    assert!(core.queue_pointer_input(
        3,
        esox_input::PointerEvent {
            phase: esox_input::PointerPhase::Press { button: 0 },
            position,
        },
    ));
    assert!(core.queue_wheel_event(esox_input::WheelEvent {
        position,
        delta: esox_input::WheelDelta::new(0.0, 0.5),
    }));
    core.run_frame(&measurer, &mut consumer, |state| {
        let response = state.take_response(SCROLL_CONTENT).unwrap();
        assert_eq!(response.position, LogicalPoint::new(20.0, 10.0));
        scene()
    })
    .unwrap();

    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);
}

#[test]
fn invalid_logical_input_and_viewports_are_rejected_without_mutation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| representative_scene())
        .unwrap();

    assert!(!core.resize(LogicalSize::new(f32::NAN, 40.0)));
    assert!(!core.resize(LogicalSize::new(100.0, 0.0)));
    assert!(!core.queue_pointer_event(
        PointerEventKind::Press,
        0,
        LogicalPoint::new(f32::INFINITY, 10.0),
    ));
    assert!(!core.queue_wheel(
        LogicalPoint::new(10.0, 10.0),
        LogicalPoint::new(f32::NAN, 1.0),
    ));
    assert!(FrameCore::try_new(LogicalSize::new(-1.0, 40.0)).is_none());

    core.run_frame(&measurer, &mut consumer, |state| {
        assert!(state.take_response(ROW_A).is_none());
        representative_scene()
    })
    .unwrap();
    assert_eq!(
        core.committed_scene().unwrap().viewport,
        LogicalSize::new(100.0, 40.0)
    );
}
