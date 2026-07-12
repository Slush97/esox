use std::cell::Cell;

use esox_input::CursorIcon;
use esox_ui::declaration::{
    run_declaration_frame, ButtonStyle, DeclarationStyle, LogicalTransform, SplitPaneIds,
    SplitPaneStyle,
};
use esox_ui::frame_core::{
    Axis, Color, DeterministicMeasurer, Element, FrameCore, FrameError, LogicalPoint, LogicalRect,
    LogicalSize, NullSceneConsumer, PaintPrimitive, PointerEventKind, SemanticRole, WidgetId,
};

const ROOT: WidgetId = WidgetId(100);
const FIRST_LEAF: WidgetId = WidgetId(101);
const SECOND_LEAF: WidgetId = WidgetId(102);
const DUPLICATE: WidgetId = WidgetId(103);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn style() -> SplitPaneStyle {
    SplitPaneStyle::default()
        .layout(DeclarationStyle::new().size(200.0, 100.0))
        .divider(10.0, Color::rgba(0.3, 0.4, 0.5, 1.0))
}

fn declare_h(ui: &mut esox_ui::declaration::DeclarationUi<'_>, ratio: f32, duplicate: bool) {
    ui.split_pane_h(
        ROOT,
        ratio,
        style(),
        |ui| {
            ui.solid_rect(
                FIRST_LEAF,
                DeclarationStyle::new().size(400.0, 20.0),
                Color::BLACK,
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
        },
        |ui| {
            ui.solid_rect(
                SECOND_LEAF,
                DeclarationStyle::new().size(300.0, 20.0),
                Color::BLACK,
            );
        },
    );
}

#[test]
fn horizontal_and_vertical_geometry_are_once_only_and_intrinsic_independent() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let ids = SplitPaneIds::new(ROOT);
    for vertical in [false, true] {
        let mut core = FrameCore::new(LogicalSize::new(240.0, 140.0));
        let mut consumer = NullSceneConsumer::default();
        let calls = Cell::new(0);
        let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            let first = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
                calls.set(calls.get() + 1);
                ui.solid_rect(
                    FIRST_LEAF,
                    DeclarationStyle::new().size(400.0, 300.0),
                    Color::BLACK,
                );
            };
            let second = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
                calls.set(calls.get() + 1);
                ui.solid_rect(
                    SECOND_LEAF,
                    DeclarationStyle::new().size(300.0, 400.0),
                    Color::BLACK,
                );
            };
            if vertical {
                ui.split_pane_v(ROOT, 0.25, style(), first, second);
            } else {
                ui.split_pane_h(ROOT, 0.25, style(), first, second);
            }
        })
        .unwrap();
        assert_eq!(calls.get(), 2);
        if vertical {
            assert_eq!(
                scene.node(ids.first).unwrap().bounds,
                rect(0.0, 0.0, 240.0, 32.5)
            );
            assert_eq!(
                scene.node(ids.divider).unwrap().bounds,
                rect(0.0, 32.5, 240.0, 10.0)
            );
            assert_eq!(
                scene.node(ids.second).unwrap().bounds,
                rect(0.0, 42.5, 240.0, 97.5)
            );
            assert_eq!(
                scene.cursor_icon_at(LogicalPoint::new(50.0, 35.0)),
                CursorIcon::RowResize
            );
        } else {
            assert_eq!(
                scene.node(ids.first).unwrap().bounds,
                rect(0.0, 0.0, 57.5, 140.0)
            );
            assert_eq!(
                scene.node(ids.divider).unwrap().bounds,
                rect(57.5, 0.0, 10.0, 140.0)
            );
            assert_eq!(
                scene.node(ids.second).unwrap().bounds,
                rect(67.5, 0.0, 172.5, 140.0)
            );
            assert_eq!(
                scene.cursor_icon_at(LogicalPoint::new(60.0, 50.0)),
                CursorIcon::ColResize
            );
        }
        let divider_paint = scene
            .display_list
            .iter()
            .find(|paint| paint.id == ids.divider)
            .unwrap();
        assert!(matches!(
            divider_paint.primitive,
            PaintPrimitive::SolidRect { .. }
        ));
        assert_eq!(
            scene.semantics.node(ids.divider).unwrap().properties.role,
            SemanticRole::Generic
        );
    }
}

#[test]
fn resize_recomputes_current_geometry_and_cursor_obeys_transform_clip_and_topmost_hit() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let ids = SplitPaneIds::new(ROOT);
    let transformed = SplitPaneStyle::default()
        .layout(
            DeclarationStyle::new()
                .size(200.0, 100.0)
                .clip_children()
                .transform(LogicalTransform::translate(20.0, 10.0)),
        )
        .divider(10.0, Color::BLACK);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.split_pane_h(ROOT, 0.5, transformed, |_| {}, |_| {});
    })
    .unwrap();
    let scene = core.committed_scene().unwrap();
    assert_eq!(
        scene.node(ids.divider).unwrap().transformed_bounds,
        rect(115.0, 10.0, 10.0, 100.0)
    );
    assert_eq!(
        scene.cursor_icon_at(LogicalPoint::new(120.0, 50.0)),
        CursorIcon::ColResize
    );
    assert_eq!(
        scene.cursor_icon_at(LogicalPoint::new(120.0, 115.0)),
        CursorIcon::Default
    );

    assert!(core.resize(LogicalSize::new(300.0, 100.0)));
    let resized = SplitPaneStyle::default()
        .layout(DeclarationStyle::new().size(300.0, 100.0))
        .divider(10.0, Color::BLACK);
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.split_pane_h(ROOT, 0.5, resized, |_| {}, |_| {});
    })
    .unwrap();
    assert_eq!(scene.node(ids.divider).unwrap().bounds.x, 145.0);

    let topmost = core
        .run_frame(&measurer, &mut consumer, |_| {
            Element::flex(ROOT, Axis::Column, 0.0)
                .without_paint()
                .with_children(vec![
                    Element::fixed(FIRST_LEAF, 40.0, 40.0)
                        .with_cursor_icon(CursorIcon::ColResize)
                        .interactive(),
                    Element::fixed(SECOND_LEAF, 40.0, 40.0)
                        .with_absolute_position(0.0, 0.0)
                        .interactive(),
                ])
        })
        .unwrap();
    assert_eq!(
        topmost.cursor_icon_at(LogicalPoint::new(10.0, 10.0)),
        CursorIcon::Default
    );
}

#[test]
fn capture_move_release_clamp_and_same_batch_routing_are_transactional() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let ids = SplitPaneIds::new(ROOT);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 7, LogicalPoint::new(100.0, 50.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(core.pointer_capture(7), Some(ids.divider));
    assert_eq!(
        core.cursor_icon_at(7, LogicalPoint::new(-50.0, 50.0)),
        CursorIcon::ColResize
    );

    core.queue_pointer_event(PointerEventKind::Move, 7, LogicalPoint::new(-100.0, 50.0));
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(scene.node(ids.first).unwrap().bounds.width, 9.5);
    core.queue_pointer_event(
        PointerEventKind::Release,
        7,
        LogicalPoint::new(-100.0, 50.0),
    );
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(core.pointer_capture(7), None);

    for (kind, x) in [
        (PointerEventKind::Press, 19.0),
        (PointerEventKind::Move, 500.0),
        (PointerEventKind::Release, 500.0),
    ] {
        core.queue_pointer_event(kind, 8, LogicalPoint::new(x, 50.0));
    }
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(scene.node(ids.first).unwrap().bounds.width, 180.5);
    assert_eq!(core.pointer_capture(8), None);
}

#[test]
fn captured_move_then_release_in_one_batch_updates_ratio_before_clearing_capture() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let ids = SplitPaneIds::new(ROOT);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 12, LogicalPoint::new(100.0, 50.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(core.pointer_capture(12), Some(ids.divider));

    core.queue_pointer_event(PointerEventKind::Move, 12, LogicalPoint::new(150.0, 50.0));
    core.queue_pointer_event(
        PointerEventKind::Release,
        12,
        LogicalPoint::new(150.0, 50.0),
    );
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();

    assert_eq!(scene.node(ids.first).unwrap().bounds.width, 145.0);
    assert_eq!(core.pointer_capture(12), None);
}

#[test]
fn competing_press_hidden_disabled_and_removal_do_not_retain_drag() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let ids = SplitPaneIds::new(ROOT);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 1, LogicalPoint::new(100.0, 50.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 2, LogicalPoint::new(100.0, 50.0));
    core.queue_pointer_event(PointerEventKind::Move, 2, LogicalPoint::new(180.0, 50.0));
    let unchanged = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(unchanged.node(ids.first).unwrap().bounds.width, 95.0);
    assert_eq!(core.pointer_capture(1), Some(ids.divider));

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new().size(200.0, 100.0), |_| {});
    })
    .unwrap();
    assert_eq!(core.pointer_capture(1), None);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Move, 1, LogicalPoint::new(180.0, 50.0));
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(scene.node(ids.first).unwrap().bounds.width, 95.0);

    for disabled in [false, true] {
        let hidden_style = style().layout(
            DeclarationStyle::new()
                .size(200.0, 100.0)
                .with_hidden(!disabled)
                .with_disabled(disabled),
        );
        let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.split_pane_h(ROOT, 0.5, hidden_style, |_| {}, |_| {});
        })
        .unwrap();
        assert_eq!(
            scene.cursor_icon_at(LogicalPoint::new(100.0, 50.0)),
            CursorIcon::Default
        );
    }
}

#[test]
fn failed_generation_rolls_back_split_state_input_and_capture_for_retry() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let ids = SplitPaneIds::new(ROOT);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 4, LogicalPoint::new(100.0, 50.0));
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, true)
    })
    .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(DUPLICATE));
    assert_eq!(core.pointer_capture(4), None);
    assert_eq!(core.committed_scene().unwrap().generation, 1);
    assert_eq!(consumer.scenes().len(), 1);

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_h(ui, 0.5, false)
    })
    .unwrap();
    assert_eq!(core.pointer_capture(4), Some(ids.divider));
    assert_eq!(core.committed_scene().unwrap().generation, 2);
}

#[test]
fn invalid_flex_basis_is_typed_and_atomic() {
    let mut core = FrameCore::new(LogicalSize::new(100.0, 100.0));
    let mut consumer = NullSceneConsumer::default();
    let error = core
        .run_frame(
            &DeterministicMeasurer::new(8.0, 18.0),
            &mut consumer,
            |_| esox_ui::frame_core::Element::fixed(ROOT, 10.0, 10.0).with_flex_basis(f32::NAN),
        )
        .unwrap_err();
    assert!(matches!(error, FrameError::InvalidFlexBasis { id: ROOT, value } if value.is_nan()));
    assert!(core.committed_scene().is_none());
}

#[test]
fn ordinary_button_release_outside_does_not_click_or_gain_implicit_capture() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut consumer = NullSceneConsumer::default();
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.button(
            ROOT,
            "button",
            ButtonStyle::default().layout(DeclarationStyle::new().size(100.0, 40.0)),
        );
    })
    .unwrap();
    for (kind, point) in [
        (PointerEventKind::Press, LogicalPoint::new(10.0, 10.0)),
        (PointerEventKind::Move, LogicalPoint::new(150.0, 10.0)),
        (PointerEventKind::Release, LogicalPoint::new(150.0, 10.0)),
    ] {
        core.queue_pointer_event(kind, 99, point);
    }
    let mut clicked = true;
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        clicked = ui
            .button(
                ROOT,
                "button",
                ButtonStyle::default().layout(DeclarationStyle::new().size(100.0, 40.0)),
            )
            .clicked;
    })
    .unwrap();
    assert!(!clicked);
    assert_eq!(core.pointer_capture(99), None);
}
