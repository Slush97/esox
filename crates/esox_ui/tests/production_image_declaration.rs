use std::cell::{Cell, RefCell};

use esox_ui::declaration::{run_declaration_frame, DeclarationStyle, ImageStyle};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, PaintPrimitive, PointerEventKind, SemanticRole, WidgetId,
};

const ROOT: WidgetId = WidgetId(410_000);
const IMAGE: WidgetId = WidgetId(410_001);
const DUPLICATE: WidgetId = WidgetId(410_002);
const RESOURCE: u64 = 77;

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn image_is_declared_once_and_uses_current_intrinsic_constraints() {
    let calls = Cell::new(0);
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(RESOURCE, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        calls.set(calls.get() + 1);
        ui.column(ROOT, DeclarationStyle::new().padding(10.0), |ui| {
            let response = ui.image(
                IMAGE,
                RESOURCE,
                ImageStyle::default().label("Project artwork"),
            );
            assert!(!response.clicked);
        });
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert!(core.resize(LogicalSize::new(70.0, 60.0)));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert_eq!(calls.get(), 2);

    let first = &consumer.scenes()[0];
    let resized = &consumer.scenes()[1];
    assert_eq!(
        first.node(IMAGE).unwrap().bounds,
        rect(10.0, 10.0, 80.0, 24.0)
    );
    assert_eq!(
        resized.node(IMAGE).unwrap().bounds,
        rect(10.0, 10.0, 50.0, 24.0)
    );
    assert_eq!(
        resized
            .display_list
            .iter()
            .find(|record| record.id == IMAGE)
            .unwrap()
            .primitive,
        PaintPrimitive::Image { resource: RESOURCE }
    );
    let semantic = resized.semantics.node(IMAGE).unwrap();
    assert_eq!(semantic.bounds, rect(10.0, 10.0, 50.0, 24.0));
    assert_eq!(semantic.properties.role, SemanticRole::Image);
    assert_eq!(
        semantic.properties.label.as_deref(),
        Some("Project artwork")
    );
    assert_eq!(resized.hit_index[0].id, IMAGE);
}

#[test]
fn failed_generation_replays_image_pointer_input() {
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(RESOURCE, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));
    let observed = RefCell::new(Vec::new());

    let frame = |core: &mut FrameCore,
                 consumer: &mut NullSceneConsumer,
                 duplicate: bool|
     -> Result<(), FrameError> {
        run_declaration_frame(core, &measurer, consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let response = ui.image(
                    IMAGE,
                    RESOURCE,
                    ImageStyle::default().layout(DeclarationStyle::new().size(40.0, 24.0)),
                );
                observed
                    .borrow_mut()
                    .push((response.clicked, response.pressed));
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
        })
        .map(|_| ())
    };

    frame(&mut core, &mut consumer, false).unwrap();
    assert!(core.queue_pointer_event(PointerEventKind::Press, 9, LogicalPoint::new(10.0, 10.0),));
    assert_eq!(
        frame(&mut core, &mut consumer, true),
        Err(FrameError::DuplicateWidgetId(DUPLICATE))
    );
    frame(&mut core, &mut consumer, false).unwrap();
    assert_eq!(
        *observed.borrow(),
        [(false, false), (false, true), (false, true)]
    );

    assert!(core.queue_pointer_event(PointerEventKind::Release, 9, LogicalPoint::new(10.0, 10.0),));
    frame(&mut core, &mut consumer, false).unwrap();
    assert_eq!(observed.borrow().last().copied(), Some((true, false)));
}

#[test]
fn disabled_image_stays_semantic_but_drops_interaction() {
    let measurer =
        DeterministicMeasurer::new(8.0, 18.0).with_image(RESOURCE, LogicalSize::new(40.0, 24.0));
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(80.0, 40.0));
    let response = RefCell::new(None);

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            response.replace(Some(ui.image(
                IMAGE,
                RESOURCE,
                ImageStyle::default().label("Unavailable").disabled(),
            )));
        });
    })
    .unwrap();

    assert!(response.borrow().as_ref().unwrap().disabled);
    assert!(scene.hit_index.is_empty());
    let semantic = scene.semantics.node(IMAGE).unwrap();
    assert!(semantic.properties.disabled);
    assert_eq!(semantic.properties.role, SemanticRole::Image);
}
