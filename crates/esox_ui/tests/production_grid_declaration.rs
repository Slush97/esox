use std::cell::Cell;
use std::convert::Infallible;

use esox_gfx::{Color as GfxColor, Frame, ShapeBuilder};
use esox_ui::declaration::{
    run_declaration_frame, ButtonIds, ButtonStyle, CrossAxisAlignment, DeclarationStyle, GridStyle,
    GridTrack, MainAxisAlignment, TextStyle,
};
use esox_ui::frame_core::{
    Color, CommittedScene, DeterministicMeasurer, FrameCore, FrameError, GridDeclarationError,
    LogicalRect, LogicalSize, NullSceneConsumer, PaintPrimitive, SemanticRole, WidgetId,
};
use esox_ui::frame_scene_consumer::submit_display_list;
use esox_ui::scene_submission::{TextPaintBoundary, TextPaintRequest};

const ROOT: WidgetId = WidgetId(100);
const CONTENT: WidgetId = WidgetId(101);
const TITLE: WidgetId = WidgetId(102);
const GRID: WidgetId = WidgetId(103);
const FIXED: WidgetId = WidgetId(104);
const AUTO_COLUMN: WidgetId = WidgetId(105);
const AUTO_BORDER: WidgetId = WidgetId(106);
const AUTO_TEXT: WidgetId = WidgetId(107);
const FRACTION_ROW: WidgetId = WidgetId(108);
const FRACTION_TEXT: WidgetId = WidgetId(109);
const FRACTION_FILL: WidgetId = WidgetId(110);
const BUTTON: WidgetId = WidgetId(111);
const IMPLICIT_COLUMN: WidgetId = WidgetId(112);
const IMPLICIT_FILL: WidgetId = WidgetId(113);

const BLUE: Color = Color::rgba(0.1, 0.3, 0.8, 1.0);
const GREEN: Color = Color::rgba(0.2, 0.7, 0.4, 1.0);
const BORDER_COLOR: Color = Color::rgba(0.8, 0.4, 0.1, 1.0);
const TEXT_COLOR: Color = Color::rgba(0.9, 0.95, 1.0, 1.0);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Default)]
struct HeadlessText {
    requests: Vec<(WidgetId, LogicalRect, Option<LogicalRect>)>,
}

impl TextPaintBoundary<Frame> for HeadlessText {
    type Error = Infallible;

    fn paint_text(
        &mut self,
        frame: &mut Frame,
        request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error> {
        self.requests
            .push((request.id, request.bounds, request.effective_clip));
        frame.push(
            ShapeBuilder::rect(
                request.bounds.x,
                request.bounds.y,
                request.bounds.width,
                request.bounds.height,
            )
            .color(GfxColor::new(
                request.color.r,
                request.color.g,
                request.color.b,
                request.color.a,
            ))
            .build(),
        );
        Ok(())
    }
}

fn assert_scene_product_geometry(scene: &CommittedScene, id: WidgetId, bounds: LogicalRect) {
    let node = scene.node(id).expect("declared widget must resolve");
    assert_eq!(node.bounds, bounds, "node geometry for {id:?}");

    if let Some(paint) = scene.display_list.iter().find(|record| record.id == id) {
        assert_eq!(node.paint_bounds, Some(bounds));
        assert_eq!(paint.bounds, bounds);
        assert_eq!(paint.effective_clip, node.effective_clip);
        let damage = scene
            .damage
            .iter()
            .find(|record| record.id == id)
            .expect("painted widgets contribute damage");
        assert_eq!(node.current_damage_bounds, Some(bounds));
        assert_eq!(damage.current_bounds, bounds);
        assert_eq!(damage.effective_clip, node.effective_clip);
    }

    if let Some(hit) = scene.hit_index.iter().find(|record| record.id == id) {
        assert_eq!(node.hit_bounds, Some(bounds));
        assert_eq!(hit.bounds, bounds);
        assert_eq!(hit.effective_clip, node.effective_clip);
    }

    if let Some(semantic) = scene.semantics.node(id) {
        assert_eq!(node.semantic_bounds, Some(bounds));
        assert_eq!(semantic.bounds, bounds);
        assert_eq!(semantic.effective_clip, node.effective_clip);
    }
}

fn assert_resolved_geometry(
    scene: &CommittedScene,
    expected: &[(WidgetId, LogicalRect)],
    descendant_clip: LogicalRect,
) {
    assert_eq!(scene.nodes.len(), expected.len());
    for &(id, bounds) in expected {
        assert_scene_product_geometry(scene, id, bounds);
    }

    for id in [
        FIXED,
        AUTO_BORDER,
        AUTO_TEXT,
        FRACTION_TEXT,
        FRACTION_FILL,
        BUTTON,
        ButtonIds::new(BUTTON).fill,
        ButtonIds::new(BUTTON).label,
        IMPLICIT_FILL,
    ] {
        assert_eq!(
            scene.node(id).unwrap().effective_clip,
            Some(descendant_clip),
            "resolved clip for {id:?}"
        );
    }
}

#[test]
fn production_grid_alignment_is_current_once_and_renderer_ready() {
    let applications = Cell::new(0);
    let rows = Cell::new(0);
    let columns = Cell::new(0);
    let grids = Cell::new(0);
    let nested_rows = Cell::new(0);
    let nested_columns = Cell::new(0);
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(420.0, 220.0));

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        applications.set(applications.get() + 1);
        ui.row(
            ROOT,
            DeclarationStyle::new().main_axis_alignment(MainAxisAlignment::Center),
            |ui| {
                rows.set(rows.get() + 1);
                ui.column(
                    CONTENT,
                    DeclarationStyle::new()
                        .padding(10.0)
                        .gap(6.0)
                        .max_size(Some(360.0), None)
                        .flex_grow(1.0)
                        .clip_children(),
                    |ui| {
                        columns.set(columns.get() + 1);
                        ui.text(
                            TITLE,
                            "Current grid",
                            TextStyle::default().color(TEXT_COLOR),
                        );
                        ui.grid(
                            GRID,
                            GridStyle::new(vec![
                                GridTrack::Fixed(50.0),
                                GridTrack::Auto,
                                GridTrack::Fraction(1.0),
                                GridTrack::Fraction(2.0),
                            ])
                            .layout(
                                DeclarationStyle::new()
                                    .padding(4.0)
                                    .flex_grow(1.0)
                                    .cross_axis_alignment(CrossAxisAlignment::Center)
                                    .clip_children(),
                            )
                            .gaps(5.0, 7.0),
                            |ui| {
                                grids.set(grids.get() + 1);
                                ui.solid_rect(
                                    FIXED,
                                    DeclarationStyle::new()
                                        .size(45.0, 30.0)
                                        .min_size(Some(48.0), Some(30.0))
                                        .max_size(Some(48.0), Some(30.0)),
                                    BLUE,
                                );
                                ui.column(
                                    AUTO_COLUMN,
                                    DeclarationStyle::new()
                                        .size(32.0, 40.0)
                                        .main_axis_alignment(MainAxisAlignment::Center)
                                        .cross_axis_alignment(CrossAxisAlignment::Center),
                                    |ui| {
                                        nested_columns.set(nested_columns.get() + 1);
                                        ui.border(
                                            AUTO_BORDER,
                                            DeclarationStyle::new().size(24.0, 20.0),
                                            BORDER_COLOR,
                                            2.0,
                                        );
                                        ui.text(
                                            AUTO_TEXT,
                                            "A",
                                            TextStyle::default()
                                                .layout(DeclarationStyle::new().size(8.0, 18.0)),
                                        );
                                    },
                                );
                                ui.row(
                                    FRACTION_ROW,
                                    DeclarationStyle::new()
                                        .height(30.0)
                                        .gap(3.0)
                                        .main_axis_alignment(MainAxisAlignment::Center)
                                        .cross_axis_alignment(CrossAxisAlignment::Center),
                                    |ui| {
                                        nested_rows.set(nested_rows.get() + 1);
                                        ui.text(FRACTION_TEXT, "F", TextStyle::default());
                                        ui.solid_rect(
                                            FRACTION_FILL,
                                            DeclarationStyle::new()
                                                .size(10.0, 10.0)
                                                .flex_grow(1.0)
                                                .max_size(Some(20.0), Some(10.0)),
                                            GREEN,
                                        );
                                    },
                                );
                                let response = ui.button(
                                    BUTTON,
                                    "Go",
                                    ButtonStyle::default().layout(
                                        DeclarationStyle::new()
                                            .height(36.0)
                                            .min_size(Some(80.0), Some(36.0))
                                            .max_size(Some(120.0), Some(36.0)),
                                    ),
                                );
                                assert!(!response.clicked);
                                ui.column(
                                    IMPLICIT_COLUMN,
                                    DeclarationStyle::new()
                                        .size(50.0, 28.0)
                                        .main_axis_alignment(MainAxisAlignment::End)
                                        .cross_axis_alignment(CrossAxisAlignment::Center),
                                    |ui| {
                                        nested_columns.set(nested_columns.get() + 1);
                                        ui.solid_rect(
                                            IMPLICIT_FILL,
                                            DeclarationStyle::new().size(20.0, 12.0),
                                            GREEN,
                                        );
                                    },
                                );
                            },
                        );
                    },
                );
            },
        );
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    core.resize(LogicalSize::new(330.0, 220.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();

    assert_eq!(applications.get(), 2);
    assert_eq!(rows.get(), 2);
    assert_eq!(columns.get(), 2);
    assert_eq!(grids.get(), 2);
    assert_eq!(nested_rows.get(), 2);
    assert_eq!(nested_columns.get(), 4);

    let first = &consumer.scenes()[0];
    assert_eq!(first.generation, 1);
    assert_eq!(first.viewport, LogicalSize::new(420.0, 220.0));
    let button_ids = ButtonIds::new(BUTTON);
    assert_resolved_geometry(
        first,
        &[
            (ROOT, rect(0.0, 0.0, 420.0, 220.0)),
            (CONTENT, rect(30.0, 0.0, 360.0, 220.0)),
            (TITLE, rect(40.0, 10.0, 340.0, 18.0)),
            (GRID, rect(40.0, 34.0, 340.0, 176.0)),
            (FIXED, rect(44.0, 84.5, 48.0, 30.0)),
            (AUTO_COLUMN, rect(99.0, 84.5, 32.0, 40.0)),
            (AUTO_BORDER, rect(103.0, 85.5, 24.0, 20.0)),
            (AUTO_TEXT, rect(111.0, 105.5, 8.0, 18.0)),
            (FRACTION_ROW, rect(136.0, 84.5, 78.33334, 30.0)),
            (FRACTION_TEXT, rect(159.66667, 90.5, 8.0, 18.0)),
            (FRACTION_FILL, rect(170.66667, 94.5, 20.0, 10.0)),
            (BUTTON, rect(219.33334, 84.5, 120.0, 36.0)),
            (button_ids.fill, rect(219.33334, 84.5, 120.0, 36.0)),
            (button_ids.label, rect(227.33334, 92.5, 104.0, 18.0)),
            (IMPLICIT_COLUMN, rect(44.0, 131.5, 50.0, 28.0)),
            (IMPLICIT_FILL, rect(59.0, 147.5, 20.0, 12.0)),
        ],
        rect(40.0, 34.0, 340.0, 176.0),
    );

    let resized = &consumer.scenes()[1];
    assert_eq!(resized.generation, 2);
    assert_eq!(resized.viewport, LogicalSize::new(330.0, 220.0));
    assert_resolved_geometry(
        resized,
        &[
            (ROOT, rect(0.0, 0.0, 330.0, 220.0)),
            (CONTENT, rect(0.0, 0.0, 330.0, 220.0)),
            (TITLE, rect(10.0, 10.0, 310.0, 18.0)),
            (GRID, rect(10.0, 34.0, 310.0, 176.0)),
            (FIXED, rect(14.0, 84.5, 48.0, 30.0)),
            (AUTO_COLUMN, rect(69.0, 84.5, 32.0, 40.0)),
            (AUTO_BORDER, rect(73.0, 85.5, 24.0, 20.0)),
            (AUTO_TEXT, rect(81.0, 105.5, 8.0, 18.0)),
            (FRACTION_ROW, rect(106.0, 84.5, 68.33334, 30.0)),
            (FRACTION_TEXT, rect(124.66667, 90.5, 8.0, 18.0)),
            (FRACTION_FILL, rect(135.66667, 94.5, 20.0, 10.0)),
            (BUTTON, rect(179.33334, 84.5, 120.0, 36.0)),
            (button_ids.fill, rect(179.33334, 84.5, 120.0, 36.0)),
            (button_ids.label, rect(187.33334, 92.5, 104.0, 18.0)),
            (IMPLICIT_COLUMN, rect(14.0, 131.5, 50.0, 28.0)),
            (IMPLICIT_FILL, rect(29.0, 147.5, 20.0, 12.0)),
        ],
        rect(10.0, 34.0, 310.0, 176.0),
    );

    // Grid padding, fixed/auto tracks, and three gaps leave 235 then 205
    // logical pixels. The fractional tracks divide each remainder 1:2.
    let first_one_fr = first.node(FRACTION_ROW).unwrap().bounds.width;
    let first_two_fr = 376.0 - first.node(BUTTON).unwrap().bounds.x;
    assert_eq!((first_one_fr, first_two_fr), (78.33334, 156.66666));
    assert_eq!(first_one_fr + first_two_fr, 235.0);
    let resized_one_fr = resized.node(FRACTION_ROW).unwrap().bounds.width;
    let resized_two_fr = 316.0 - resized.node(BUTTON).unwrap().bounds.x;
    assert_eq!((resized_one_fr, resized_two_fr), (68.33334, 136.66666));
    assert_eq!(resized_one_fr + resized_two_fr, 205.0);

    let paint_order = resized
        .display_list
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    assert_eq!(
        paint_order,
        vec![
            TITLE,
            FIXED,
            AUTO_BORDER,
            AUTO_TEXT,
            FRACTION_TEXT,
            FRACTION_FILL,
            BUTTON,
            button_ids.fill,
            button_ids.label,
            IMPLICIT_FILL,
        ]
    );
    assert_eq!(resized.focus_order, vec![BUTTON]);
    assert_eq!(resized.hit_index.len(), 1);
    assert_eq!(resized.hit_index[0].id, BUTTON);
    assert_eq!(
        resized.hit_index[0].bounds,
        resized.node(BUTTON).unwrap().bounds
    );
    assert!(resized.focus_scopes.is_empty());
    assert_eq!(
        resized.semantics.node(BUTTON).unwrap().properties.role,
        SemanticRole::Button
    );
    assert_eq!(
        resized
            .semantics
            .node(BUTTON)
            .unwrap()
            .properties
            .label
            .as_deref(),
        Some("Go")
    );
    assert_eq!(
        resized
            .semantics
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![TITLE, AUTO_TEXT, FRACTION_TEXT, BUTTON, button_ids.label]
    );
    assert_eq!(
        resized
            .damage
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        paint_order
    );
    assert_eq!(
        resized
            .display_list
            .iter()
            .find(|record| record.id == FIXED)
            .unwrap()
            .primitive,
        PaintPrimitive::SolidRect { color: BLUE }
    );
    assert_eq!(
        resized
            .display_list
            .iter()
            .find(|record| record.id == AUTO_BORDER)
            .unwrap()
            .primitive,
        PaintPrimitive::Border {
            color: BORDER_COLOR,
            width: 2.0,
        }
    );
    assert!(matches!(
        &resized
            .display_list
            .iter()
            .find(|record| record.id == TITLE)
            .unwrap()
            .primitive,
        PaintPrimitive::Text { content, color: TEXT_COLOR, .. } if content == "Current grid"
    ));

    let before_submission = resized.clone();
    let mut frame = Frame::new();
    let mut text = HeadlessText::default();
    submit_display_list(&resized.display_list, &mut frame, &mut text).unwrap();
    assert_eq!(resized, &before_submission);
    assert_eq!(frame.instance_data().len(), resized.display_list.len());
    assert_eq!(
        frame
            .instance_data()
            .iter()
            .map(|instance| instance.rect)
            .collect::<Vec<_>>(),
        resized
            .display_list
            .iter()
            .map(|record| [
                record.bounds.x,
                record.bounds.y,
                record.bounds.width,
                record.bounds.height,
            ])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        frame
            .instance_data()
            .iter()
            .map(|instance| instance.clip_rect)
            .collect::<Vec<_>>(),
        resized
            .display_list
            .iter()
            .map(|record| record.effective_clip.map_or([0.0; 4], |clip| [
                clip.x,
                clip.y,
                clip.width,
                clip.height,
            ]))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        text.requests
            .iter()
            .map(|request| request.0)
            .collect::<Vec<_>>(),
        vec![TITLE, AUTO_TEXT, FRACTION_TEXT, button_ids.label]
    );
}

#[test]
fn invalid_production_grid_returns_exact_typed_error_without_commit() {
    const INVALID: WidgetId = WidgetId(900);
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.solid_rect(ROOT, DeclarationStyle::new().size(100.0, 60.0), BLUE);
    })
    .unwrap();
    let committed = core.committed_scene().unwrap().clone();

    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.grid(
            INVALID,
            GridStyle::new(vec![GridTrack::Fraction(0.0)]).gaps(5.0, 7.0),
            |_| {},
        );
    })
    .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidGrid {
            id: INVALID,
            error: GridDeclarationError::InvalidFractionTrack {
                index: 0,
                value: 0.0,
            },
        }
    );
    assert_eq!(core.committed_scene(), Some(&committed));
    assert_eq!(consumer.scenes(), &[committed]);
}
