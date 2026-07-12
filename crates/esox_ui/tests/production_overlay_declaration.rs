use std::cell::Cell;

use esox_ui::declaration::{run_declaration_frame, ButtonStyle, DeclarationStyle, DeclarationUi};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, PointerEventKind, WidgetId,
};

const ROOT: WidgetId = WidgetId(1);
const CONTENT: WidgetId = WidgetId(2);
const OVERLAY: WidgetId = WidgetId(3);
const OVERLAY_ACTION: WidgetId = WidgetId(4);
const DUPLICATE: WidgetId = WidgetId(5);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Clone, Copy)]
enum OverlayState {
    Removed,
    Visible,
    Hidden,
}

fn declare_tree(
    ui: &mut DeclarationUi<'_>,
    overlay: OverlayState,
    request_capture: Option<u64>,
    release_capture: Option<u64>,
    duplicate: bool,
) {
    if let Some(pointer) = request_capture {
        ui.request_pointer_capture(pointer, OVERLAY_ACTION);
    }
    if let Some(pointer) = release_capture {
        ui.request_pointer_release(pointer);
    }
    ui.column(ROOT, DeclarationStyle::new().size(200.0, 120.0), |ui| {
        ui.button(
            CONTENT,
            "Content",
            ButtonStyle::default().layout(DeclarationStyle::new().size(200.0, 120.0)),
        );
        if !matches!(overlay, OverlayState::Removed) {
            let overlay_style = DeclarationStyle::new()
                .size(100.0, 60.0)
                .absolute_position(20.0, 20.0)
                .background(Color::rgba(0.2, 0.2, 0.2, 1.0))
                .blocking_overlay()
                .with_hidden(matches!(overlay, OverlayState::Hidden));
            ui.column(OVERLAY, overlay_style, |ui| {
                ui.button(
                    OVERLAY_ACTION,
                    "Overlay action",
                    ButtonStyle::default().layout(DeclarationStyle::new().size(40.0, 20.0)),
                );
                if duplicate {
                    ui.solid_rect(
                        DUPLICATE,
                        DeclarationStyle::new().size(1.0, 1.0),
                        Color::BLACK,
                    );
                    ui.solid_rect(
                        DUPLICATE,
                        DeclarationStyle::new().size(1.0, 1.0),
                        Color::BLACK,
                    );
                }
            });
        }
    });
}

#[test]
fn production_blocking_overlay_owns_order_semantics_and_focus_scope() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 120.0));
    let calls = Cell::new(0);

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        calls.set(calls.get() + 1);
        declare_tree(ui, OverlayState::Visible, None, None, false);
    })
    .unwrap()
    .clone();

    assert_eq!(calls.get(), 1);
    let overlay = scene.node(OVERLAY).unwrap();
    assert_eq!(overlay.bounds, rect(20.0, 20.0, 100.0, 60.0));
    assert!(overlay.blocks_input);
    assert_eq!(overlay.focus_scope, Some(OVERLAY));
    assert_eq!(scene.focus_order, vec![OVERLAY, OVERLAY_ACTION]);
    assert_eq!(scene.focus_scopes.len(), 1);
    assert_eq!(scene.focus_scopes[0].owner, OVERLAY);
    assert_eq!(scene.focus_scopes[0].members, scene.focus_order);
    assert_eq!(core.keyboard_focus(), Some(OVERLAY));
    assert_eq!(
        scene
            .hit_test(LogicalPoint::new(90.0, 50.0))
            .map(|hit| hit.id),
        Some(OVERLAY)
    );

    let paint_ids: Vec<_> = scene.display_list.iter().map(|record| record.id).collect();
    assert!(
        paint_ids.iter().position(|id| *id == CONTENT).unwrap()
            < paint_ids.iter().position(|id| *id == OVERLAY).unwrap()
    );
    assert!(
        paint_ids.iter().position(|id| *id == OVERLAY).unwrap()
            < paint_ids
                .iter()
                .position(|id| *id == OVERLAY_ACTION)
                .unwrap()
    );
    assert_eq!(
        scene.semantics.node(OVERLAY).unwrap().children[0],
        OVERLAY_ACTION
    );
    assert_eq!(
        scene.semantics.node(OVERLAY_ACTION).unwrap().parent,
        Some(OVERLAY)
    );
}

#[test]
fn production_overlay_hiding_and_removal_restore_focus_and_cancel_capture() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);

    for closed in [OverlayState::Hidden, OverlayState::Removed] {
        let mut consumer = NullSceneConsumer::default();
        let mut core = FrameCore::new(LogicalSize::new(200.0, 120.0));
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.request_keyboard_focus(CONTENT);
            declare_tree(ui, OverlayState::Removed, None, None, false);
        })
        .unwrap();
        assert_eq!(core.keyboard_focus(), Some(CONTENT));

        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            declare_tree(ui, OverlayState::Visible, Some(7), None, false);
        })
        .unwrap();
        assert_eq!(core.keyboard_focus(), Some(OVERLAY));
        assert_eq!(core.pointer_capture(7), Some(OVERLAY_ACTION));

        let closed_scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            declare_tree(ui, closed, None, None, false);
        })
        .unwrap();
        assert!(closed_scene.focus_scopes.is_empty());
        assert_eq!(core.keyboard_focus(), Some(CONTENT));
        assert_eq!(core.pointer_capture(7), None);
        let cancellation = core.take_cancellation().unwrap();
        assert_eq!(cancellation.kind, PointerEventKind::Cancel);
        assert_eq!(cancellation.pointer, 7);
        assert_eq!(cancellation.target, OVERLAY_ACTION);
        assert_eq!(cancellation.committed_generation, 2);
        assert_eq!(cancellation.target_bounds, rect(20.0, 20.0, 40.0, 20.0));
        assert_eq!(core.take_cancellation(), None);
    }
}

#[test]
fn production_overlay_requests_are_transactional_and_retry_once() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 120.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.request_keyboard_focus(CONTENT);
        declare_tree(ui, OverlayState::Removed, None, None, false);
    })
    .unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 11, LogicalPoint::new(150.0, 100.0),));

    let failed_calls = Cell::new(0);
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        failed_calls.set(failed_calls.get() + 1);
        declare_tree(ui, OverlayState::Visible, Some(11), None, true);
    })
    .unwrap_err();
    assert_eq!(failed_calls.get(), 1);
    assert_eq!(error, FrameError::DuplicateWidgetId(DUPLICATE));
    assert_eq!(core.committed_scene().unwrap().generation, 1);
    assert_eq!(consumer.scenes().len(), 1);
    assert_eq!(core.keyboard_focus(), Some(CONTENT));
    assert_eq!(core.pointer_capture(11), None);
    assert_eq!(core.take_cancellation(), None);

    let retry_calls = Cell::new(0);
    let mut content_pressed = false;
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        retry_calls.set(retry_calls.get() + 1);
        ui.request_pointer_capture(11, OVERLAY_ACTION);
        ui.column(ROOT, DeclarationStyle::new().size(200.0, 120.0), |ui| {
            content_pressed = ui
                .button(
                    CONTENT,
                    "Content",
                    ButtonStyle::default().layout(DeclarationStyle::new().size(200.0, 120.0)),
                )
                .pressed;
            ui.column(
                OVERLAY,
                DeclarationStyle::new()
                    .size(100.0, 60.0)
                    .absolute_position(20.0, 20.0)
                    .blocking_overlay(),
                |ui| {
                    ui.button(
                        OVERLAY_ACTION,
                        "Overlay action",
                        ButtonStyle::default().layout(DeclarationStyle::new().size(40.0, 20.0)),
                    );
                },
            );
        });
    })
    .unwrap();
    assert_eq!(retry_calls.get(), 1);
    assert!(content_pressed);
    assert_eq!(core.committed_scene().unwrap().generation, 2);
    assert_eq!(consumer.scenes().len(), 2);
    assert_eq!(core.keyboard_focus(), Some(OVERLAY));
    assert_eq!(core.pointer_capture(11), Some(OVERLAY_ACTION));

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_tree(ui, OverlayState::Visible, None, Some(11), false);
    })
    .unwrap();
    assert_eq!(core.pointer_capture(11), None);
    assert_eq!(core.take_cancellation(), None);
}
