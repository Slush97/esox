use std::cell::{Cell, RefCell};

use esox_ui::declaration::{run_declaration_frame, CheckboxIds, CheckboxStyle, DeclarationStyle};
use esox_ui::frame_core::{
    CheckboxDeclarationError, Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint,
    LogicalRect, LogicalSize, NullSceneConsumer, PaintPrimitive, PointerEventKind, SemanticRole,
    WidgetId,
};
use esox_ui::scene_submission::{preflight_display_list, SubmissionPrimitive};

const ROOT: WidgetId = WidgetId(440_000);
const CHECKBOX: WidgetId = WidgetId(440_001);
const HIDDEN: WidgetId = WidgetId(440_002);
const DISABLED: WidgetId = WidgetId(440_003);
const INVALID: WidgetId = WidgetId(440_004);

const UNCHECKED: Color = Color::rgba(0.1, 0.2, 0.3, 1.0);
const CHECKED: Color = Color::rgba(0.4, 0.5, 0.6, 1.0);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn style() -> CheckboxStyle {
    CheckboxStyle::default()
        .layout(DeclarationStyle::new().height(36.0))
        .indicator_size(18.0)
        .gap(8.0)
        .radius(4.0)
        .fills(UNCHECKED, CHECKED)
}

#[test]
fn checkbox_declares_once_with_current_geometry_stable_parts_and_exact_vocabulary() {
    let calls = Cell::new(0);
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(120.0, 60.0));
    let ids = CheckboxIds::new(CHECKBOX);

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        calls.set(calls.get() + 1);
        ui.column(ROOT, DeclarationStyle::new().padding(4.0), |ui| {
            let response = ui.checkbox(CHECKBOX, "Enabled", true, style()).unwrap();
            assert!(!response.changed);
        });
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert!(core.resize(LogicalSize::new(80.0, 60.0)));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert_eq!(calls.get(), 2);

    let first = &consumer.scenes()[0];
    let resized = &consumer.scenes()[1];
    assert_eq!(
        first.node(CHECKBOX).unwrap().bounds,
        rect(4.0, 4.0, 112.0, 36.0)
    );
    assert_eq!(
        resized.node(CHECKBOX).unwrap().bounds,
        rect(4.0, 4.0, 72.0, 36.0)
    );
    assert_eq!(
        resized.node(ids.indicator).unwrap().bounds,
        rect(4.0, 13.0, 18.0, 18.0)
    );
    assert_eq!(resized.node(ids.label).unwrap().bounds.x, 30.0);
    assert!(resized.node(ids.mark).is_some());

    let indicator = resized
        .display_list
        .iter()
        .find(|record| record.id == ids.indicator)
        .unwrap();
    assert_eq!(
        indicator.primitive,
        PaintPrimitive::RoundedRect {
            color: CHECKED,
            radius: 4.0,
        }
    );
    let semantic = resized.semantics.node(CHECKBOX).unwrap();
    assert_eq!(semantic.properties.role, SemanticRole::Checkbox);
    assert_eq!(semantic.properties.label.as_deref(), Some("Enabled"));
    assert_eq!(semantic.properties.checked, Some(true));
    assert_eq!(semantic.bounds, resized.node(CHECKBOX).unwrap().bounds);
    assert_eq!(resized.hit_index[0].id, CHECKBOX);

    let prepared = preflight_display_list(&resized.display_list).unwrap();
    assert!(matches!(
        prepared
            .records()
            .iter()
            .find(|record| record.id == ids.indicator)
            .unwrap()
            .primitive,
        SubmissionPrimitive::RoundedRect {
            color: CHECKED,
            radius: 4.0,
        }
    ));
}

#[test]
fn checkbox_pointer_response_is_controlled_and_transactional() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 50.0));
    let observed = RefCell::new(Vec::new());

    let frame = |core: &mut FrameCore, consumer: &mut NullSceneConsumer, checked| {
        run_declaration_frame(core, &measurer, consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let response = ui
                    .checkbox(CHECKBOX, "Controlled", checked, style())
                    .unwrap();
                observed
                    .borrow_mut()
                    .push((response.clicked, response.changed, response.pressed));
            });
        })
        .unwrap();
    };

    frame(&mut core, &mut consumer, false);
    assert!(core.queue_pointer_event(PointerEventKind::Press, 7, LogicalPoint::new(5.0, 5.0)));
    frame(&mut core, &mut consumer, false);
    assert!(core.queue_pointer_event(PointerEventKind::Release, 7, LogicalPoint::new(5.0, 5.0)));
    frame(&mut core, &mut consumer, false);
    frame(&mut core, &mut consumer, true);

    assert_eq!(
        *observed.borrow(),
        [
            (false, false, false),
            (false, false, true),
            (true, true, false),
            (false, false, false),
        ]
    );
    assert_eq!(
        consumer.scenes()[2]
            .semantics
            .node(CHECKBOX)
            .unwrap()
            .properties
            .checked,
        Some(false)
    );
    assert_eq!(
        consumer.scenes()[3]
            .semantics
            .node(CHECKBOX)
            .unwrap()
            .properties
            .checked,
        Some(true)
    );
}

#[test]
fn hidden_and_disabled_checkboxes_preserve_participation_contracts() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 80.0));
    let responses = RefCell::new(Vec::new());

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            responses.borrow_mut().push(
                ui.checkbox(
                    HIDDEN,
                    "Hidden",
                    true,
                    style().layout(DeclarationStyle::new().height(36.0).hidden()),
                )
                .unwrap(),
            );
            responses.borrow_mut().push(
                ui.checkbox(DISABLED, "Disabled", false, style().disabled())
                    .unwrap(),
            );
        });
    })
    .unwrap();

    assert!(scene.node(HIDDEN).unwrap().effective_hidden);
    assert!(scene.semantics.node(HIDDEN).is_none());
    let hidden_ids = CheckboxIds::new(HIDDEN);
    assert!(scene.display_list.iter().all(|record| ![
        hidden_ids.indicator,
        hidden_ids.mark,
        hidden_ids.label,
    ]
    .contains(&record.id)));
    assert!(scene.node(DISABLED).unwrap().effective_disabled);
    assert!(scene.hit_index.iter().all(|hit| hit.id != DISABLED));
    let semantic = scene.semantics.node(DISABLED).unwrap();
    assert!(semantic.properties.disabled);
    assert_eq!(semantic.properties.checked, Some(false));
    assert!(!responses.borrow()[0].clicked);
    assert!(responses.borrow()[1].disabled);
}

#[test]
fn invalid_checkbox_is_typed_and_failed_generation_replays_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 50.0));
    let observed = RefCell::new(Vec::new());

    let frame = |core: &mut FrameCore,
                 consumer: &mut NullSceneConsumer,
                 radius: f32|
     -> Result<(), FrameError> {
        run_declaration_frame(core, &measurer, consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let response = ui.checkbox(CHECKBOX, "Retry", false, style()).unwrap();
                observed.borrow_mut().push(response.pressed);
                let _ = ui.checkbox(INVALID, "Invalid", false, style().radius(radius));
            });
        })
        .map(|_| ())
    };

    frame(&mut core, &mut consumer, 4.0).unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 9, LogicalPoint::new(5.0, 5.0)));
    assert_eq!(
        frame(&mut core, &mut consumer, 10.0),
        Err(FrameError::InvalidCheckbox {
            id: INVALID,
            error: CheckboxDeclarationError::RadiusExceedsIndicator {
                radius: 10.0,
                indicator_size: 18.0,
            },
        })
    );
    assert_eq!(consumer.scenes().len(), 1);

    frame(&mut core, &mut consumer, 4.0).unwrap();
    assert_eq!(*observed.borrow(), [false, true, true]);
    assert_eq!(consumer.scenes().len(), 2);
}

#[test]
fn invalid_checkbox_geometry_is_rejected_without_clamping_or_paint_fallback() {
    let cases = [
        (
            style().indicator_size(0.0),
            CheckboxDeclarationError::InvalidIndicatorSize(0.0),
        ),
        (
            style().gap(f32::INFINITY),
            CheckboxDeclarationError::InvalidGap(f32::INFINITY),
        ),
        (
            style().radius(-1.0),
            CheckboxDeclarationError::InvalidRadius(-1.0),
        ),
    ];

    for (style, expected) in cases {
        let measurer = DeterministicMeasurer::new(8.0, 18.0);
        let mut consumer = NullSceneConsumer::default();
        let mut core = FrameCore::new(LogicalSize::new(100.0, 50.0));
        let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let _ = ui.checkbox(INVALID, "Invalid", false, style);
            });
        })
        .unwrap_err();
        assert_eq!(
            error,
            FrameError::InvalidCheckbox {
                id: INVALID,
                error: expected,
            }
        );
        assert!(consumer.scenes().is_empty());
    }
}
