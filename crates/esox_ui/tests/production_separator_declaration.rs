use std::cell::{Cell, RefCell};

use esox_ui::declaration::{run_declaration_frame, ButtonStyle, DeclarationStyle, SeparatorStyle};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, PaintPrimitive, PointerEventKind, SemanticRole, SeparatorDeclarationError,
    WidgetId,
};
use esox_ui::scene_submission::preflight_display_list;

const ROOT: WidgetId = WidgetId(420_000);
const HORIZONTAL: WidgetId = WidgetId(420_001);
const ROW: WidgetId = WidgetId(420_002);
const VERTICAL: WidgetId = WidgetId(420_003);
const HIDDEN: WidgetId = WidgetId(420_004);
const DISABLED: WidgetId = WidgetId(420_005);
const BUTTON: WidgetId = WidgetId(420_006);
const INVALID: WidgetId = WidgetId(420_007);

const LINE: Color = Color::rgba(0.25, 0.5, 0.75, 1.0);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn separators_declare_once_and_use_current_frame_geometry() {
    let calls = Cell::new(0);
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(120.0, 80.0));

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        calls.set(calls.get() + 1);
        ui.column(ROOT, DeclarationStyle::new().padding(4.0).gap(6.0), |ui| {
            ui.separator(HORIZONTAL, SeparatorStyle::new(LINE).thickness(2.0))
                .unwrap();
            ui.row(ROW, DeclarationStyle::new().size(60.0, 30.0), |ui| {
                ui.separator(
                    VERTICAL,
                    SeparatorStyle::new(LINE).vertical().thickness(3.0),
                )
                .unwrap();
            });
        });
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert!(core.resize(LogicalSize::new(90.0, 80.0)));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert_eq!(calls.get(), 2);

    let first = &consumer.scenes()[0];
    let resized = &consumer.scenes()[1];
    assert_eq!(
        first.node(HORIZONTAL).unwrap().bounds,
        rect(4.0, 4.0, 112.0, 2.0)
    );
    assert_eq!(
        resized.node(HORIZONTAL).unwrap().bounds,
        rect(4.0, 4.0, 82.0, 2.0)
    );
    assert_eq!(
        resized.node(VERTICAL).unwrap().bounds,
        rect(4.0, 12.0, 3.0, 30.0)
    );

    for id in [HORIZONTAL, VERTICAL] {
        let paint = resized
            .display_list
            .iter()
            .find(|record| record.id == id)
            .unwrap();
        assert_eq!(paint.bounds, resized.node(id).unwrap().bounds);
        assert_eq!(paint.primitive, PaintPrimitive::SolidRect { color: LINE });
        let semantic = resized.semantics.node(id).unwrap();
        assert_eq!(semantic.bounds, resized.node(id).unwrap().bounds);
        assert_eq!(semantic.properties.role, SemanticRole::Separator);
        assert!(resized.damage.iter().any(|record| record.id == id));
    }
    assert!(resized.hit_index.is_empty());
    assert_eq!(
        preflight_display_list(&resized.display_list)
            .unwrap()
            .records()
            .len(),
        2
    );
}

#[test]
fn hidden_and_disabled_separators_preserve_participation_contracts() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.separator(
                HIDDEN,
                SeparatorStyle::new(LINE).layout(DeclarationStyle::new().hidden()),
            )
            .unwrap();
            ui.separator(
                DISABLED,
                SeparatorStyle::new(LINE).layout(DeclarationStyle::new().disabled()),
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
    assert_eq!(semantic.properties.role, SemanticRole::Separator);
    assert!(semantic.properties.disabled);
    assert!(scene.hit_index.is_empty());
}

#[test]
fn invalid_separator_generation_replays_input_deterministically() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));
    let observed = RefCell::new(Vec::new());

    let frame = |core: &mut FrameCore,
                 consumer: &mut NullSceneConsumer,
                 thickness: f32|
     -> Result<(), FrameError> {
        run_declaration_frame(core, &measurer, consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let response = ui.button(
                    BUTTON,
                    "Retry",
                    ButtonStyle::default().layout(DeclarationStyle::new().size(40.0, 20.0)),
                );
                observed.borrow_mut().push(response.pressed);
                let _ = ui.separator(INVALID, SeparatorStyle::new(LINE).thickness(thickness));
            });
        })
        .map(|_| ())
    };

    frame(&mut core, &mut consumer, 1.0).unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 9, LogicalPoint::new(10.0, 10.0),));
    assert_eq!(
        frame(&mut core, &mut consumer, 0.0),
        Err(FrameError::InvalidSeparator {
            id: INVALID,
            error: SeparatorDeclarationError::InvalidThickness(0.0),
        })
    );
    assert_eq!(consumer.scenes().len(), 1);

    frame(&mut core, &mut consumer, 3.0).unwrap();
    assert_eq!(*observed.borrow(), [false, true, true]);
    assert_eq!(consumer.scenes().len(), 2);
    assert_eq!(
        consumer.scenes()[1].node(INVALID).unwrap().bounds.height,
        3.0
    );
}
