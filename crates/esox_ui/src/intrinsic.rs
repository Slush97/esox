//! GPU-independent production intrinsic measurement.
//!
//! This module owns only CPU font shaping and decoded image dimensions. It
//! deliberately does not own a surface, texture atlas, glyph rasterizer, or
//! editor geometry.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, MutexGuard};

use esox_font::{FontFace, FontId, FontStyle, SystemFontDb, TextShaper};

use crate::frame_core::{
    AvailableLength, ImageMeasureRequest, IntrinsicMeasurer, LogicalSize, TextMeasureRequest,
};

const DEFAULT_FONT_FAMILIES: &[&str] = &["Inter", "Noto Sans", "DejaVu Sans", "Liberation Sans"];

/// Construction or metadata-registration error for production measurement.
#[derive(Clone, Debug, PartialEq)]
pub enum ProductionMeasurerError {
    Font(String),
    InvalidImageSize(LogicalSize),
}

impl fmt::Display for ProductionMeasurerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Font(error) => write!(formatter, "font initialization failed: {error}"),
            Self::InvalidImageSize(size) => write!(
                formatter,
                "image intrinsic size must be finite and non-negative, got {}x{}",
                size.width, size.height
            ),
        }
    }
}

impl std::error::Error for ProductionMeasurerError {}

/// CPU-only production implementation of [`IntrinsicMeasurer`].
///
/// The configured regular face is the stable fallback for every family. A
/// bold face, when present, is selected for weights of 600 and above. Image
/// resources are resolved from decoded pixel dimensions registered by the
/// application before layout.
pub struct ProductionIntrinsicMeasurer {
    family: String,
    regular: FontFace,
    bold: Option<FontFace>,
    shaper: Mutex<TextShaper>,
    images: HashMap<u64, LogicalSize>,
}

impl fmt::Debug for ProductionIntrinsicMeasurer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductionIntrinsicMeasurer")
            .field("family", &self.family)
            .field("regular", &self.regular)
            .field("bold", &self.bold)
            .field("image_count", &self.images.len())
            .finish_non_exhaustive()
    }
}

impl ProductionIntrinsicMeasurer {
    /// Load the same preferred system sans-serif family used by `TextRenderer`.
    pub fn new() -> Result<Self, ProductionMeasurerError> {
        let database = SystemFontDb::new();
        let regular = DEFAULT_FONT_FAMILIES
            .iter()
            .find_map(|family| database.query_family(family, FontStyle::Regular))
            .ok_or_else(|| {
                ProductionMeasurerError::Font("no supported sans-serif font found".into())
            })?;
        let bold = database.query_family(&regular.family, FontStyle::Bold);
        Self::from_font_data(regular.family, regular.data, bold.map(|font| font.data))
    }

    /// Construct from caller-supplied font bytes without touching the GPU or system font lookup.
    pub fn from_font_data(
        family: impl Into<String>,
        regular: Vec<u8>,
        bold: Option<Vec<u8>>,
    ) -> Result<Self, ProductionMeasurerError> {
        let regular = FontFace::from_bytes(FontId(0), regular)
            .map_err(|error| ProductionMeasurerError::Font(error.to_string()))?;
        let bold = bold
            .map(|data| FontFace::from_bytes(FontId(1), data))
            .transpose()
            .map_err(|error| ProductionMeasurerError::Font(error.to_string()))?;
        Ok(Self {
            family: family.into(),
            regular,
            bold,
            shaper: Mutex::new(TextShaper::new()),
            images: HashMap::new(),
        })
    }

    /// The concrete family loaded as this measurer's production fallback.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Register decoded image dimensions for a renderer-neutral resource key.
    pub fn register_image(
        &mut self,
        resource: u64,
        size: LogicalSize,
    ) -> Result<(), ProductionMeasurerError> {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width < 0.0
            || size.height < 0.0
        {
            return Err(ProductionMeasurerError::InvalidImageSize(size));
        }
        self.images.insert(resource, size);
        Ok(())
    }

    /// Remove and return decoded image dimensions for a resource key.
    pub fn remove_image(&mut self, resource: u64) -> Option<LogicalSize> {
        self.images.remove(&resource)
    }

    /// Read registered decoded dimensions without invoking layout.
    pub fn image_size(&self, resource: u64) -> Option<LogicalSize> {
        self.images.get(&resource).copied()
    }

    fn lock_shaper(&self) -> MutexGuard<'_, TextShaper> {
        self.shaper
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn face(&self, weight: u16) -> &FontFace {
        if weight >= 600 {
            self.bold.as_ref().unwrap_or(&self.regular)
        } else {
            &self.regular
        }
    }

    fn advance(shaper: &mut TextShaper, face: &FontFace, text: &str, size: f32) -> f32 {
        shaper
            .shape(face, text, size)
            .glyphs
            .iter()
            .map(|glyph| glyph.x_advance)
            .sum::<f32>()
            .max(0.0)
    }

    fn natural_dimensions(
        shaper: &mut TextShaper,
        face: &FontFace,
        content: &str,
        size: f32,
        line_height: f32,
    ) -> LogicalSize {
        let width = content
            .split('\n')
            .map(|line| Self::advance(shaper, face, line, size))
            .fold(0.0_f32, f32::max);
        let lines = content.split('\n').count().max(1) as f32;
        LogicalSize::new(width, lines * line_height)
    }

    fn min_content_width(
        shaper: &mut TextShaper,
        face: &FontFace,
        content: &str,
        size: f32,
    ) -> f32 {
        content
            .split_whitespace()
            .map(|word| Self::advance(shaper, face, word, size))
            .fold(0.0_f32, f32::max)
    }

    fn wrapped_line_count(
        shaper: &mut TextShaper,
        face: &FontFace,
        content: &str,
        size: f32,
        width: f32,
    ) -> usize {
        content
            .split('\n')
            .map(|line| {
                if line.is_empty() {
                    return 1;
                }
                let mut lines = 1;
                let mut line_width = 0.0;
                for character in line.chars() {
                    let advance =
                        Self::advance(shaper, face, character.encode_utf8(&mut [0; 4]), size);
                    if line_width > 0.0 && line_width + advance > width {
                        lines += 1;
                        line_width = 0.0;
                    }
                    line_width += advance;
                }
                lines
            })
            .sum::<usize>()
            .max(1)
    }
}

impl IntrinsicMeasurer for ProductionIntrinsicMeasurer {
    fn measure_text(&self, request: TextMeasureRequest<'_>) -> LogicalSize {
        if let (Some(width), Some(height)) = (
            request.known_dimensions.width,
            request.known_dimensions.height,
        ) {
            return LogicalSize::new(width, height);
        }

        let size = request.properties.font_size.max(0.0);
        let face = self.face(request.properties.font_weight);
        let line_height = face.metrics(size).cell_height.max(0.0);
        let mut shaper = self.lock_shaper();
        let natural =
            Self::natural_dimensions(&mut shaper, face, request.content, size, line_height);

        let measured_width = match request.known_dimensions.width {
            Some(width) => width,
            None => match request.available_space.width {
                AvailableLength::Definite(width) => natural.width.min(width.max(0.0)),
                AvailableLength::MinContent => {
                    Self::min_content_width(&mut shaper, face, request.content, size)
                }
                AvailableLength::MaxContent => natural.width,
            },
        };
        let measured_height = request.known_dimensions.height.unwrap_or_else(|| {
            if measured_width < natural.width {
                Self::wrapped_line_count(
                    &mut shaper,
                    face,
                    request.content,
                    size,
                    measured_width.max(f32::EPSILON),
                ) as f32
                    * line_height
            } else {
                natural.height
            }
        });
        LogicalSize::new(measured_width, measured_height)
    }

    fn measure_image(&self, request: ImageMeasureRequest) -> LogicalSize {
        let intrinsic = self
            .images
            .get(&request.resource)
            .copied()
            .unwrap_or_default();
        match (
            request.known_dimensions.width,
            request.known_dimensions.height,
        ) {
            (Some(width), Some(height)) => LogicalSize::new(width, height),
            (Some(width), None) if intrinsic.width > 0.0 => {
                LogicalSize::new(width, intrinsic.height * width / intrinsic.width)
            }
            (None, Some(height)) if intrinsic.height > 0.0 => {
                LogicalSize::new(intrinsic.width * height / intrinsic.height, height)
            }
            (Some(width), None) => LogicalSize::new(width, intrinsic.height),
            (None, Some(height)) => LogicalSize::new(intrinsic.width, height),
            (None, None) => intrinsic,
        }
    }
}
