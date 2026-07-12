use esox_ui::declaration::{
    run_declaration_frame, ButtonStyle, DeclarationStyle, LogicalTransform, TextStyle,
};
use esox_ui::frame_core::{
    Axis, Color, DeterministicMeasurer, Element, FrameCore, FrameError, LogicalPoint, LogicalRect,
    LogicalSize, NullSceneConsumer, PointerEventKind, WidgetId,
};
use esox_ui::frame_scene_consumer::{submit_display_list_scaled, RendererScale};
use esox_ui::scene_submission::{TextPaintBoundary, TextPaintRequest};

use esox_gfx::Frame;

const ROOT: WidgetId = WidgetId(1);
const PARENT: WidgetId = WidgetId(2);
const CHILD: WidgetId = WidgetId(3);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

struct NoText;

impl TextPaintBoundary<Frame> for NoText {
    type Error = std::convert::Infallible;

    fn paint_text(
        &mut self,
        _target: &mut Frame,
        _request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error> {
        unreachable!("the test scene contains no text")
    }
}

#[test]
fn translated_declaration_resolves_every_product_on_its_first_frame() {
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let scene = run_declaration_frame(
        &mut core,
        &DeterministicMeasurer::new(8.0, 18.0),
        &mut consumer,
        |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                ui.button(
                    CHILD,
                    "Move",
                    ButtonStyle::default()
                        .layout(
                            DeclarationStyle::new()
                                .size(20.0, 10.0)
                                .transform(LogicalTransform::translate(30.0, 40.0)),
                        )
                        .text(TextStyle::default()),
                );
            });
        },
    )
    .unwrap();

    let layout = rect(0.0, 0.0, 20.0, 10.0);
    let visual = rect(30.0, 40.0, 20.0, 10.0);
    let node = scene.node(CHILD).unwrap();
    assert_eq!(node.bounds, layout);
    assert_eq!(node.transformed_bounds, visual);
    assert_eq!(node.paint_bounds, Some(visual));
    assert_eq!(node.hit_bounds, Some(visual));
    assert_eq!(node.semantic_bounds, Some(visual));
    assert_eq!(node.current_damage_bounds, Some(visual));
    assert_eq!(scene.display_list[0].bounds, visual);
    assert_eq!(
        scene.hit_test(LogicalPoint::new(35.0, 45.0)).unwrap().id,
        CHILD
    );
    assert!(scene.hit_test(LogicalPoint::new(5.0, 5.0)).is_none());
    assert_eq!(scene.semantics.node(CHILD).unwrap().bounds, visual);
    assert!(scene
        .damage
        .iter()
        .any(|damage| damage.id == CHILD && damage.current_bounds == visual));
}

#[test]
fn scaling_uses_current_layout_center_and_nested_transforms_compose() {
    let mut core = FrameCore::new(LogicalSize::new(300.0, 200.0));
    let mut consumer = NullSceneConsumer::default();
    let scene = core
        .run_frame(
            &DeterministicMeasurer::new(8.0, 18.0),
            &mut consumer,
            |_| {
                Element::flex(ROOT, Axis::Column, 0.0)
                    .without_paint()
                    .with_children(vec![Element::flex(PARENT, Axis::Column, 0.0)
                        .without_paint()
                        .with_size(Some(100.0), Some(100.0))
                        .with_transform(LogicalTransform::new(10.0, 20.0, 2.0, 2.0))
                        .with_children(vec![Element::fixed(CHILD, 20.0, 10.0)
                            .interactive()
                            .with_transform(LogicalTransform::new(5.0, 0.0, 0.5, 2.0))])])
            },
        )
        .unwrap();

    assert_eq!(
        scene.node(PARENT).unwrap().bounds,
        rect(0.0, 0.0, 100.0, 100.0)
    );
    assert_eq!(
        scene.node(PARENT).unwrap().transformed_bounds,
        rect(-40.0, -30.0, 200.0, 200.0)
    );
    let child = scene.node(CHILD).unwrap();
    assert_eq!(child.bounds, rect(0.0, 0.0, 20.0, 10.0));
    assert_eq!(child.transformed_bounds, rect(-20.0, -40.0, 20.0, 40.0));
    assert_eq!(child.paint_bounds, Some(child.transformed_bounds));
    assert_eq!(child.hit_bounds, Some(child.transformed_bounds));
    assert_eq!(child.semantic_bounds, Some(child.transformed_bounds));
}

#[test]
fn transformed_descendants_use_transformed_intersected_clips() {
    let mut core = FrameCore::new(LogicalSize::new(300.0, 200.0));
    let mut consumer = NullSceneConsumer::default();
    let scene = core
        .run_frame(
            &DeterministicMeasurer::new(8.0, 18.0),
            &mut consumer,
            |_| {
                Element::flex(ROOT, Axis::Column, 0.0)
                    .without_paint()
                    .with_children(vec![Element::flex(PARENT, Axis::Column, 0.0)
                        .without_paint()
                        .with_size(Some(100.0), Some(100.0))
                        .clip_children()
                        .with_transform(LogicalTransform::new(50.0, 0.0, 2.0, 1.0))
                        .with_children(vec![Element::fixed(CHILD, 50.0, 50.0)
                            .interactive()
                            .with_transform(LogicalTransform::translate(100.0, 0.0))])])
            },
        )
        .unwrap();

    let clip = rect(0.0, 0.0, 200.0, 100.0);
    let child = scene.node(CHILD).unwrap();
    assert_eq!(child.transformed_bounds, rect(200.0, 0.0, 100.0, 50.0));
    assert_eq!(child.effective_clip, Some(clip));
    assert_eq!(
        scene.display_list.last().unwrap().effective_clip,
        Some(clip)
    );
    assert!(scene.hit_test(LogicalPoint::new(225.0, 25.0)).is_none());
}

#[test]
fn transform_changes_and_removal_damage_previous_and_current_visual_extents() {
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let declare = |translation| {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::fixed(CHILD, 20.0, 10.0)
                .with_transform(LogicalTransform::translate(translation, 0.0))])
    };

    core.run_frame(&measurer, &mut consumer, |_| declare(10.0))
        .unwrap();
    let moved = core
        .run_frame(&measurer, &mut consumer, |_| declare(60.0))
        .unwrap()
        .clone();
    let child_damage: Vec<_> = moved
        .damage
        .iter()
        .filter(|damage| damage.id == CHILD)
        .map(|damage| damage.current_bounds)
        .collect();
    assert!(child_damage.contains(&rect(10.0, 0.0, 20.0, 10.0)));
    assert!(child_damage.contains(&rect(60.0, 0.0, 20.0, 10.0)));

    let removed = core
        .run_frame(&measurer, &mut consumer, |_| {
            Element::flex(ROOT, Axis::Column, 0.0).without_paint()
        })
        .unwrap();
    assert!(removed
        .damage
        .iter()
        .any(|damage| damage.id == CHILD && damage.current_bounds == rect(60.0, 0.0, 20.0, 10.0)));
}

#[test]
fn transformed_pointer_and_wheel_routing_keep_logical_event_values() {
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    core.set_wheel_scroll_speed(10.0);
    let mut consumer = NullSceneConsumer::default();
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let declaration = || {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::flex(CHILD, Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(40.0), Some(20.0))
                .clip_children()
                .scrollable()
                .interactive()
                .with_transform(LogicalTransform::translate(50.0, 30.0))
                .with_children(vec![Element::fixed(PARENT, 80.0, 80.0)])])
    };
    core.run_frame(&measurer, &mut consumer, |_| declaration())
        .unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 7, LogicalPoint::new(55.0, 35.0)));
    assert!(core.queue_wheel(LogicalPoint::new(55.0, 35.0), LogicalPoint::new(1.0, 1.0)));
    let mut response = None;
    core.run_frame(&measurer, &mut consumer, |state| {
        response = state.take_response(CHILD);
        declaration()
    })
    .unwrap();

    let response = response.unwrap();
    assert_eq!(response.position, LogicalPoint::new(55.0, 35.0));
    assert_eq!(response.target_bounds, rect(50.0, 30.0, 40.0, 20.0));
    assert_eq!(
        core.scroll_offset(CHILD).unwrap().applied,
        LogicalPoint::new(10.0, 10.0)
    );
}

#[test]
fn invalid_and_overflowing_transforms_are_atomic_and_retry_input_in_order() {
    let mut core = FrameCore::new(LogicalSize::new(100.0, 50.0));
    core.set_wheel_scroll_speed(10.0);
    let mut consumer = NullSceneConsumer::default();
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let valid = || {
        Element::flex(ROOT, Axis::Column, 0.0)
            .interactive()
            .clip_children()
            .scrollable()
            .with_children(vec![Element::fixed(CHILD, 100.0, 100.0)])
    };
    core.run_frame(&measurer, &mut consumer, |_| valid())
        .unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 1, LogicalPoint::new(5.0, 5.0)));
    assert!(core.queue_pointer_event(PointerEventKind::Release, 1, LogicalPoint::new(5.0, 5.0)));
    assert!(core.queue_wheel(LogicalPoint::new(5.0, 5.0), LogicalPoint::new(0.0, 1.0)));

    for transform in [
        LogicalTransform::translate(f32::NAN, 0.0),
        LogicalTransform::scale(0.0, 1.0),
        LogicalTransform::new(f32::MAX, 0.0, f32::MAX, 1.0),
    ] {
        let before = core.committed_scene().unwrap().clone();
        assert_eq!(
            core.run_frame(&measurer, &mut consumer, |_| valid()
                .with_transform(transform)),
            Err(FrameError::InvalidTransform(ROOT))
        );
        assert_eq!(core.committed_scene(), Some(&before));
        assert_eq!(core.scroll_offset(ROOT).unwrap().applied.y, 0.0);
        assert_eq!(consumer.scenes().len(), 1);
    }

    let mut responses = Vec::new();
    core.run_frame(&measurer, &mut consumer, |state| {
        while let Some(response) = state.take_response(ROOT) {
            responses.push(response.kind);
        }
        valid()
    })
    .unwrap();
    assert_eq!(
        responses,
        vec![PointerEventKind::Press, PointerEventKind::Release]
    );
    assert_eq!(consumer.scenes().len(), 2);
    assert_eq!(core.scroll_offset(ROOT).unwrap().applied.y, 10.0);
}

#[test]
fn hiding_transformed_content_damages_its_previous_visual_extent() {
    let mut core = FrameCore::new(LogicalSize::new(100.0, 50.0));
    let mut consumer = NullSceneConsumer::default();
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let element = |hidden| {
        Element::flex(ROOT, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![Element::fixed(CHILD, 20.0, 10.0)
                .interactive()
                .with_transform(LogicalTransform::translate(40.0, 20.0))
                .with_hidden(hidden)])
    };
    core.run_frame(&measurer, &mut consumer, |state| {
        state.request_pointer_capture(9, CHILD);
        element(false)
    })
    .unwrap();
    let hidden = core
        .run_frame(&measurer, &mut consumer, |_| element(true))
        .unwrap();
    assert!(hidden.display_list.is_empty());
    assert!(hidden.hit_index.is_empty());
    assert!(hidden.semantics.nodes.is_empty());
    assert!(hidden.damage.iter().any(|damage| {
        damage.id == CHILD && damage.current_bounds == rect(40.0, 20.0, 20.0, 10.0)
    }));
    let cancellation = core.take_cancellation().unwrap();
    assert_eq!(cancellation.kind, PointerEventKind::Cancel);
    assert_eq!(cancellation.target_bounds, rect(40.0, 20.0, 20.0, 10.0));
}

#[test]
fn declaration_transform_does_not_change_layout_of_following_siblings() {
    let mut core = FrameCore::new(LogicalSize::new(100.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let scene = run_declaration_frame(
        &mut core,
        &DeterministicMeasurer::new(8.0, 18.0),
        &mut consumer,
        |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                ui.solid_rect(
                    PARENT,
                    DeclarationStyle::new()
                        .size(20.0, 10.0)
                        .transform(LogicalTransform::scale(2.0, 3.0)),
                    Color::BLACK,
                );
                ui.solid_rect(
                    CHILD,
                    DeclarationStyle::new().size(20.0, 10.0),
                    Color::BLACK,
                );
            });
        },
    )
    .unwrap();
    assert_eq!(
        scene.node(PARENT).unwrap().bounds,
        rect(0.0, 0.0, 20.0, 10.0)
    );
    assert_eq!(
        scene.node(PARENT).unwrap().transformed_bounds,
        rect(-10.0, -10.0, 40.0, 30.0)
    );
    assert_eq!(
        scene.node(CHILD).unwrap().bounds,
        rect(0.0, 10.0, 20.0, 10.0)
    );
}

#[test]
fn renderer_scale_does_not_reapply_the_committed_element_transform() {
    let mut core = FrameCore::new(LogicalSize::new(100.0, 50.0));
    let mut consumer = NullSceneConsumer::default();
    let scene = core
        .run_frame(
            &DeterministicMeasurer::new(8.0, 18.0),
            &mut consumer,
            |_| {
                Element::flex(ROOT, Axis::Column, 0.0)
                    .without_paint()
                    .with_children(vec![Element::fixed(CHILD, 20.0, 10.0)
                        .with_paint(esox_ui::frame_core::PaintPrimitive::SolidRect {
                            color: Color::BLACK,
                        })
                        .with_transform(LogicalTransform::translate(30.0, 20.0))])
            },
        )
        .unwrap()
        .clone();
    let committed = scene.clone();
    let mut frame = Frame::new();

    submit_display_list_scaled(
        &scene.display_list,
        &mut frame,
        &mut NoText,
        RendererScale::new(2.0).unwrap(),
    )
    .unwrap();

    assert_eq!(frame.instance_data()[0].rect, [60.0, 40.0, 40.0, 20.0]);
    assert_eq!(scene, committed);
    assert_eq!(
        scene.node(CHILD).unwrap().bounds,
        rect(0.0, 0.0, 20.0, 10.0)
    );
    assert_eq!(
        scene.node(CHILD).unwrap().transformed_bounds,
        rect(30.0, 20.0, 20.0, 10.0)
    );
}
