use esox_ui::frame_core::{
    Color, LogicalRect, PaintPrimitive, PaintRecord, TextDirection, TextProperties, WidgetId,
};
use esox_ui::scene_submission::{
    preflight_display_list, SceneSubmissionError, SubmissionPrimitive, SubmissionTextWeight,
    UnsupportedPaintPrimitive, UnsupportedTextCapability,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn text_record(id: u64, properties: TextProperties) -> PaintRecord {
    PaintRecord {
        id: WidgetId(id),
        primitive: PaintPrimitive::Text {
            content: "styled".to_owned(),
            properties,
            color: Color::rgba(0.1, 0.2, 0.3, 0.4),
        },
        bounds: rect(10.25, 11.5, 83.75, 19.125),
        effective_clip: Some(rect(8.0, 9.0, 90.0, 20.0)),
    }
}

#[test]
fn preflight_preserves_supported_order_and_values() {
    let display_list = vec![
        PaintRecord {
            id: WidgetId(1),
            primitive: PaintPrimitive::SolidRect {
                color: Color::rgba(0.25, 0.5, 0.75, 1.0),
            },
            bounds: rect(1.25, 2.5, 30.75, 40.125),
            effective_clip: Some(rect(2.0, 3.0, 20.0, 21.0)),
        },
        text_record(
            2,
            TextProperties {
                font_size: 18.5,
                font_weight: 700,
                ..TextProperties::default()
            },
        ),
        PaintRecord {
            id: WidgetId(3),
            primitive: PaintPrimitive::Border {
                color: Color::rgba(0.9, 0.8, 0.7, 0.6),
                width: 2.25,
            },
            bounds: rect(5.5, 6.25, 70.125, 80.875),
            effective_clip: None,
        },
    ];

    let prepared = preflight_display_list(&display_list).unwrap();
    assert_eq!(prepared.records().len(), 3);
    assert_eq!(prepared.records()[0].id, WidgetId(1));
    assert_eq!(prepared.records()[1].id, WidgetId(2));
    assert_eq!(prepared.records()[2].id, WidgetId(3));
    assert_eq!(prepared.records()[0].bounds, display_list[0].bounds);
    assert_eq!(
        prepared.records()[0].effective_clip,
        display_list[0].effective_clip
    );

    let SubmissionPrimitive::Text(request) = prepared.records()[1].primitive else {
        panic!("second record must remain text");
    };
    assert_eq!(request.content, "styled");
    assert_eq!(request.bounds, display_list[1].bounds);
    assert_eq!(request.effective_clip, display_list[1].effective_clip);
    assert_eq!(request.color, Color::rgba(0.1, 0.2, 0.3, 0.4));
    assert_eq!(request.font_size, 18.5);
    assert_eq!(request.font_weight, SubmissionTextWeight::Bold);

    assert_eq!(
        prepared.records()[2].primitive,
        SubmissionPrimitive::Border {
            color: Color::rgba(0.9, 0.8, 0.7, 0.6),
            width: 2.25,
        }
    );
}

#[test]
fn unsupported_primitives_are_typed_with_their_original_position() {
    for (primitive, unsupported) in [
        (PaintPrimitive::Box, UnsupportedPaintPrimitive::Box),
        (
            PaintPrimitive::Image { resource: 42 },
            UnsupportedPaintPrimitive::Image,
        ),
    ] {
        let records = vec![
            PaintRecord {
                id: WidgetId(10),
                primitive: PaintPrimitive::SolidRect {
                    color: Color::BLACK,
                },
                bounds: rect(0.0, 0.0, 1.0, 1.0),
                effective_clip: None,
            },
            PaintRecord {
                id: WidgetId(11),
                primitive,
                bounds: rect(1.0, 1.0, 2.0, 2.0),
                effective_clip: None,
            },
        ];

        assert_eq!(
            preflight_display_list(&records).unwrap_err(),
            SceneSubmissionError::UnsupportedPrimitive {
                record_index: 1,
                id: WidgetId(11),
                primitive: unsupported,
            }
        );
    }
}

#[test]
fn every_unimplemented_text_requirement_is_rejected_explicitly() {
    let cases = [
        (
            TextProperties {
                font_family: Some("Inter".to_owned()),
                ..TextProperties::default()
            },
            UnsupportedTextCapability::FontFamily("Inter".to_owned()),
        ),
        (
            TextProperties {
                locale: Some("en-US".to_owned()),
                ..TextProperties::default()
            },
            UnsupportedTextCapability::Locale("en-US".to_owned()),
        ),
        (
            TextProperties {
                direction: TextDirection::RightToLeft,
                ..TextProperties::default()
            },
            UnsupportedTextCapability::Direction(TextDirection::RightToLeft),
        ),
        (
            TextProperties {
                font_weight: 500,
                ..TextProperties::default()
            },
            UnsupportedTextCapability::FontWeight(500),
        ),
    ];

    for (properties, capability) in cases {
        assert_eq!(
            preflight_display_list(&[text_record(20, properties)]).unwrap_err(),
            SceneSubmissionError::UnsupportedText {
                record_index: 0,
                id: WidgetId(20),
                capability,
            }
        );
    }
}

#[test]
fn regular_and_bold_are_the_only_supported_weights() {
    for (weight, expected) in [
        (400, SubmissionTextWeight::Regular),
        (700, SubmissionTextWeight::Bold),
    ] {
        let records = [text_record(
            30,
            TextProperties {
                font_weight: weight,
                ..TextProperties::default()
            },
        )];
        let prepared = preflight_display_list(&records).unwrap();
        let SubmissionPrimitive::Text(request) = prepared.records()[0].primitive else {
            panic!("record must remain text");
        };
        assert_eq!(request.font_weight, expected);
        assert_eq!(request.font_weight.value(), weight);
    }
}

#[test]
fn rounded_rect_preflight_preserves_exact_radius_and_rejects_invalid_radius() {
    let record = PaintRecord {
        id: WidgetId(40),
        primitive: PaintPrimitive::RoundedRect {
            color: Color::BLACK,
            radius: 7.25,
        },
        bounds: rect(1.0, 2.0, 30.0, 10.0),
        effective_clip: None,
    };
    let records = [record];
    let prepared = preflight_display_list(&records).unwrap();
    assert_eq!(
        prepared.records()[0].primitive,
        SubmissionPrimitive::RoundedRect {
            color: Color::BLACK,
            radius: 7.25,
        }
    );

    let invalid = PaintRecord {
        id: WidgetId(41),
        primitive: PaintPrimitive::RoundedRect {
            color: Color::BLACK,
            radius: f32::NAN,
        },
        bounds: rect(1.0, 2.0, 30.0, 10.0),
        effective_clip: None,
    };
    assert_eq!(
        preflight_display_list(&[invalid]).unwrap_err(),
        SceneSubmissionError::InvalidGeometry {
            record_index: 0,
            id: WidgetId(41),
        }
    );
}
