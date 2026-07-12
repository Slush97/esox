use esox_ui::frame_core::{
    Axis, DeterministicMeasurer, Element, FrameCore, FrameError, LogicalPoint, LogicalSize,
    NullSceneConsumer, PointerEventKind, WidgetId,
};

const ROOT: WidgetId = WidgetId(1);
const ACTION: WidgetId = WidgetId(2);
const DUPLICATE: WidgetId = WidgetId(3);
const STATE: WidgetId = WidgetId(4);
const SCROLL: WidgetId = WidgetId(5);
const SCROLL_CONTENT: WidgetId = WidgetId(6);

fn scene() -> Element {
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![Element::fixed(ACTION, 100.0, 40.0).interactive()])
}

fn invalid_scene() -> Element {
    Element::flex(ROOT, Axis::Row, 0.0)
        .without_paint()
        .with_children(vec![
            Element::fixed(DUPLICATE, 50.0, 40.0),
            Element::fixed(DUPLICATE, 50.0, 40.0),
        ])
}

fn scroll_scene() -> Element {
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![Element::flex(SCROLL, Axis::Column, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(40.0))
            .clip_children()
            .scrollable()
            .with_children(vec![Element::fixed(SCROLL_CONTENT, 100.0, 100.0)])])
}

#[test]
fn split_generation_success_matches_run_frame() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let viewport = LogicalSize::new(100.0, 40.0);
    let mut wrapped_core = FrameCore::new(viewport);
    let mut wrapped_consumer = NullSceneConsumer::default();
    let wrapped = wrapped_core
        .run_frame(&measurer, &mut wrapped_consumer, |state| {
            state.insert(STATE, 11);
            state.request_keyboard_focus(ACTION);
            scene()
        })
        .unwrap()
        .clone();

    let mut split_core = FrameCore::new(viewport);
    let mut split_consumer = NullSceneConsumer::default();
    let mut attempt = split_core.begin_generation();
    attempt.widget_state().insert(STATE, 11);
    attempt.widget_state().request_keyboard_focus(ACTION);
    let split = split_core
        .finish_generation(attempt, scene(), &measurer, &mut split_consumer)
        .unwrap()
        .clone();

    assert_eq!(split, wrapped);
    assert_eq!(split_core.keyboard_focus(), Some(ACTION));
    assert_eq!(split_consumer.scenes(), wrapped_consumer.scenes());

    let mut inspection = split_core.begin_generation();
    assert_eq!(inspection.widget_state().get(STATE), Some(11));
    inspection.abort();
}

#[test]
fn failed_split_finish_rolls_back_and_retries_dispatched_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |state| {
        state.insert(STATE, 7);
        scene()
    })
    .unwrap();
    let committed = core.committed_scene().unwrap().clone();

    assert!(core.queue_pointer_event(PointerEventKind::Press, 9, LogicalPoint::new(10.0, 10.0),));
    let mut failed = core.begin_generation();
    let response = failed.widget_state().take_response(ACTION).unwrap();
    assert_eq!(response.committed_generation, committed.generation);
    failed.widget_state().insert(STATE, 99);
    failed.widget_state().request_pointer_capture(9, ACTION);
    let error = core
        .finish_generation(failed, invalid_scene(), &measurer, &mut consumer)
        .unwrap_err();

    assert_eq!(error, FrameError::DuplicateWidgetId(DUPLICATE));
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(core.pointer_capture(9), None);
    assert_eq!(consumer.scenes().len(), 1);

    let mut retry = core.begin_generation();
    assert_eq!(retry.widget_state().get(STATE), Some(7));
    let response = retry.widget_state().take_response(ACTION).unwrap();
    assert_eq!(response.pointer, 9);
    retry.widget_state().insert(STATE, 8);
    core.finish_generation(retry, scene(), &measurer, &mut consumer)
        .unwrap();

    assert_eq!(core.committed_scene().unwrap().generation, 2);
    assert_eq!(consumer.scenes().len(), 2);
    let mut inspection = core.begin_generation();
    assert_eq!(inspection.widget_state().get(STATE), Some(8));
    assert!(inspection.widget_state().take_response(ACTION).is_none());
    inspection.abort();
}

#[test]
fn explicit_abort_and_drop_both_roll_back_candidate_state_and_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |state| {
        state.insert(STATE, 7);
        scene()
    })
    .unwrap();
    let committed = core.committed_scene().unwrap().clone();

    core.queue_pointer_press(LogicalPoint::new(10.0, 10.0));
    let mut aborted = core.begin_generation();
    assert!(aborted.widget_state().take_response(ACTION).is_some());
    aborted.widget_state().insert(STATE, 88);
    aborted.abort();

    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(consumer.scenes().len(), 1);

    {
        let mut dropped = core.begin_generation();
        assert_eq!(dropped.widget_state().get(STATE), Some(7));
        assert!(dropped.widget_state().take_response(ACTION).is_some());
        dropped.widget_state().insert(STATE, 99);
    }

    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(consumer.scenes().len(), 1);

    let mut retry = core.begin_generation();
    assert_eq!(retry.widget_state().get(STATE), Some(7));
    assert!(retry.widget_state().take_response(ACTION).is_some());
    retry.widget_state().insert(STATE, 8);
    core.finish_generation(retry, scene(), &measurer, &mut consumer)
        .unwrap();

    assert_eq!(core.committed_scene().unwrap().generation, 2);
    assert_eq!(consumer.scenes().len(), 2);
}

#[test]
fn events_queued_after_begin_survive_the_attempt_commit() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 1, LogicalPoint::new(10.0, 10.0));
    let mut attempt = core.begin_generation();
    assert_eq!(
        attempt
            .widget_state()
            .take_response(ACTION)
            .unwrap()
            .pointer,
        1
    );

    core.queue_pointer_event(PointerEventKind::Move, 2, LogicalPoint::new(20.0, 10.0));
    core.finish_generation(attempt, scene(), &measurer, &mut consumer)
        .unwrap();

    let mut next = core.begin_generation();
    let response = next.widget_state().take_response(ACTION).unwrap();
    assert_eq!(response.kind, PointerEventKind::Move);
    assert_eq!(response.pointer, 2);
    assert!(next.widget_state().take_response(ACTION).is_none());
    next.abort();
}

#[test]
fn viewport_change_rejects_attempt_without_consuming_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| scene())
        .unwrap();
    core.queue_pointer_press(LogicalPoint::new(10.0, 10.0));

    let mut attempt = core.begin_generation();
    assert!(attempt.widget_state().take_response(ACTION).is_some());
    assert!(core.resize(LogicalSize::new(120.0, 40.0)));
    let error = core
        .finish_generation(attempt, scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert_eq!(
        error,
        FrameError::GenerationViewportChanged {
            attempted: LogicalSize::new(100.0, 40.0),
            current: LogicalSize::new(120.0, 40.0),
        }
    );
    assert_eq!(core.committed_scene().unwrap().generation, 1);
    assert_eq!(consumer.scenes().len(), 1);

    let mut retry = core.begin_generation();
    assert!(retry.widget_state().take_response(ACTION).is_some());
    core.finish_generation(retry, scene(), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(core.committed_scene().unwrap().viewport.width, 120.0);
}

#[test]
fn stale_attempt_is_rejected_without_replacing_newer_commit() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let first = core.begin_generation();
    let stale = core.begin_generation();
    core.finish_generation(first, scene(), &measurer, &mut consumer)
        .unwrap();
    let committed = core.committed_scene().unwrap().clone();

    let error = core
        .finish_generation(stale, scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert_eq!(
        error,
        FrameError::StaleGenerationAttempt {
            base_generation: 0,
            current_generation: 1,
        }
    );
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(consumer.scenes().len(), 1);
}

#[test]
fn attempt_cannot_commit_into_a_different_frame_core_owner() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut left = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut right = FrameCore::new(LogicalSize::new(100.0, 40.0));

    let attempt = left.begin_generation();
    let error = right
        .finish_generation(attempt, scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert!(matches!(error, FrameError::GenerationOwnerMismatch { .. }));
    assert!(left.committed_scene().is_none());
    assert!(right.committed_scene().is_none());
    assert!(consumer.scenes().is_empty());

    let attempt = left.begin_generation();
    left.finish_generation(attempt, scene(), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(left.committed_scene().unwrap().generation, 1);
    assert!(right.committed_scene().is_none());
}

#[test]
fn external_candidate_state_mutation_invalidates_attempt_without_resurrection() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |state| {
        state.request_pointer_capture(7, ACTION);
        scene()
    })
    .unwrap();
    core.run_frame(&measurer, &mut consumer, |_| {
        Element::flex(ROOT, Axis::Column, 0.0).without_paint()
    })
    .unwrap();
    assert_eq!(core.pointer_capture(7), None);

    let attempt = core.begin_generation();
    let cancellation = core.take_cancellation().unwrap();
    assert_eq!(cancellation.pointer, 7);
    let committed = core.committed_scene().unwrap().clone();
    let error = core
        .finish_generation(attempt, scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert!(matches!(error, FrameError::GenerationStateChanged { .. }));
    assert_eq!(core.committed_scene(), Some(&committed));
    assert!(core.take_cancellation().is_none());
    assert_eq!(consumer.scenes().len(), 2);
}

#[test]
fn resize_round_trip_invalidates_attempt_even_when_viewport_matches_again() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let attempt = core.begin_generation();
    assert!(core.resize(LogicalSize::new(120.0, 40.0)));
    assert!(core.resize(LogicalSize::new(100.0, 40.0)));

    let error = core
        .finish_generation(attempt, scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert_eq!(
        error,
        FrameError::GenerationStateChanged {
            attempted_revision: 0,
            current_revision: 2,
        }
    );
    assert!(core.committed_scene().is_none());
    assert!(consumer.scenes().is_empty());
}

#[test]
fn split_wheel_prefixes_retry_after_failure_and_preserve_mid_attempt_events() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.run_frame(&measurer, &mut consumer, |_| scroll_scene())
        .unwrap();

    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    let failed = core.begin_generation();
    let error = core
        .finish_generation(failed, invalid_scene(), &measurer, &mut consumer)
        .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(DUPLICATE));
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 0.0);

    let retry = core.begin_generation();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    core.finish_generation(retry, scroll_scene(), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 20.0);

    let remaining = core.begin_generation();
    core.finish_generation(remaining, scroll_scene(), &measurer, &mut consumer)
        .unwrap();
    assert_eq!(core.scroll_offset(SCROLL).unwrap().applied.y, 40.0);
}
