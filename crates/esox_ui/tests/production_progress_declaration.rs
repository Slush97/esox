use std::cell::{Cell, RefCell};

use esox_ui::declaration::{
    progress_fill_id, run_declaration_frame, ButtonStyle, DeclarationStyle, ProgressStyle,
};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, PaintPrimitive, PointerEventKind, ProgressDeclarationError, SemanticRole,
    SemanticValueRange, WidgetId,
};
use esox_ui::scene_submission::{preflight_display_list, SubmissionPrimitive};

const ROOT: WidgetId = WidgetId(430_000);
const PROGRESS: WidgetId = WidgetId(430_001);
const HIDDEN: WidgetId = WidgetId(430_002);
const DISABLED: WidgetId = WidgetId(430_003);
const BUTTON: WidgetId = WidgetId(430_004);
const INVALID: WidgetId = WidgetId(430_005);

const TRACK: Color = Color::rgba(0.1, 0.2, 0.3, 1.0);
const FILL: Color = Color::rgba(0.4, 0.6, 0.8, 1.0);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn determinate_progress_declares_once_with_current_geometry_and_exact_vocabulary() {
    let calls = Cell::new(0);
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(120.0, 50.0));

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        calls.set(calls.get() + 1);
        ui.column(ROOT, DeclarationStyle::new().padding(4.0), |ui| {
            ui.progress(
                PROGRESS,
                ProgressStyle::new(25.0, TRACK, FILL)
                    .range(0.0, 100.0, 25.0)
                    .radius(6.0)
                    .layout(DeclarationStyle::new().height(12.0)),
            )
            .unwrap();
        });
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert!(core.resize(LogicalSize::new(88.0, 50.0)));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert_eq!(calls.get(), 2);

    let first = &consumer.scenes()[0];
    let resized = &consumer.scenes()[1];
    assert_eq!(
        first.node(PROGRESS).unwrap().bounds,
        rect(4.0, 4.0, 112.0, 12.0)
    );
    assert_eq!(
        resized.node(PROGRESS).unwrap().bounds,
        rect(4.0, 4.0, 80.0, 12.0)
    );
    assert_eq!(
        resized.node(progress_fill_id(PROGRESS)).unwrap().bounds,
        rect(4.0, 4.0, 20.0, 12.0)
    );

    let track = resized
        .display_list
        .iter()
        .find(|record| record.id == PROGRESS)
        .unwrap();
    let fill = resized
        .display_list
        .iter()
        .find(|record| record.id == progress_fill_id(PROGRESS))
        .unwrap();
    assert_eq!(
        track.primitive,
        PaintPrimitive::RoundedRect {
            color: TRACK,
            radius: 6.0,
        }
    );
    assert_eq!(
        fill.primitive,
        PaintPrimitive::RoundedRect {
            color: FILL,
            radius: 6.0,
        }
    );
    assert_eq!(fill.bounds, rect(4.0, 4.0, 20.0, 12.0));

    let semantic = resized.semantics.node(PROGRESS).unwrap();
    assert_eq!(semantic.properties.role, SemanticRole::ProgressBar);
    assert_eq!(semantic.bounds, resized.node(PROGRESS).unwrap().bounds);
    assert_eq!(
        semantic.properties.value_range,
        Some(SemanticValueRange {
            minimum: 0.0,
            maximum: 100.0,
            value: 25.0,
        })
    );
    assert!(resized.hit_index.is_empty());

    let prepared = preflight_display_list(&resized.display_list).unwrap();
    assert!(matches!(
        prepared
            .records()
            .iter()
            .find(|record| record.id == progress_fill_id(PROGRESS))
            .unwrap()
            .primitive,
        SubmissionPrimitive::RoundedRect {
            color: FILL,
            radius: 6.0,
        }
    ));
}

#[test]
fn hidden_and_disabled_progress_preserve_participation_contracts() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.progress(
                HIDDEN,
                ProgressStyle::new(0.5, TRACK, FILL)
                    .layout(DeclarationStyle::new().height(8.0).hidden()),
            )
            .unwrap();
            ui.progress(
                DISABLED,
                ProgressStyle::new(0.5, TRACK, FILL)
                    .layout(DeclarationStyle::new().height(8.0).disabled()),
            )
            .unwrap();
        });
    })
    .unwrap();

    assert!(scene.display_list.iter().all(|record| record.id != HIDDEN));
    assert!(scene.semantics.node(HIDDEN).is_none());
    assert!(scene
        .display_list
        .iter()
        .any(|record| record.id == DISABLED));
    let semantic = scene.semantics.node(DISABLED).unwrap();
    assert_eq!(semantic.properties.role, SemanticRole::ProgressBar);
    assert!(semantic.properties.disabled);
    assert!(scene.hit_index.is_empty());
}

#[test]
fn invalid_progress_is_typed_and_failed_generation_replays_input() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));
    let observed = RefCell::new(Vec::new());

    let frame = |core: &mut FrameCore,
                 consumer: &mut NullSceneConsumer,
                 radius: f32|
     -> Result<(), FrameError> {
        run_declaration_frame(core, &measurer, consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let response = ui.button(
                    BUTTON,
                    "Retry",
                    ButtonStyle::default().layout(DeclarationStyle::new().size(40.0, 20.0)),
                );
                observed.borrow_mut().push(response.pressed);
                let _ = ui.progress(
                    INVALID,
                    ProgressStyle::new(0.5, TRACK, FILL)
                        .radius(radius)
                        .layout(DeclarationStyle::new().height(8.0)),
                );
            });
        })
        .map(|_| ())
    };

    frame(&mut core, &mut consumer, 4.0).unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 9, LogicalPoint::new(10.0, 10.0),));
    assert_eq!(
        frame(&mut core, &mut consumer, -1.0),
        Err(FrameError::InvalidProgress {
            id: INVALID,
            error: ProgressDeclarationError::InvalidRadius(-1.0),
        })
    );
    assert_eq!(consumer.scenes().len(), 1);

    frame(&mut core, &mut consumer, 4.0).unwrap();
    assert_eq!(*observed.borrow(), [false, true, true]);
    assert_eq!(consumer.scenes().len(), 2);
}

#[test]
fn invalid_ranges_and_values_are_rejected_without_clamping() {
    let cases = [
        (
            ProgressStyle::new(0.5, TRACK, FILL).range(1.0, 1.0, 1.0),
            ProgressDeclarationError::InvalidRange {
                minimum: 1.0,
                maximum: 1.0,
            },
        ),
        (
            ProgressStyle::new(0.5, TRACK, FILL).range(0.0, 1.0, 2.0),
            ProgressDeclarationError::ValueOutOfRange {
                minimum: 0.0,
                maximum: 1.0,
                value: 2.0,
            },
        ),
    ];

    for (style, expected) in cases {
        let measurer = DeterministicMeasurer::new(8.0, 18.0);
        let mut consumer = NullSceneConsumer::default();
        let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));
        let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let _ = ui.progress(INVALID, style);
            });
        })
        .unwrap_err();
        assert_eq!(
            error,
            FrameError::InvalidProgress {
                id: INVALID,
                error: expected,
            }
        );
        assert!(consumer.scenes().is_empty());
    }
}
