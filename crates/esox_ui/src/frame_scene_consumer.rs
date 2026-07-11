//! Submission of a preflighted FrameCore display list to [`esox_gfx::Frame`].
//!
//! This adapter consumes only committed paint records. It does not consult or
//! derive layout, measurement, hit-test, semantic, focus, or damage geometry.

use std::error::Error;
use std::fmt;

use esox_gfx::{Color as GfxColor, Frame, GpuContext, RenderResources, ShapeBuilder};

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
            Self::Text(error) => Some(error),
        }
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

/// Submit an already-preflighted display list without deriving new geometry.
pub fn submit_prepared_display_list<TextBackend>(
    display_list: &PreparedDisplayList<'_>,
    frame: &mut Frame,
    text_backend: &mut TextBackend,
) -> Result<(), FrameSceneSubmissionError<TextBackend::Error>>
where
    TextBackend: TextPaintBoundary<Frame>,
{
    let previous_clip = frame.active_clip();

    for record in display_list.records() {
        frame.set_active_clip(record.effective_clip.map(logical_clip));

        match record.primitive {
            SubmissionPrimitive::SolidRect { color } => frame.push(
                ShapeBuilder::rect(
                    record.bounds.x,
                    record.bounds.y,
                    record.bounds.width,
                    record.bounds.height,
                )
                .color(gfx_color(color))
                .build(),
            ),
            SubmissionPrimitive::Border { color, width } => frame.push(
                ShapeBuilder::rect(
                    record.bounds.x,
                    record.bounds.y,
                    record.bounds.width,
                    record.bounds.height,
                )
                .color(gfx_color(color))
                .stroke(width)
                .build(),
            ),
            SubmissionPrimitive::Text(request) => {
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
    [rect.x, rect.y, rect.width, rect.height]
}
