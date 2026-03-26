//! Drawing helpers — borders, focus rings, dashed outlines.

use esox_gfx::{BorderRadius, Color, Frame, ShapeBuilder};

use crate::layout::Rect;
use crate::theme::{Elevation, Gradient};

/// Draw a 1px solid border around a rectangle (non-rounded).
pub fn draw_border(frame: &mut Frame, rect: Rect, color: Color) {
    draw_rounded_border(frame, rect, color, 0.0);
}

/// Draw a 1px solid border around a rounded rectangle.
///
/// Uses a stroked SDF rounded rect so the border follows the corner radius
/// instead of drawing straight lines that poke out at the corners.
pub fn draw_rounded_border(frame: &mut Frame, rect: Rect, color: Color, radius: f32) {
    frame.push(
        ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
            .color(color)
            .border_radius(BorderRadius::uniform(radius))
            .stroke(1.0)
            .build(),
    );
}

/// Draw a rounded rectangle.
pub fn draw_rounded_rect(frame: &mut Frame, rect: Rect, color: Color, radius: f32) {
    frame.push(
        ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
            .color(color)
            .border_radius(BorderRadius::uniform(radius))
            .build(),
    );
}

/// Draw a rounded rectangle with an elevation shadow.
///
/// The shadow is rendered via the GPU SDF shader (`ShapeBuilder::shadow`),
/// producing a smooth Gaussian blur. When `elevation.blur` is zero, this
/// falls back to a plain `draw_rounded_rect` with no shadow overhead.
pub fn draw_elevated_rect(
    frame: &mut Frame,
    rect: Rect,
    color: Color,
    radius: f32,
    elevation: &Elevation,
) {
    if elevation.blur < 0.001 {
        draw_rounded_rect(frame, rect, color, radius);
        return;
    }
    frame.push(
        ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
            .color(color)
            .border_radius(BorderRadius::uniform(radius))
            .shadow(elevation.blur, elevation.dx, elevation.dy)
            .color2(elevation.color)
            .build(),
    );
}

/// Draw a focus ring (stroked rounded rect with gap around the widget).
///
/// Draws a 2px stroke ring with `gap` pixels between the widget edge and
/// the inner edge of the ring.
pub fn draw_focus_ring(frame: &mut Frame, rect: Rect, color: Color, radius: f32, gap: f32) {
    let stroke_width = 2.0;
    let offset = gap + stroke_width / 2.0;
    frame.push(
        ShapeBuilder::rect(
            rect.x - offset,
            rect.y - offset,
            rect.w + offset * 2.0,
            rect.h + offset * 2.0,
        )
        .color(color)
        .border_radius(BorderRadius::uniform(radius + offset))
        .stroke(stroke_width)
        .build(),
    );
}

/// Draw a vertical line (1px wide).
pub fn draw_vline(frame: &mut Frame, x: f32, y: f32, h: f32, color: Color) {
    frame.push(ShapeBuilder::rect(x, y, 1.0, h).color(color).build());
}

/// Linearly interpolate between two colors by `t` in [0, 1].
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Draw a gradient fade overlay at a scroll edge.
///
/// `from` is the color at the start of the gradient direction.
/// `to` is the color at the end.
/// `angle` is the gradient direction in radians (PI/2 = top-to-bottom, 0 = left-to-right).
pub fn draw_scroll_fade(frame: &mut Frame, rect: Rect, from: Color, to: Color, angle: f32) {
    frame.push(
        ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
            .color(from)
            .linear_gradient(to, angle)
            .build(),
    );
}

/// Draw a dashed border around a rectangle.
pub fn draw_dashed_border(
    frame: &mut Frame,
    rect: Rect,
    color: Color,
    dash: f32,
    gap: f32,
    thickness: f32,
) {
    let (x, y, w, h) = (rect.x, rect.y, rect.w, rect.h);

    // Top and bottom edges.
    let mut dx = x;
    while dx < x + w {
        let seg_w = dash.min(x + w - dx);
        frame.push(
            ShapeBuilder::rect(dx, y, seg_w, thickness)
                .color(color)
                .build(),
        );
        frame.push(
            ShapeBuilder::rect(dx, y + h - thickness, seg_w, thickness)
                .color(color)
                .build(),
        );
        dx += dash + gap;
    }

    // Left and right edges.
    let mut dy = y;
    while dy < y + h {
        let seg_h = dash.min(y + h - dy);
        frame.push(
            ShapeBuilder::rect(x, dy, thickness, seg_h)
                .color(color)
                .build(),
        );
        frame.push(
            ShapeBuilder::rect(x + w - thickness, dy, thickness, seg_h)
                .color(color)
                .build(),
        );
        dy += dash + gap;
    }
}

/// Draw a fully-styled rectangle with gradient, shadow, border, and opacity.
///
/// This is the primary drawing function for widgets that need per-widget
/// visual customization. All parameters are optional via the resolver system.
#[allow(clippy::too_many_arguments)] // Visual properties are inherently multi-dimensional.
pub fn draw_styled_rect(
    frame: &mut Frame,
    rect: Rect,
    bg: Color,
    gradient: Option<Gradient>,
    radius: BorderRadius,
    border_color: Option<Color>,
    border_width: f32,
    elevation: Option<&Elevation>,
    opacity: f32,
) {
    // Background fill (with optional gradient and shadow).
    let mut builder = ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
        .color(bg)
        .border_radius(radius)
        .opacity(opacity);

    if let Some(grad) = gradient {
        builder = match grad {
            Gradient::Linear { end_color, angle } => builder.linear_gradient(end_color, angle),
            Gradient::Radial { end_color } => builder.radial_gradient(end_color),
            Gradient::Conic {
                end_color,
                start_angle,
            } => builder.conic_gradient(end_color, start_angle),
        };
    }

    if let Some(elev) = elevation {
        if elev.blur > 0.001 {
            builder = builder
                .shadow(elev.blur, elev.dx, elev.dy)
                .color2(elev.color);
        }
    }

    frame.push(builder.build());

    // Border stroke (separate shape so it layers on top).
    if let Some(bc) = border_color {
        if border_width > 0.0 {
            frame.push(
                ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                    .color(bc)
                    .border_radius(radius)
                    .stroke(border_width)
                    .opacity(opacity)
                    .build(),
            );
        }
    }
}

/// Draw per-side borders as thin rectangles.
///
/// Each side is optional: pass `Some((color, width))` to draw, `None` to skip.
pub fn draw_per_side_border(
    frame: &mut Frame,
    rect: Rect,
    top: Option<(Color, f32)>,
    right: Option<(Color, f32)>,
    bottom: Option<(Color, f32)>,
    left: Option<(Color, f32)>,
) {
    if let Some((color, width)) = top {
        frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, width)
                .color(color)
                .build(),
        );
    }
    if let Some((color, width)) = bottom {
        frame.push(
            ShapeBuilder::rect(rect.x, rect.y + rect.h - width, rect.w, width)
                .color(color)
                .build(),
        );
    }
    if let Some((color, width)) = left {
        frame.push(
            ShapeBuilder::rect(rect.x, rect.y, width, rect.h)
                .color(color)
                .build(),
        );
    }
    if let Some((color, width)) = right {
        frame.push(
            ShapeBuilder::rect(rect.x + rect.w - width, rect.y, width, rect.h)
                .color(color)
                .build(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_color_at_zero_returns_a() {
        let a = Color::new(0.2, 0.4, 0.6, 1.0);
        let b = Color::new(0.8, 0.1, 0.3, 0.5);
        let result = lerp_color(a, b, 0.0);
        assert_eq!(result, a);
    }

    #[test]
    fn lerp_color_at_one_returns_b() {
        let a = Color::new(0.2, 0.4, 0.6, 1.0);
        let b = Color::new(0.8, 0.1, 0.3, 0.5);
        let result = lerp_color(a, b, 1.0);
        assert!((result.r - b.r).abs() < 1e-6);
        assert!((result.g - b.g).abs() < 1e-6);
        assert!((result.b - b.b).abs() < 1e-6);
        assert!((result.a - b.a).abs() < 1e-6);
    }

    #[test]
    fn lerp_color_at_half_returns_midpoint() {
        let a = Color::new(0.0, 0.0, 0.0, 0.0);
        let b = Color::new(1.0, 1.0, 1.0, 1.0);
        let result = lerp_color(a, b, 0.5);
        assert!((result.r - 0.5).abs() < 1e-6);
        assert!((result.g - 0.5).abs() < 1e-6);
        assert!((result.b - 0.5).abs() < 1e-6);
        assert!((result.a - 0.5).abs() < 1e-6);
    }

    #[test]
    fn lerp_color_midpoint_non_trivial() {
        let a = Color::new(0.2, 0.4, 0.6, 0.8);
        let b = Color::new(0.8, 0.2, 0.4, 0.4);
        let result = lerp_color(a, b, 0.5);
        assert!((result.r - 0.5).abs() < 1e-6);
        assert!((result.g - 0.3).abs() < 1e-6);
        assert!((result.b - 0.5).abs() < 1e-6);
        assert!((result.a - 0.6).abs() < 1e-6);
    }
}
