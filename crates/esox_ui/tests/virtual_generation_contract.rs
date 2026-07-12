use esox_ui::declaration::virtual_item_id;
use esox_ui::frame_core::{
    Axis, DeterministicMeasurer, Element, FrameCore, FrameError, LogicalPoint, LogicalSize,
    NullSceneConsumer, PointerEventKind, VirtualDeclarationError, VirtualListSpec, VirtualWindow,
    WidgetId,
};

const ROOT: WidgetId = WidgetId(90_000);
const VIRTUAL: WidgetId = WidgetId(90_001);
const STATE: WidgetId = WidgetId(90_002);
const DUPLICATE: WidgetId = WidgetId(90_003);

fn scene(window: &VirtualWindow) -> Element {
    let wrappers = window
        .visible_range
        .clone()
        .map(|index| {
            Element::flex(virtual_item_id(VIRTUAL, index), Axis::Column, 0.0)
                .without_paint()
                .with_size(Some(100.0), Some(20.0))
                .with_absolute_position(0.0, index as f32 * 20.0)
                .with_children(vec![Element::fixed(
                    WidgetId(91_000 + index as u64),
                    100.0,
                    20.0,
                )
                .interactive()])
        })
        .collect();
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![Element::flex(VIRTUAL, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(40.0))
            .clip_children()
            .with_scroll_offset(0.0, window.requested_offset)
            .with_virtual_content_height(window.content_height)
            .with_children(wrappers)])
}

fn invalid_scene() -> Element {
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![
            Element::fixed(DUPLICATE, 10.0, 10.0),
            Element::fixed(DUPLICATE, 10.0, 10.0),
        ])
}

fn commit(
    core: &mut FrameCore,
    measurer: &DeterministicMeasurer,
    consumer: &mut NullSceneConsumer,
    spec: VirtualListSpec,
) -> VirtualWindow {
    let mut attempt = core.begin_generation();
    let window = attempt.plan_virtual_list(spec).unwrap();
    core.finish_generation(attempt, scene(&window), measurer, consumer)
        .unwrap();
    window
}

#[test]
fn failed_generation_rolls_back_virtual_offset_widget_state_and_wheel_for_retry() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    commit(
        &mut core,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(20.0),
    );
    let committed = core.committed_scene().unwrap().clone();
    assert!(core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5)));

    let mut failed = core.begin_generation();
    let failed_window = failed
        .plan_virtual_list(VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0))
        .unwrap();
    assert_eq!(failed_window.applied_offset, 40.0);
    assert_eq!(failed_window.visible_range, 2..4);
    failed.widget_state().insert(STATE, 99);
    let error = core
        .finish_generation(failed, invalid_scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(DUPLICATE));
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(core.scroll_offset(VIRTUAL).unwrap().applied.y, 20.0);

    let mut retry = core.begin_generation();
    assert_eq!(retry.widget_state().get(STATE), None);
    let retry_window = retry
        .plan_virtual_list(VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0))
        .unwrap();
    assert_eq!(retry_window, failed_window);
    retry.widget_state().insert(STATE, 7);
    core.finish_generation(retry, scene(&retry_window), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(core.scroll_offset(VIRTUAL).unwrap().applied.y, 40.0);
    let mut inspection = core.begin_generation();
    assert_eq!(inspection.widget_state().get(STATE), Some(7));
    inspection.abort();
}

#[test]
fn two_frame_core_owners_keep_virtual_ranges_and_wheels_isolated() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut left = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut right = FrameCore::new(LogicalSize::new(100.0, 40.0));
    commit(
        &mut left,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 10, 20.0, 40.0).with_offset(0.0),
    );
    commit(
        &mut right,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 10, 20.0, 40.0).with_offset(80.0),
    );
    assert!(left.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5)));

    let left_window = commit(
        &mut left,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 10, 20.0, 40.0),
    );
    let right_window = commit(
        &mut right,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 10, 20.0, 40.0),
    );
    assert_eq!(left_window.visible_range, 1..3);
    assert_eq!(right_window.visible_range, 4..6);
    assert_eq!(left.scroll_offset(VIRTUAL).unwrap().applied.y, 20.0);
    assert_eq!(right.scroll_offset(VIRTUAL).unwrap().applied.y, 80.0);
}

#[test]
fn invalid_direct_virtual_extent_is_atomic() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let attempt = core.begin_generation();
    let root = Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![Element::flex(VIRTUAL, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(40.0))
            .with_scroll_offset(0.0, 0.0)
            .with_virtual_content_height(f32::NAN)]);
    let error = core
        .finish_generation(attempt, root, &measurer, &mut consumer)
        .unwrap_err();
    assert!(matches!(
        error,
        FrameError::InvalidVirtualContent {
            id: VIRTUAL,
            error: VirtualDeclarationError::InvalidContentHeight(value),
        } if value.is_nan()
    ));
    assert!(core.committed_scene().is_none());
    assert!(consumer.scenes().is_empty());
}

#[test]
fn large_representable_positions_plan_in_f64_and_collapsed_positions_are_rejected() {
    let core = FrameCore::new(LogicalSize::new(100.0, 2_048.0));
    let mut attempt = core.begin_generation();
    let window = attempt
        .plan_virtual_list(
            VirtualListSpec::new(VIRTUAL, 1_000_000, 1_024.0, 2_048.0).with_offset(1_023_996_928.0),
        )
        .unwrap();
    assert_eq!(window.visible_range, 999_997..999_999);
    attempt.abort();

    let mut attempt = core.begin_generation();
    let error = attempt
        .plan_virtual_list(VirtualListSpec::new(VIRTUAL, 20_000_001, 1.0, 40.0))
        .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidVirtualContent {
            id: VIRTUAL,
            error: VirtualDeclarationError::UnrepresentableItemPositions {
                item_count: 20_000_001,
                item_height: 1.0,
            },
        }
    );
    attempt.abort();
}

#[test]
fn virtualized_capture_is_cancelled_when_owner_remains_but_item_leaves_range() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let mut attempt = core.begin_generation();
    let first = attempt
        .plan_virtual_list(VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(0.0))
        .unwrap();
    let captured = WidgetId(91_000);
    attempt.widget_state().request_pointer_capture(7, captured);
    core.finish_generation(attempt, scene(&first), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(core.pointer_capture(7), Some(captured));

    assert!(core.queue_pointer_event(PointerEventKind::Move, 7, LogicalPoint::new(10.0, 10.0),));
    let mut shifted = core.begin_generation();
    let shifted_window = shifted
        .plan_virtual_list(VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(40.0))
        .unwrap();
    core.finish_generation(shifted, scene(&shifted_window), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(core.pointer_capture(7), None);
    let cancellation = core.take_cancellation().unwrap();
    assert_eq!(cancellation.kind, PointerEventKind::Cancel);
    assert_eq!(cancellation.target, captured);

    let mut returned = core.begin_generation();
    assert!(returned.widget_state().take_response(captured).is_some());
    returned.abort();
}

#[test]
fn unconsumed_virtual_response_expires_after_one_successful_generation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    commit(
        &mut core,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(0.0),
    );
    let target = WidgetId(91_000);
    assert!(core.queue_pointer_event(PointerEventKind::Move, 3, LogicalPoint::new(10.0, 10.0),));
    commit(
        &mut core,
        &measurer,
        &mut consumer,
        VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(40.0),
    );

    let next = core.begin_generation();
    let window = VirtualWindow {
        visible_range: 2..4,
        requested_offset: 40.0,
        applied_offset: 40.0,
        maximum_offset: 60.0,
        content_height: 100.0,
    };
    core.finish_generation(next, scene(&window), &measurer, &mut consumer)
        .unwrap();
    let mut inspection = core.begin_generation();
    assert!(inspection.widget_state().take_response(target).is_none());
    inspection.abort();
}
