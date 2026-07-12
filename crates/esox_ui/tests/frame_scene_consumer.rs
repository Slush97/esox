use std::convert::Infallible;

use esox_gfx::{Color as GfxColor, Frame, ShapeBuilder};
use esox_ui::frame_core::{
    Color, CommittedScene, DamageRecord, FocusScope, HitRecord, LogicalRect, LogicalSize,
    PaintPrimitive, PaintRecord, ResolvedNode, SemanticNode, SemanticProperties, SemanticRole,
    SemanticSnapshot, TextDirection, TextProperties, WidgetId,
};
use esox_ui::frame_scene_consumer::{
    submit_display_list, submit_display_list_scaled, FrameSceneSubmissionError, RendererScale,
};
use esox_ui::scene_submission::{
    SceneSubmissionError, SubmissionTextWeight, TextPaintBoundary, TextPaintRequest,
    UnsupportedPaintPrimitive, UnsupportedTextCapability,
};

const SOLID: WidgetId = WidgetId(11);
const REGULAR_TEXT: WidgetId = WidgetId(12);
const BORDER: WidgetId = WidgetId(13);
const BOLD_TEXT: WidgetId = WidgetId(14);

const SOLID_COLOR: Color = Color::rgba(0.125, 0.25, 0.5, 0.75);
const REGULAR_TEXT_COLOR: Color = Color::rgba(0.2, 0.4, 0.6, 0.8);
const BORDER_COLOR: Color = Color::rgba(0.9, 0.7, 0.5, 0.3);
const BOLD_TEXT_COLOR: Color = Color::rgba(0.8, 0.6, 0.4, 0.2);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn paint_record(
    id: WidgetId,
    primitive: PaintPrimitive,
    bounds: LogicalRect,
    effective_clip: Option<LogicalRect>,
) -> PaintRecord {
    PaintRecord {
        id,
        primitive,
        bounds,
        effective_clip,
    }
}

fn text_properties(font_size: f32, font_weight: u16) -> TextProperties {
    TextProperties {
        font_size,
        font_weight,
        ..TextProperties::default()
    }
}

fn representative_scene() -> CommittedScene {
    let solid_bounds = rect(1.25, 2.5, 30.75, 40.125);
    let solid_clip = rect(0.5, 1.5, 25.25, 35.75);
    let regular_text_bounds = rect(41.5, 3.25, 70.875, 18.625);
    let regular_text_clip = rect(40.0, 2.0, 65.5, 17.25);
    let border_bounds = rect(5.75, 50.125, 80.25, 35.5);
    let bold_text_bounds = rect(12.375, 92.25, 91.625, 22.875);
    let bold_text_clip = rect(10.25, 90.5, 75.75, 20.25);

    let display_list = vec![
        paint_record(
            SOLID,
            PaintPrimitive::SolidRect { color: SOLID_COLOR },
            solid_bounds,
            Some(solid_clip),
        ),
        paint_record(
            REGULAR_TEXT,
            PaintPrimitive::Text {
                content: "first styled run".to_owned(),
                properties: text_properties(17.25, 400),
                color: REGULAR_TEXT_COLOR,
            },
            regular_text_bounds,
            Some(regular_text_clip),
        ),
        paint_record(
            BORDER,
            PaintPrimitive::Border {
                color: BORDER_COLOR,
                width: 2.75,
            },
            border_bounds,
            None,
        ),
        paint_record(
            BOLD_TEXT,
            PaintPrimitive::Text {
                content: "second styled run".to_owned(),
                properties: text_properties(23.5, 700),
                color: BOLD_TEXT_COLOR,
            },
            bold_text_bounds,
            Some(bold_text_clip),
        ),
    ];

    CommittedScene {
        generation: 37,
        viewport: LogicalSize::new(640.5, 480.25),
        nodes: vec![ResolvedNode {
            id: SOLID,
            parent: None,
            bounds: rect(301.0, 302.0, 303.0, 304.0),
            paint_bounds: Some(solid_bounds),
            hit_bounds: Some(rect(311.0, 312.0, 313.0, 314.0)),
            semantic_bounds: Some(rect(321.0, 322.0, 323.0, 324.0)),
            effective_clip: Some(solid_clip),
            current_damage_bounds: Some(rect(331.0, 332.0, 333.0, 334.0)),
            scroll_metrics: None,
            focus_scope: Some(BORDER),
            blocks_input: true,
            effective_hidden: false,
            effective_disabled: false,
        }],
        display_list,
        hit_index: vec![HitRecord {
            id: BORDER,
            bounds: rect(401.0, 402.0, 403.0, 404.0),
            effective_clip: Some(rect(411.0, 412.0, 413.0, 414.0)),
            focus_scope: Some(BOLD_TEXT),
            blocks_input: true,
        }],
        semantics: SemanticSnapshot {
            roots: vec![REGULAR_TEXT],
            nodes: vec![SemanticNode {
                id: REGULAR_TEXT,
                parent: None,
                children: vec![BOLD_TEXT],
                properties: SemanticProperties::new(SemanticRole::Text)
                    .with_label("sentinel semantics"),
                bounds: rect(501.0, 502.0, 503.0, 504.0),
                effective_clip: Some(rect(511.0, 512.0, 513.0, 514.0)),
                focus_scope: Some(BORDER),
            }],
        },
        damage: vec![DamageRecord {
            id: BOLD_TEXT,
            current_bounds: rect(601.0, 602.0, 603.0, 604.0),
            effective_clip: Some(rect(611.0, 612.0, 613.0, 614.0)),
        }],
        focus_order: vec![BORDER, REGULAR_TEXT],
        focus_scopes: vec![FocusScope {
            owner: BORDER,
            members: vec![REGULAR_TEXT, BOLD_TEXT],
        }],
    }
}

#[derive(Debug, PartialEq)]
struct ObservedText {
    id: WidgetId,
    content: String,
    bounds: LogicalRect,
    effective_clip: Option<LogicalRect>,
    color: Color,
    font_size: f32,
    font_weight: SubmissionTextWeight,
}

#[derive(Default)]
struct FakeTextPaint {
    requests: Vec<ObservedText>,
}

impl TextPaintBoundary<Frame> for FakeTextPaint {
    type Error = Infallible;

    fn paint_text(
        &mut self,
        frame: &mut Frame,
        request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error> {
        self.requests.push(ObservedText {
            id: request.id,
            content: request.content.to_owned(),
            bounds: request.bounds,
            effective_clip: request.effective_clip,
            color: request.color,
            font_size: request.font_size,
            font_weight: request.font_weight,
        });

        // A marker instance makes text's position in the logical display order
        // observable without a GPU, rasterizer, or atlas.
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

#[test]
fn committed_display_list_reaches_frame_in_order_without_recomputing_scene_products() {
    let scene = representative_scene();
    let scene_before_submission = scene.clone();
    let previous_clip = [901.25, 902.5, 903.75, 904.125];
    let mut frame = Frame::new();
    frame.set_active_clip(Some(previous_clip));
    let mut text = FakeTextPaint::default();

    submit_display_list(&scene.display_list, &mut frame, &mut text).unwrap();

    assert_eq!(frame.active_clip(), Some(previous_clip));
    assert_eq!(scene, scene_before_submission);

    let instances = frame.instance_data();
    assert_eq!(instances.len(), 4);
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.rect)
            .collect::<Vec<_>>(),
        vec![
            [1.25, 2.5, 30.75, 40.125],
            [41.5, 3.25, 70.875, 18.625],
            [5.75, 50.125, 80.25, 35.5],
            [12.375, 92.25, 91.625, 22.875],
        ]
    );
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.color)
            .collect::<Vec<_>>(),
        vec![
            [0.125, 0.25, 0.5, 0.75],
            [0.2, 0.4, 0.6, 0.8],
            [0.9, 0.7, 0.5, 0.3],
            [0.8, 0.6, 0.4, 0.2],
        ]
    );
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.flags[1])
            .collect::<Vec<_>>(),
        vec![0.0, 0.0, 2.75, 0.0]
    );
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.clip_rect)
            .collect::<Vec<_>>(),
        vec![
            [0.5, 1.5, 25.25, 35.75],
            [40.0, 2.0, 65.5, 17.25],
            [0.0; 4],
            [10.25, 90.5, 75.75, 20.25],
        ]
    );

    assert_eq!(
        text.requests,
        vec![
            ObservedText {
                id: REGULAR_TEXT,
                content: "first styled run".to_owned(),
                bounds: rect(41.5, 3.25, 70.875, 18.625),
                effective_clip: Some(rect(40.0, 2.0, 65.5, 17.25)),
                color: REGULAR_TEXT_COLOR,
                font_size: 17.25,
                font_weight: SubmissionTextWeight::Regular,
            },
            ObservedText {
                id: BOLD_TEXT,
                content: "second styled run".to_owned(),
                bounds: rect(12.375, 92.25, 91.625, 22.875),
                effective_clip: Some(rect(10.25, 90.5, 75.75, 20.25)),
                color: BOLD_TEXT_COLOR,
                font_size: 23.5,
                font_weight: SubmissionTextWeight::Bold,
            },
        ]
    );
}

#[test]
fn renderer_scale_converts_logical_geometry_exactly_once() {
    let scene = representative_scene();
    let scene_before_submission = scene.clone();
    let mut frame = Frame::new();
    let mut text = FakeTextPaint::default();

    submit_display_list_scaled(
        &scene.display_list,
        &mut frame,
        &mut text,
        RendererScale::new(2.0).unwrap(),
    )
    .unwrap();

    assert_eq!(scene, scene_before_submission);
    assert_eq!(frame.instance_data()[0].rect, [2.5, 5.0, 61.5, 80.25]);
    assert_eq!(frame.instance_data()[0].clip_rect, [1.0, 3.0, 50.5, 71.5]);
    assert_eq!(frame.instance_data()[2].flags[1], 5.5);
    assert_eq!(text.requests[0].bounds, rect(83.0, 6.5, 141.75, 37.25));
    assert_eq!(text.requests[0].font_size, 34.5);
}

#[test]
fn invalid_renderer_transform_is_rejected_before_target_mutation() {
    assert!(RendererScale::new(0.0).is_none());
    assert!(RendererScale::new(f64::NAN).is_none());
    assert!(RendererScale::new(f64::MAX).is_none());

    let display_list = vec![paint_record(
        WidgetId(94),
        PaintPrimitive::SolidRect {
            color: Color::BLACK,
        },
        rect(f32::MAX, 0.0, 1.0, 1.0),
        None,
    )];
    let mut frame = Frame::new();
    let previous_clip = [1.0, 2.0, 3.0, 4.0];
    frame.set_active_clip(Some(previous_clip));
    let mut text = FakeTextPaint::default();

    assert_eq!(
        submit_display_list_scaled(
            &display_list,
            &mut frame,
            &mut text,
            RendererScale::new(2.0).unwrap(),
        ),
        Err(FrameSceneSubmissionError::InvalidTransform)
    );
    assert_eq!(frame.instance_len(), 0);
    assert_eq!(frame.active_clip(), Some(previous_clip));
    assert!(text.requests.is_empty());
}

#[test]
fn explicit_empty_clip_survives_the_frame_no_clip_sentinel() {
    let display_list = vec![paint_record(
        WidgetId(93),
        PaintPrimitive::SolidRect {
            color: Color::BLACK,
        },
        rect(0.0, 0.0, 20.0, 20.0),
        Some(rect(0.0, 0.0, 0.0, 0.0)),
    )];
    let mut frame = Frame::new();
    let mut text = FakeTextPaint::default();

    submit_display_list(&display_list, &mut frame, &mut text).unwrap();

    assert_eq!(frame.instance_data().len(), 1);
    assert_eq!(
        frame.instance_data()[0].clip_rect,
        [0.0, 0.0, 0.0, f32::EPSILON]
    );
}

#[derive(Clone)]
struct UnsupportedCase {
    primitive: PaintPrimitive,
    expected: SceneSubmissionError,
}

#[test]
fn unsupported_input_is_rejected_before_frame_or_text_mutation() {
    let unsupported_id = WidgetId(92);
    let cases = [
        UnsupportedCase {
            primitive: PaintPrimitive::Box,
            expected: SceneSubmissionError::UnsupportedPrimitive {
                record_index: 2,
                id: unsupported_id,
                primitive: UnsupportedPaintPrimitive::Box,
            },
        },
        UnsupportedCase {
            primitive: PaintPrimitive::Image { resource: 77 },
            expected: SceneSubmissionError::UnsupportedPrimitive {
                record_index: 2,
                id: unsupported_id,
                primitive: UnsupportedPaintPrimitive::Image,
            },
        },
        UnsupportedCase {
            primitive: PaintPrimitive::Text {
                content: "family".to_owned(),
                properties: TextProperties {
                    font_family: Some("Required Family".to_owned()),
                    ..TextProperties::default()
                },
                color: Color::BLACK,
            },
            expected: SceneSubmissionError::UnsupportedText {
                record_index: 2,
                id: unsupported_id,
                capability: UnsupportedTextCapability::FontFamily("Required Family".to_owned()),
            },
        },
        UnsupportedCase {
            primitive: PaintPrimitive::Text {
                content: "locale".to_owned(),
                properties: TextProperties {
                    locale: Some("ja-JP".to_owned()),
                    ..TextProperties::default()
                },
                color: Color::BLACK,
            },
            expected: SceneSubmissionError::UnsupportedText {
                record_index: 2,
                id: unsupported_id,
                capability: UnsupportedTextCapability::Locale("ja-JP".to_owned()),
            },
        },
        UnsupportedCase {
            primitive: PaintPrimitive::Text {
                content: "direction".to_owned(),
                properties: TextProperties {
                    direction: TextDirection::RightToLeft,
                    ..TextProperties::default()
                },
                color: Color::BLACK,
            },
            expected: SceneSubmissionError::UnsupportedText {
                record_index: 2,
                id: unsupported_id,
                capability: UnsupportedTextCapability::Direction(TextDirection::RightToLeft),
            },
        },
        UnsupportedCase {
            primitive: PaintPrimitive::Text {
                content: "weight".to_owned(),
                properties: TextProperties {
                    font_weight: 500,
                    ..TextProperties::default()
                },
                color: Color::BLACK,
            },
            expected: SceneSubmissionError::UnsupportedText {
                record_index: 2,
                id: unsupported_id,
                capability: UnsupportedTextCapability::FontWeight(500),
            },
        },
    ];

    for case in cases {
        let display_list = vec![
            paint_record(
                WidgetId(90),
                PaintPrimitive::SolidRect { color: SOLID_COLOR },
                rect(1.0, 2.0, 3.0, 4.0),
                Some(rect(5.0, 6.0, 7.0, 8.0)),
            ),
            paint_record(
                WidgetId(91),
                PaintPrimitive::Text {
                    content: "supported before error".to_owned(),
                    properties: TextProperties::default(),
                    color: REGULAR_TEXT_COLOR,
                },
                rect(9.0, 10.0, 11.0, 12.0),
                None,
            ),
            paint_record(
                unsupported_id,
                case.primitive,
                rect(13.0, 14.0, 15.0, 16.0),
                Some(rect(17.0, 18.0, 19.0, 20.0)),
            ),
        ];

        let mut frame = Frame::new();
        let original_clip = [21.0, 22.0, 23.0, 24.0];
        frame.set_active_clip(Some(original_clip));
        frame.push(
            ShapeBuilder::rect(31.0, 32.0, 33.0, 34.0)
                .color(GfxColor::new(0.11, 0.22, 0.33, 0.44))
                .stroke(4.5)
                .build(),
        );
        let original = frame.instance_data()[0];
        let mut text = FakeTextPaint::default();

        let result = submit_display_list(&display_list, &mut frame, &mut text);

        assert_eq!(
            result,
            Err(FrameSceneSubmissionError::Preflight(case.expected))
        );
        assert_eq!(frame.active_clip(), Some(original_clip));
        assert!(text.requests.is_empty());
        assert_eq!(frame.instance_data().len(), 1);
        assert_eq!(frame.instance_data()[0].rect, original.rect);
        assert_eq!(frame.instance_data()[0].color, original.color);
        assert_eq!(frame.instance_data()[0].flags, original.flags);
        assert_eq!(frame.instance_data()[0].clip_rect, original.clip_rect);
    }
}
