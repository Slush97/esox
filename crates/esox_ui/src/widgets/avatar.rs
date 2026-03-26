//! Avatar widget — circular display with initials or image.
//!
//! # Examples
//!
//! ```ignore
//! ui.avatar("JD", 40.0);                           // auto-colored circle
//! ui.avatar_colored("AB", 32.0, Color::new(0.2, 0.6, 0.9, 1.0));  // custom color
//! ```

use esox_gfx::{Color, ShapeBuilder};

use crate::Ui;

/// Derive a deterministic color from initials for consistent per-user coloring.
fn color_from_initials(initials: &str) -> Color {
    let mut hash: u32 = 0x811c9dc5;
    for b in initials.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    // Map hash to a pleasant hue (avoid too-dark or too-light).
    let hue = (hash % 360) as f32;
    let (r, g, b) = hsl_to_rgb(hue, 0.55, 0.50);
    Color::new(r, g, b, 1.0)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    (r1 + m, g1 + m, b1 + m)
}

/// Online presence status for avatar overlay dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Online,
    Idle,
    DoNotDisturb,
    Offline,
}

impl Status {
    fn color(self) -> Color {
        match self {
            Status::Online => Color::new(0.23, 0.70, 0.44, 1.0), // green
            Status::Idle => Color::new(0.98, 0.66, 0.10, 1.0),   // amber
            Status::DoNotDisturb => Color::new(0.91, 0.30, 0.24, 1.0), // red
            Status::Offline => Color::new(0.45, 0.47, 0.50, 1.0), // gray
        }
    }
}

impl<'f> Ui<'f> {
    /// Draw a circular avatar with initials (1-2 characters).
    /// Color is deterministically derived from the initials.
    pub fn avatar(&mut self, initials: &str, size: f32) {
        let bg = color_from_initials(initials);
        self.avatar_colored(initials, size, bg);
    }

    /// Draw a circular avatar with initials and a custom background color.
    pub fn avatar_colored(&mut self, initials: &str, size: f32, bg: Color) {
        let rect = self.allocate_rect(size, size);
        Self::draw_avatar_circle(
            initials,
            size,
            bg,
            rect.x,
            rect.y,
            self.text,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a circular avatar with a status dot overlay.
    pub fn avatar_with_status(&mut self, initials: &str, size: f32, status: Status) {
        let bg = color_from_initials(initials);
        self.avatar_colored_with_status(initials, size, bg, status);
    }

    /// Draw a circular avatar with a custom color and status dot overlay.
    pub fn avatar_colored_with_status(
        &mut self,
        initials: &str,
        size: f32,
        bg: Color,
        status: Status,
    ) {
        let rect = self.allocate_rect(size, size);
        Self::draw_avatar_circle(
            initials,
            size,
            bg,
            rect.x,
            rect.y,
            self.text,
            self.frame,
            self.gpu,
            self.resources,
        );

        // Status dot: bottom-right corner, ~30% of avatar size.
        let dot_size = (size * 0.3).max(8.0);
        let dot_radius = dot_size / 2.0;
        let dot_cx = rect.x + size - dot_radius;
        let dot_cy = rect.y + size - dot_radius;

        // Cutout ring (background-colored circle behind the dot for visual separation).
        let ring_radius = dot_radius + 2.0;
        self.frame.push(
            ShapeBuilder::circle(dot_cx, dot_cy, ring_radius)
                .color(self.theme.bg_base)
                .build(),
        );

        // Status dot.
        self.frame.push(
            ShapeBuilder::circle(dot_cx, dot_cy, dot_radius)
                .color(status.color())
                .build(),
        );
    }

    #[allow(clippy::too_many_arguments)] // rendering helper passes through Ui fields
    fn draw_avatar_circle(
        initials: &str,
        size: f32,
        bg: Color,
        x: f32,
        y: f32,
        text: &mut crate::TextRenderer,
        frame: &mut esox_gfx::Frame,
        gpu: &esox_gfx::GpuContext,
        resources: &mut esox_gfx::RenderResources,
    ) {
        let cx = x + size / 2.0;
        let cy = y + size / 2.0;
        let radius = size / 2.0;

        // Circle background.
        frame.push(ShapeBuilder::circle(cx, cy, radius).color(bg).build());

        // Centered initials.
        let font_size = size * 0.42;
        let display = &initials[..initials.len().min(2)];
        let text_w = text.measure_text(display, font_size);
        text.draw_text(
            display,
            cx - text_w / 2.0,
            cy - font_size / 2.0,
            font_size,
            Color::new(1.0, 1.0, 1.0, 1.0),
            frame,
            gpu,
            resources,
        );
    }
}
