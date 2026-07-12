use esox_input::{Key, KeyCode, KeyEvent, Modifiers, NamedKey, SmolStr};
use esox_ui::frame_core::{
    Axis, DeterministicMeasurer, Element, FrameCore, FrameError, LogicalSize, NullSceneConsumer,
    WidgetId,
};

const ROOT: WidgetId = WidgetId(1);
const FIRST: WidgetId = WidgetId(2);
const SECOND: WidgetId = WidgetId(3);

fn focus_scene() -> Element {
    Element::flex(ROOT, Axis::Column, 0.0)
        .without_paint()
        .with_children(vec![
            Element::fixed(FIRST, 100.0, 20.0).interactive(),
            Element::fixed(SECOND, 100.0, 20.0).interactive(),
        ])
}

fn named_key(key: NamedKey, physical_key: KeyCode) -> KeyEvent {
    KeyEvent {
        key: Key::Named(key),
        physical_key,
        pressed: true,
        repeat: false,
        text: None,
    }
}

fn character_key(text: &str, physical_key: KeyCode) -> KeyEvent {
    KeyEvent {
        key: Key::Character(SmolStr::new(text)),
        physical_key,
        pressed: true,
        repeat: false,
        text: Some(SmolStr::new(text)),
    }
}

#[test]
fn keyboard_ledger_is_focus_routed_ordered_retryable_and_per_window() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut left_consumer = NullSceneConsumer::default();
    let mut right_consumer = NullSceneConsumer::default();
    let mut left = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut right = FrameCore::new(LogicalSize::new(100.0, 40.0));

    left.run_frame(&measurer, &mut left_consumer, |state| {
        state.request_keyboard_focus(FIRST);
        focus_scene()
    })
    .unwrap();
    right
        .run_frame(&measurer, &mut right_consumer, |state| {
            state.request_keyboard_focus(SECOND);
            focus_scene()
        })
        .unwrap();

    let first_event = named_key(NamedKey::ArrowDown, KeyCode::ArrowDown);
    let mut second_event = character_key("x", KeyCode::KeyX);
    second_event.pressed = false;
    second_event.repeat = true;
    left.queue_keyboard_input(first_event.clone(), Modifiers::empty());
    left.queue_keyboard_input(second_event.clone(), Modifiers::empty().with_shift());
    right.queue_keyboard_input(
        named_key(NamedKey::Enter, KeyCode::Enter),
        Modifiers::empty().with_ctrl(),
    );

    let error = left
        .run_frame(&measurer, &mut left_consumer, |state| {
            let first = state.take_keyboard_response(FIRST).unwrap();
            let second = state.take_keyboard_response(FIRST).unwrap();
            assert_eq!(first.event, first_event);
            assert_eq!(second.event, second_event);
            assert_eq!((first.dispatch_ordinal, second.dispatch_ordinal), (0, 1));
            assert!(second.modifiers.shift());
            state.request_keyboard_focus(SECOND);
            Element::flex(ROOT, Axis::Column, 0.0).with_children(vec![
                Element::fixed(SECOND, 50.0, 20.0),
                Element::fixed(SECOND, 50.0, 20.0),
            ])
        })
        .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(SECOND));
    assert_eq!(left.keyboard_focus(), Some(FIRST));

    left.run_frame(&measurer, &mut left_consumer, |state| {
        let first = state.take_keyboard_response(FIRST).unwrap();
        let second = state.take_keyboard_response(FIRST).unwrap();
        assert_eq!(first.committed_generation, 1);
        assert_eq!(first.target, FIRST);
        assert_eq!(first.event, first_event);
        assert_eq!(second.event, second_event);
        assert_eq!((first.dispatch_ordinal, second.dispatch_ordinal), (0, 1));
        assert_eq!(state.take_keyboard_response(FIRST), None);
        assert_eq!(state.take_keyboard_response(SECOND), None);
        state.request_keyboard_focus(SECOND);
        focus_scene()
    })
    .unwrap();
    assert_eq!(left.keyboard_focus(), Some(SECOND));

    right
        .run_frame(&measurer, &mut right_consumer, |state| {
            assert_eq!(state.take_keyboard_response(FIRST), None);
            let response = state.take_keyboard_response(SECOND).unwrap();
            assert_eq!(response.target, SECOND);
            assert_eq!(response.event.key, Key::Named(NamedKey::Enter));
            assert!(response.modifiers.ctrl());
            assert_eq!(state.take_keyboard_response(SECOND), None);
            focus_scene()
        })
        .unwrap();

    left.run_frame(&measurer, &mut left_consumer, |state| {
        assert_eq!(state.take_keyboard_response(FIRST), None);
        assert_eq!(state.take_keyboard_response(SECOND), None);
        focus_scene()
    })
    .unwrap();
}

#[test]
fn unconsumed_keyboard_response_is_removed_with_an_inactive_focus_target() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    for target_state in ["removed", "hidden", "disabled"] {
        let mut consumer = NullSceneConsumer::default();
        let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

        core.run_frame(&measurer, &mut consumer, |state| {
            state.request_keyboard_focus(FIRST);
            focus_scene()
        })
        .unwrap();
        core.queue_keyboard_input(
            named_key(NamedKey::ArrowDown, KeyCode::ArrowDown),
            Modifiers::empty(),
        );
        core.run_frame(&measurer, &mut consumer, |_| {
            let mut children = Vec::new();
            if target_state != "removed" {
                let first = Element::fixed(FIRST, 100.0, 20.0).interactive();
                children.push(match target_state {
                    "hidden" => first.with_hidden(true),
                    "disabled" => first.disabled(),
                    _ => unreachable!(),
                });
            }
            children.push(Element::fixed(SECOND, 100.0, 20.0).interactive());
            Element::flex(ROOT, Axis::Column, 0.0)
                .without_paint()
                .with_children(children)
        })
        .unwrap();
        core.run_frame(&measurer, &mut consumer, |state| {
            assert_eq!(state.take_keyboard_response(FIRST), None);
            focus_scene()
        })
        .unwrap();
    }
}

#[test]
fn keyboard_prefix_commits_without_consuming_later_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    core.queue_keyboard_input(
        named_key(NamedKey::Enter, KeyCode::Enter),
        Modifiers::empty(),
    );
    core.run_frame(&measurer, &mut consumer, |state| {
        assert_eq!(state.take_keyboard_response(FIRST), None);
        state.request_keyboard_focus(FIRST);
        focus_scene()
    })
    .unwrap();

    core.queue_keyboard_input(
        named_key(NamedKey::ArrowDown, KeyCode::ArrowDown),
        Modifiers::empty(),
    );
    core.begin_generation().abort();
    let mut attempt = core.begin_generation();
    let response = attempt
        .widget_state()
        .take_keyboard_response(FIRST)
        .unwrap();
    assert_eq!(response.event.key, Key::Named(NamedKey::ArrowDown));
    core.queue_keyboard_input(
        named_key(NamedKey::ArrowUp, KeyCode::ArrowUp),
        Modifiers::empty(),
    );
    core.finish_generation(attempt, focus_scene(), &measurer, &mut consumer)
        .unwrap();

    core.run_frame(&measurer, &mut consumer, |state| {
        let response = state.take_keyboard_response(FIRST).unwrap();
        assert_eq!(response.event.key, Key::Named(NamedKey::ArrowUp));
        assert_eq!(response.dispatch_ordinal, 0);
        assert_eq!(state.take_keyboard_response(FIRST), None);
        focus_scene()
    })
    .unwrap();
}
