//! Skeleton / loading placeholder widgets — animated shimmer rectangles.
//!
//! # Examples
//!
//! ```ignore
//! ui.skeleton(200.0, 20.0);    // generic placeholder
//! ui.skeleton_text();           // text-line shaped
//! ui.skeleton_circle(40.0);    // circular (for avatars)
//! ```

use std::time::Instant;

use esox_gfx::{BorderRadius, Color, ShapeBuilder};

use crate::Ui;

/// Epoch for shimmer phase calculation.
static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn epoch() -> Instant {
    *EPOCH.get_or_init(Instant::now)
}

impl<'f> Ui<'f> {
    /// Draw a rectangular skeleton placeholder with a shimmer animation.
    pub fn skeleton(&mut self, width: f32, height: f32) {
        let rect = self.allocate_rect(width, height);
        self.state.spinner_active = true; // request continuous repaint

        let radius = self.theme.corner_radius;
        let base = self.theme.bg_raised;

        // Base rect.
        self.frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                .color(base)
                .border_radius(BorderRadius::uniform(radius))
                .build(),
        );

        // Shimmer overlay — a travelling gradient band.
        let elapsed = Instant::now().duration_since(epoch()).as_secs_f32();
        let cycle = 1.5; // seconds per shimmer sweep
        let t = (elapsed % cycle) / cycle; // 0..1

        // Shimmer band position (travels from left to right).
        let band_w = rect.w * 0.4;
        let band_x = rect.x + (rect.w + band_w) * t - band_w;

        // Clip shimmer to the skeleton rect.
        let shimmer_x = band_x.max(rect.x);
        let shimmer_end = (band_x + band_w).min(rect.x + rect.w);
        let shimmer_w = (shimmer_end - shimmer_x).max(0.0);

        if shimmer_w > 0.0 {
            let shimmer = Color::new(
                (base.r + 0.06).min(1.0),
                (base.g + 0.06).min(1.0),
                (base.b + 0.06).min(1.0),
                0.5,
            );
            self.frame.push(
                ShapeBuilder::rect(shimmer_x, rect.y, shimmer_w, rect.h)
                    .color(shimmer)
                    .border_radius(BorderRadius::uniform(radius))
                    .build(),
            );
        }
    }

    /// Draw a text-line skeleton placeholder (font-size height, full region width).
    pub fn skeleton_text(&mut self) {
        let h = self.theme.font_size + self.theme.label_pad_y;
        self.skeleton(self.region.w * 0.7, h);
    }

    /// Draw a circular skeleton placeholder (e.g., for avatar loading).
    pub fn skeleton_circle(&mut self, diameter: f32) {
        let rect = self.allocate_rect(diameter, diameter);
        self.state.spinner_active = true;

        let radius = diameter / 2.0;
        let base = self.theme.bg_raised;

        self.frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                .color(base)
                .border_radius(BorderRadius::uniform(radius))
                .build(),
        );

        // Simplified shimmer for circles — just alpha pulse.
        let elapsed = Instant::now().duration_since(epoch()).as_secs_f32();
        let alpha = 0.15 + 0.15 * (elapsed * 3.0).sin();
        let shimmer = Color::new(1.0, 1.0, 1.0, alpha);
        self.frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                .color(shimmer)
                .border_radius(BorderRadius::uniform(radius))
                .build(),
        );
    }
}
