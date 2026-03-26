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
        let cx = rect.x + size / 2.0;
        let cy = rect.y + size / 2.0;
        let radius = size / 2.0;

        // Circle background.
        self.frame
            .push(ShapeBuilder::circle(cx, cy, radius).color(bg).build());

        // Centered initials.
        let font_size = size * 0.42;
        let display = &initials[..initials.len().min(2)];
        let text_w = self.text.measure_text(display, font_size);
        self.text.draw_text(
            display,
            cx - text_w / 2.0,
            cy - font_size / 2.0,
            font_size,
            Color::new(1.0, 1.0, 1.0, 1.0),
            self.frame,
            self.gpu,
            self.resources,
        );
    }
}
