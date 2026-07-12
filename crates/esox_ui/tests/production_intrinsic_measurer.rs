use esox_ui::frame_core::{
    AvailableLength, AvailableSize, ImageMeasureRequest, IntrinsicMeasurer, KnownDimensions,
    LogicalSize, TextMeasureRequest, TextProperties,
};
use esox_ui::{ProductionIntrinsicMeasurer, ProductionMeasurerError};

fn measurer() -> ProductionIntrinsicMeasurer {
    let font = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-data/JetBrainsMono-Regular.ttf"
    ))
    .unwrap();
    ProductionIntrinsicMeasurer::from_font_data("Test Mono", font, None).unwrap()
}

fn available(width: AvailableLength) -> AvailableSize {
    AvailableSize {
        width,
        height: AvailableLength::MaxContent,
    }
}

#[test]
fn cpu_text_measurement_shapes_and_wraps_without_renderer_state() {
    let measurer = measurer();
    let properties = TextProperties {
        font_family: Some("Test Mono".into()),
        font_size: 16.0,
        ..TextProperties::default()
    };
    let natural = measurer.measure_text(TextMeasureRequest {
        content: "abcd",
        properties: &properties,
        known_dimensions: KnownDimensions::default(),
        available_space: available(AvailableLength::MaxContent),
    });
    let constrained = measurer.measure_text(TextMeasureRequest {
        content: "abcd",
        properties: &properties,
        known_dimensions: KnownDimensions::default(),
        available_space: available(AvailableLength::Definite(natural.width / 2.0)),
    });

    assert!(natural.width > 0.0);
    assert!(natural.height > 0.0);
    assert_eq!(constrained.width, natural.width / 2.0);
    assert!(constrained.height > natural.height);

    let fixed = measurer.measure_text(TextMeasureRequest {
        content: "this content must not influence fixed dimensions",
        properties: &properties,
        known_dimensions: KnownDimensions {
            width: Some(37.0),
            height: Some(19.0),
        },
        available_space: available(AvailableLength::Definite(1.0)),
    });
    assert_eq!(fixed, LogicalSize::new(37.0, 19.0));
}

#[test]
fn decoded_image_metadata_preserves_aspect_ratio_and_validates_registration() {
    let mut measurer = measurer();
    measurer
        .register_image(7, LogicalSize::new(80.0, 40.0))
        .unwrap();
    assert_eq!(measurer.image_size(7), Some(LogicalSize::new(80.0, 40.0)));

    let width_constrained = measurer.measure_image(ImageMeasureRequest {
        resource: 7,
        known_dimensions: KnownDimensions {
            width: Some(20.0),
            height: None,
        },
        available_space: available(AvailableLength::Definite(10.0)),
    });
    assert_eq!(width_constrained, LogicalSize::new(20.0, 10.0));

    assert!(matches!(
        measurer.register_image(8, LogicalSize::new(f32::NAN, 10.0)),
        Err(ProductionMeasurerError::InvalidImageSize(size))
            if size.width.is_nan() && size.height == 10.0
    ));
    assert_eq!(measurer.remove_image(7), Some(LogicalSize::new(80.0, 40.0)));
    assert_eq!(
        measurer.measure_image(ImageMeasureRequest {
            resource: 7,
            known_dimensions: KnownDimensions::default(),
            available_space: available(AvailableLength::MaxContent),
        }),
        LogicalSize::default()
    );
}
