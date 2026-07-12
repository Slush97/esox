//! Submission of a preflighted FrameCore display list to [`esox_gfx::Frame`].
//!
//! This adapter consumes only committed paint records. It does not consult or
//! derive layout, measurement, hit-test, semantic, focus, or damage geometry.

use std::error::Error;
use std::fmt;

use esox_gfx::{BorderRadius, Color as GfxColor, Frame, GpuContext, RenderResources, ShapeBuilder};

use crate::frame_core::{Color, LogicalRect, PaintRecord};
use crate::scene_submission::{
    preflight_display_list, PreparedDisplayList, SceneSubmissionError, SubmissionPrimitive,
    SubmissionTextWeight, TextPaintBoundary, TextPaintRequest,
};
use crate::text::TextRenderer;

/// Failure while preflighting or painting a committed display list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameSceneSubmissionError<TextError> {
    /// The complete display list was rejected before [`Frame`] was mutated.
    Preflight(SceneSubmissionError),
    /// The logical-to-physical renderer transform was invalid or overflowed.
    InvalidTransform,
    /// The injected text backend failed while painting a supported request.
    Text(TextError),
}

impl<TextError> fmt::Display for FrameSceneSubmissionError<TextError>
where
    TextError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight(error) => error.fmt(formatter),
            Self::InvalidTransform => formatter.write_str("invalid renderer coordinate transform"),
            Self::Text(error) => write!(formatter, "text submission failed: {error}"),
        }
    }
}

impl<TextError> Error for FrameSceneSubmissionError<TextError>
where
    TextError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::InvalidTransform => None,
            Self::Text(error) => Some(error),
        }
    }
}

/// A validated logical-to-physical scale used only during renderer submission.
///
/// Platform input performs the inverse conversion before events reach
/// FrameCore. Keeping this type at submission prevents committed logical scene
/// geometry from being converted earlier or more than once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RendererScale(f32);

impl RendererScale {
    pub fn new(scale_factor: f64) -> Option<Self> {
        let scale = scale_factor as f32;
        (scale_factor.is_finite() && scale_factor > 0.0 && scale.is_finite()).then_some(Self(scale))
    }

    pub const fn one() -> Self {
        Self(1.0)
    }

    pub const fn get(self) -> f32 {
        self.0
    }

    fn rect(self, rect: LogicalRect) -> Option<LogicalRect> {
        let scaled = LogicalRect {
            x: rect.x * self.0,
            y: rect.y * self.0,
            width: rect.width * self.0,
            height: rect.height * self.0,
        };
        (scaled.x.is_finite()
            && scaled.y.is_finite()
            && scaled.width.is_finite()
            && scaled.height.is_finite())
        .then_some(scaled)
    }
}

impl<TextError> From<SceneSubmissionError> for FrameSceneSubmissionError<TextError> {
    fn from(error: SceneSubmissionError) -> Self {
        Self::Preflight(error)
    }
}

/// Preflight and submit a committed display list in its original logical order.
///
/// Unsupported primitives and text capabilities are discovered across the
/// complete list before the frame or text backend is touched. The frame's
/// previous active clip is restored on both success and text-backend failure.
pub fn submit_display_list<TextBackend>(
    display_list: &[PaintRecord],
    frame: &mut Frame,
    text_backend: &mut TextBackend,
) -> Result<(), FrameSceneSubmissionError<TextBackend::Error>>
where
    TextBackend: TextPaintBoundary<Frame>,
{
    let prepared = preflight_display_list(display_list)?;
    submit_prepared_display_list(&prepared, frame, text_backend)
}

/// Preflight and submit a logical display list in physical renderer pixels.
pub fn submit_display_list_scaled<TextBackend>(
    display_list: &[PaintRecord],
    frame: &mut Frame,
    text_backend: &mut TextBackend,
    scale: RendererScale,
) -> Result<(), FrameSceneSubmissionError<TextBackend::Error>>
where
    TextBackend: TextPaintBoundary<Frame>,
{
    let prepared = preflight_display_list(display_list)?;
    submit_prepared_display_list_scaled(&prepared, frame, text_backend, scale)
}

/// Submit an already-preflighted display list without deriving new geometry.
pub fn submit_prepared_display_list<TextBackend>(
    display_list: &PreparedDisplayList<'_>,
    frame: &mut Frame,
    text_backend: &mut TextBackend,
) -> Result<(), FrameSceneSubmissionError<TextBackend::Error>>
where
    TextBackend: TextPaintBoundary<Frame>,
{
    submit_prepared_display_list_scaled(display_list, frame, text_backend, RendererScale::one())
}

/// Submit a preflighted logical display list using one physical renderer scale.
pub fn submit_prepared_display_list_scaled<TextBackend>(
    display_list: &PreparedDisplayList<'_>,
    frame: &mut Frame,
    text_backend: &mut TextBackend,
    scale: RendererScale,
) -> Result<(), FrameSceneSubmissionError<TextBackend::Error>>
where
    TextBackend: TextPaintBoundary<Frame>,
{
    for record in display_list.records() {
        let valid = scale.rect(record.bounds).is_some()
            && record
                .effective_clip
                .is_none_or(|clip| scale.rect(clip).is_some())
            && match record.primitive {
                SubmissionPrimitive::SolidRect { .. } => true,
                SubmissionPrimitive::RoundedRect { radius, .. } => {
                    (radius * scale.get()).is_finite()
                }
                SubmissionPrimitive::Border { width, .. } => (width * scale.get()).is_finite(),
                SubmissionPrimitive::Text(request) => {
                    (request.font_size * scale.get()).is_finite()
                        && scale.rect(request.bounds).is_some()
                        && request
                            .effective_clip
                            .is_none_or(|clip| scale.rect(clip).is_some())
                }
            };
        if !valid {
            return Err(FrameSceneSubmissionError::InvalidTransform);
        }
    }

    let previous_clip = frame.active_clip();

    for record in display_list.records() {
        let bounds = scale
            .rect(record.bounds)
            .expect("the complete transform was preflighted");
        frame.set_active_clip(
            record
                .effective_clip
                .map(|clip| logical_clip(scale.rect(clip).expect("clip was preflighted"))),
        );

        match record.primitive {
            SubmissionPrimitive::SolidRect { color } => frame.push(
                ShapeBuilder::rect(bounds.x, bounds.y, bounds.width, bounds.height)
                    .color(gfx_color(color))
                    .build(),
            ),
            SubmissionPrimitive::RoundedRect { color, radius } => frame.push(
                ShapeBuilder::rect(bounds.x, bounds.y, bounds.width, bounds.height)
                    .color(gfx_color(color))
                    .border_radius(BorderRadius::uniform(radius * scale.get()))
                    .build(),
            ),
            SubmissionPrimitive::Border { color, width } => frame.push(
                ShapeBuilder::rect(bounds.x, bounds.y, bounds.width, bounds.height)
                    .color(gfx_color(color))
                    .stroke(width * scale.get())
                    .build(),
            ),
            SubmissionPrimitive::Text(mut request) => {
                request.bounds = bounds;
                request.effective_clip = request
                    .effective_clip
                    .map(|clip| scale.rect(clip).expect("text clip was preflighted"));
                request.font_size *= scale.get();
                if let Err(error) = text_backend.paint_text(frame, request) {
                    frame.set_active_clip(previous_clip);
                    return Err(FrameSceneSubmissionError::Text(error));
                }
            }
        }
    }

    frame.set_active_clip(previous_clip);
    Ok(())
}

/// Production text boundary; all rasterizer, atlas, and GPU access is borrowed
/// only for the duration of renderer submission.
pub struct TextRendererFramePaint<'a> {
    renderer: &'a mut TextRenderer,
    gpu: &'a GpuContext,
    resources: &'a mut RenderResources,
}

impl<'a> TextRendererFramePaint<'a> {
    pub fn new(
        renderer: &'a mut TextRenderer,
        gpu: &'a GpuContext,
        resources: &'a mut RenderResources,
    ) -> Self {
        Self {
            renderer,
            gpu,
            resources,
        }
    }
}

impl TextPaintBoundary<Frame> for TextRendererFramePaint<'_> {
    type Error = std::convert::Infallible;

    fn paint_text(
        &mut self,
        frame: &mut Frame,
        request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error> {
        let style = match request.font_weight {
            SubmissionTextWeight::Regular => 0,
            SubmissionTextWeight::Bold => 1,
        };
        self.renderer.draw_text_styled(
            request.content,
            request.bounds.x,
            request.bounds.y,
            request.font_size,
            gfx_color(request.color),
            style,
            frame,
            self.gpu,
            self.resources,
        );
        Ok(())
    }
}

const fn gfx_color(color: Color) -> GfxColor {
    GfxColor::new(color.r, color.g, color.b, color.a)
}

const fn logical_clip(rect: LogicalRect) -> [f32; 4] {
    if rect.width <= 0.0 && rect.height <= 0.0 {
        // Frame's all-zero instance clip is the no-clip sentinel. Preserve an
        // explicitly empty scene clip by keeping one dimension non-zero; the
        // other zero dimension still produces an empty scissor.
        [rect.x, rect.y, 0.0, f32::EPSILON]
    } else {
        [rect.x, rect.y, rect.width, rect.height]
    }
}
