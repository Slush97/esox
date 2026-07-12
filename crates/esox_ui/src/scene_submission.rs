//! Renderer-boundary vocabulary for submitting a committed display list.
//!
//! This module deliberately contains no renderer implementation. It validates
//! and borrows the already-resolved paint records without consulting any other
//! committed-scene products or recomputing geometry.

use std::error::Error;
use std::fmt;

use crate::frame_core::{Color, LogicalRect, PaintPrimitive, PaintRecord, TextDirection, WidgetId};

/// Text weights supported by the production renderer boundary in this slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionTextWeight {
    Regular,
    Bold,
}

impl SubmissionTextWeight {
    pub const fn value(self) -> u16 {
        match self {
            Self::Regular => 400,
            Self::Bold => 700,
        }
    }
}

/// A renderer-independent text paint request.
///
/// Preflight stores committed logical geometry. A concrete renderer boundary
/// may copy it into target coordinates immediately before invoking
/// [`TextPaintBoundary`]; committed records remain logical and unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPaintRequest<'a> {
    pub id: WidgetId,
    pub content: &'a str,
    pub bounds: LogicalRect,
    pub effective_clip: Option<LogicalRect>,
    pub color: Color,
    pub font_size: f32,
    pub font_weight: SubmissionTextWeight,
}

/// Injected boundary used by headless fakes and production text submission.
///
/// Implementations receive an explicit target so rasterization and atlas
/// access can remain confined to submission time.
pub trait TextPaintBoundary<Target> {
    type Error;

    fn paint_text(
        &mut self,
        target: &mut Target,
        request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error>;
}

/// One supported operation in its original logical display-list position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubmissionPrimitive<'a> {
    SolidRect { color: Color },
    RoundedRect { color: Color, radius: f32 },
    Border { color: Color, width: f32 },
    Text(TextPaintRequest<'a>),
}

/// A preflighted record borrowing geometry from the committed display list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubmissionRecord<'a> {
    pub id: WidgetId,
    pub bounds: LogicalRect,
    pub effective_clip: Option<LogicalRect>,
    pub primitive: SubmissionPrimitive<'a>,
}

/// A complete, validated display list ready for renderer mutation.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedDisplayList<'a> {
    records: Vec<SubmissionRecord<'a>>,
}

impl<'a> PreparedDisplayList<'a> {
    pub fn records(&self) -> &[SubmissionRecord<'a>] {
        &self.records
    }

    pub fn into_records(self) -> Vec<SubmissionRecord<'a>> {
        self.records
    }
}

/// Paint primitives intentionally unsupported by this renderer slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedPaintPrimitive {
    Box,
    Image,
}

/// A text requirement the current production renderer cannot honor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnsupportedTextCapability {
    FontFamily(String),
    Locale(String),
    Direction(TextDirection),
    FontWeight(u16),
}

/// Typed preflight failure. No renderer target may be mutated before success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneSubmissionError {
    InvalidGeometry {
        record_index: usize,
        id: WidgetId,
    },
    UnsupportedPrimitive {
        record_index: usize,
        id: WidgetId,
        primitive: UnsupportedPaintPrimitive,
    },
    UnsupportedText {
        record_index: usize,
        id: WidgetId,
        capability: UnsupportedTextCapability,
    },
}

impl fmt::Display for SceneSubmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry { record_index, id } => write!(
                formatter,
                "display-list record {record_index} ({id:?}) has non-finite geometry"
            ),
            Self::UnsupportedPrimitive {
                record_index,
                id,
                primitive,
            } => write!(
                formatter,
                "display-list record {record_index} ({id:?}) uses unsupported {primitive:?} paint"
            ),
            Self::UnsupportedText {
                record_index,
                id,
                capability,
            } => write!(
                formatter,
                "display-list record {record_index} ({id:?}) requires unsupported text capability {capability:?}"
            ),
        }
    }
}

impl Error for SceneSubmissionError {}

/// Validate the complete display list and preserve its logical record order.
///
/// The returned values copy only already-resolved bounds, clips, colors, and
/// widths, while text content continues to borrow the committed record.
pub fn preflight_display_list(
    display_list: &[PaintRecord],
) -> Result<PreparedDisplayList<'_>, SceneSubmissionError> {
    let mut records = Vec::with_capacity(display_list.len());

    for (record_index, record) in display_list.iter().enumerate() {
        let finite_rect = |rect: LogicalRect| {
            rect.x.is_finite()
                && rect.y.is_finite()
                && rect.width.is_finite()
                && rect.height.is_finite()
        };
        if !finite_rect(record.bounds) || record.effective_clip.is_some_and(|r| !finite_rect(r)) {
            return Err(SceneSubmissionError::InvalidGeometry {
                record_index,
                id: record.id,
            });
        }
        let primitive = match &record.primitive {
            PaintPrimitive::Box => {
                return Err(SceneSubmissionError::UnsupportedPrimitive {
                    record_index,
                    id: record.id,
                    primitive: UnsupportedPaintPrimitive::Box,
                });
            }
            PaintPrimitive::Image { .. } => {
                return Err(SceneSubmissionError::UnsupportedPrimitive {
                    record_index,
                    id: record.id,
                    primitive: UnsupportedPaintPrimitive::Image,
                });
            }
            PaintPrimitive::SolidRect { color } => SubmissionPrimitive::SolidRect { color: *color },
            PaintPrimitive::RoundedRect { color, radius } => {
                if !radius.is_finite() || *radius < 0.0 {
                    return Err(SceneSubmissionError::InvalidGeometry {
                        record_index,
                        id: record.id,
                    });
                }
                SubmissionPrimitive::RoundedRect {
                    color: *color,
                    radius: *radius,
                }
            }
            PaintPrimitive::Border { color, width } => {
                if !width.is_finite() {
                    return Err(SceneSubmissionError::InvalidGeometry {
                        record_index,
                        id: record.id,
                    });
                }
                SubmissionPrimitive::Border {
                    color: *color,
                    width: *width,
                }
            }
            PaintPrimitive::Text {
                content,
                properties,
                color,
            } => {
                if !properties.font_size.is_finite() {
                    return Err(SceneSubmissionError::InvalidGeometry {
                        record_index,
                        id: record.id,
                    });
                }
                let capability = properties
                    .font_family
                    .as_ref()
                    .map(|family| UnsupportedTextCapability::FontFamily(family.clone()))
                    .or_else(|| {
                        properties
                            .locale
                            .as_ref()
                            .map(|locale| UnsupportedTextCapability::Locale(locale.clone()))
                    })
                    .or_else(|| {
                        (properties.direction != TextDirection::Auto)
                            .then_some(UnsupportedTextCapability::Direction(properties.direction))
                    });
                if let Some(capability) = capability {
                    return Err(SceneSubmissionError::UnsupportedText {
                        record_index,
                        id: record.id,
                        capability,
                    });
                }

                let font_weight = match properties.font_weight {
                    400 => SubmissionTextWeight::Regular,
                    700 => SubmissionTextWeight::Bold,
                    weight => {
                        return Err(SceneSubmissionError::UnsupportedText {
                            record_index,
                            id: record.id,
                            capability: UnsupportedTextCapability::FontWeight(weight),
                        });
                    }
                };
                SubmissionPrimitive::Text(TextPaintRequest {
                    id: record.id,
                    content,
                    bounds: record.bounds,
                    effective_clip: record.effective_clip,
                    color: *color,
                    font_size: properties.font_size,
                    font_weight,
                })
            }
        };

        records.push(SubmissionRecord {
            id: record.id,
            bounds: record.bounds,
            effective_clip: record.effective_clip,
            primitive,
        });
    }

    Ok(PreparedDisplayList { records })
}
