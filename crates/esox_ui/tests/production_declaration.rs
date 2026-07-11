use std::cell::Cell;

use esox_ui::declaration::{
    run_declaration_frame, ButtonIds, ButtonStyle, DeclarationStyle, TextStyle,
};
use esox_ui::frame_core::{
    Color, CommittedScene, DeterministicMeasurer, LogicalRect, LogicalSize, NullSceneConsumer,
    PaintPrimitive, SemanticRole, WidgetId,
};

const ROOT: WidgetId = WidgetId(1);
const TITLE: WidgetId = WidgetId(2);
const CONTENT_ROW: WidgetId = WidgetId(3);
const SWATCH: WidgetId = WidgetId(4);
const BUTTON: WidgetId = WidgetId(5);

const ACCENT: Color = Color::rgba(0.2, 0.4, 0.8, 1.0);
const BORDER: Color = Color::rgba(0.1, 0.2, 0.3, 1.0);
const TEXT: Color = Color::rgba(0.9, 0.95, 1.0, 1.0);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn assert_button_products_agree(scene: &CommittedScene, bounds: LogicalRect) {
    let clip = Some(rect(0.0, 0.0, scene.viewport.width, scene.viewport.height));
    let node = scene.node(BUTTON).expect("button must resolve");
    assert_eq!(node.bounds, bounds);
    assert_eq!(node.paint_bounds, Some(bounds));
    assert_eq!(node.hit_bounds, Some(bounds));
    assert_eq!(node.semantic_bounds, Some(bounds));
    assert_eq!(node.current_damage_bounds, Some(bounds));
    assert_eq!(node.effective_clip, clip);

    let paint = scene
        .display_list
        .iter()
        .find(|record| record.id == BUTTON)
        .expect("button border must paint");
    assert_eq!(paint.bounds, bounds);
    assert_eq!(paint.effective_clip, clip);
    assert_eq!(
        paint.primitive,
        PaintPrimitive::Border {
            color: BORDER,
            width: 2.0,
        }
    );

    let hit = scene
        .hit_index
        .iter()
        .find(|record| record.id == BUTTON)
        .expect("button must be hit-testable");
    assert_eq!(hit.bounds, bounds);
    assert_eq!(hit.effective_clip, clip);

    let semantic = scene
        .semantics
        .node(BUTTON)
        .expect("button semantics must be committed");
    assert_eq!(semantic.bounds, bounds);
    assert_eq!(semantic.effective_clip, clip);
    assert_eq!(semantic.properties.role, SemanticRole::Button);
    assert_eq!(semantic.properties.label.as_deref(), Some("Resize me"));

    let damage = scene
        .damage
        .iter()
        .find(|record| record.id == BUTTON)
        .expect("button must contribute current damage");
    assert_eq!(damage.current_bounds, bounds);
    assert_eq!(damage.effective_clip, clip);
}

fn assert_painted_semantic_leaf_agrees(scene: &CommittedScene, id: WidgetId, bounds: LogicalRect) {
    let node = scene.node(id).expect("leaf must resolve");
    let paint = scene
        .display_list
        .iter()
        .find(|record| record.id == id)
        .expect("leaf must paint");
    let semantic = scene
        .semantics
        .node(id)
        .expect("text leaf must have semantics");
    let damage = scene
        .damage
        .iter()
        .find(|record| record.id == id)
        .expect("leaf must contribute damage");

    assert_eq!(node.bounds, bounds);
    assert_eq!(node.paint_bounds, Some(bounds));
    assert_eq!(node.semantic_bounds, Some(bounds));
    assert_eq!(node.current_damage_bounds, Some(bounds));
    assert_eq!(paint.bounds, bounds);
    assert_eq!(semantic.bounds, bounds);
    assert_eq!(damage.current_bounds, bounds);
    assert_eq!(paint.effective_clip, node.effective_clip);
    assert_eq!(semantic.effective_clip, node.effective_clip);
    assert_eq!(damage.effective_clip, node.effective_clip);
}

fn assert_painted_leaf_agrees(scene: &CommittedScene, id: WidgetId, bounds: LogicalRect) {
    let node = scene.node(id).expect("painted leaf must resolve");
    let paint = scene
        .display_list
        .iter()
        .find(|record| record.id == id)
        .expect("leaf must paint");
    let damage = scene
        .damage
        .iter()
        .find(|record| record.id == id)
        .expect("leaf must contribute damage");

    assert_eq!(node.bounds, bounds);
    assert_eq!(node.paint_bounds, Some(bounds));
    assert_eq!(node.current_damage_bounds, Some(bounds));
    assert_eq!(paint.bounds, bounds);
    assert_eq!(damage.current_bounds, bounds);
    assert_eq!(paint.effective_clip, node.effective_clip);
    assert_eq!(damage.effective_clip, node.effective_clip);
}

#[test]
fn production_declaration_is_once_and_current_on_first_frame_and_resize() {
    let applications = Cell::new(0);
    let columns = Cell::new(0);
    let rows = Cell::new(0);
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = esox_ui::frame_core::FrameCore::new(LogicalSize::new(320.0, 140.0));

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        applications.set(applications.get() + 1);
        ui.column(
            ROOT,
            DeclarationStyle::new()
                .padding(10.0)
                .gap(6.0)
                .clip_children(),
            |ui| {
                columns.set(columns.get() + 1);
                ui.text(
                    TITLE,
                    "Production declaration",
                    TextStyle::default().color(TEXT),
                );
                ui.row(
                    CONTENT_ROW,
                    DeclarationStyle::new().padding(4.0).gap(8.0).flex_grow(1.0),
                    |ui| {
                        rows.set(rows.get() + 1);
                        ui.solid_rect(
                            SWATCH,
                            DeclarationStyle::new()
                                .size(60.0, 40.0)
                                .min_size(Some(70.0), Some(40.0))
                                .max_size(Some(80.0), Some(40.0)),
                            ACCENT,
                        );
                        let response = ui.button(
                            BUTTON,
                            "Resize me",
                            ButtonStyle::default()
                                .layout(
                                    DeclarationStyle::new()
                                        .height(40.0)
                                        .min_size(Some(100.0), Some(40.0))
                                        .max_size(Some(180.0), Some(40.0))
                                        .flex_grow(1.0),
                                )
                                .fill(ACCENT)
                                .border(BORDER, 2.0)
                                .text(TextStyle::default().color(TEXT)),
                        );
                        assert!(!response.clicked);
                        assert!(!response.pressed);
                    },
                );
            },
        );
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert_eq!((applications.get(), columns.get(), rows.get()), (1, 1, 1));

    core.resize(LogicalSize::new(220.0, 140.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    assert_eq!((applications.get(), columns.get(), rows.get()), (2, 2, 2));

    let ids = ButtonIds::new(BUTTON);
    let first = &consumer.scenes()[0];
    assert_eq!(first.generation, 1);
    assert_eq!(first.viewport, LogicalSize::new(320.0, 140.0));
    assert_eq!(
        first.node(ROOT).unwrap().bounds,
        rect(0.0, 0.0, 320.0, 140.0)
    );
    assert_painted_semantic_leaf_agrees(first, TITLE, rect(10.0, 10.0, 300.0, 18.0));
    assert_eq!(
        first.node(CONTENT_ROW).unwrap().bounds,
        rect(10.0, 34.0, 300.0, 96.0)
    );
    assert_painted_leaf_agrees(first, SWATCH, rect(14.0, 38.0, 70.0, 40.0));
    assert_button_products_agree(first, rect(92.0, 38.0, 180.0, 40.0));
    assert_painted_leaf_agrees(first, ids.fill, rect(92.0, 38.0, 180.0, 40.0));
    assert_painted_semantic_leaf_agrees(first, ids.label, rect(100.0, 46.0, 164.0, 18.0));

    let resized = &consumer.scenes()[1];
    assert_eq!(resized.generation, 2);
    assert_eq!(resized.viewport, LogicalSize::new(220.0, 140.0));
    assert_eq!(
        resized.node(ROOT).unwrap().bounds,
        rect(0.0, 0.0, 220.0, 140.0)
    );
    assert_painted_semantic_leaf_agrees(resized, TITLE, rect(10.0, 10.0, 200.0, 18.0));
    assert_eq!(
        resized.node(CONTENT_ROW).unwrap().bounds,
        rect(10.0, 34.0, 200.0, 96.0)
    );
    assert_painted_leaf_agrees(resized, SWATCH, rect(14.0, 38.0, 70.0, 40.0));
    assert_button_products_agree(resized, rect(92.0, 38.0, 114.0, 40.0));
    assert_painted_leaf_agrees(resized, ids.fill, rect(92.0, 38.0, 114.0, 40.0));
    assert_painted_semantic_leaf_agrees(resized, ids.label, rect(100.0, 46.0, 98.0, 18.0));

    assert_eq!(
        resized
            .display_list
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![TITLE, SWATCH, BUTTON, ids.fill, ids.label]
    );
    assert_eq!(
        resized
            .hit_index
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![BUTTON]
    );

    assert_eq!(
        resized
            .display_list
            .iter()
            .find(|record| record.id == SWATCH)
            .unwrap()
            .primitive,
        PaintPrimitive::SolidRect { color: ACCENT }
    );
    assert_eq!(
        resized
            .display_list
            .iter()
            .find(|record| record.id == ids.fill)
            .unwrap()
            .primitive,
        PaintPrimitive::SolidRect { color: ACCENT }
    );
    for (id, content) in [(TITLE, "Production declaration"), (ids.label, "Resize me")] {
        let primitive = &resized
            .display_list
            .iter()
            .find(|record| record.id == id)
            .unwrap()
            .primitive;
        assert!(matches!(
            primitive,
            PaintPrimitive::Text {
                content: actual,
                color: TEXT,
                ..
            } if actual == content
        ));
    }
}
