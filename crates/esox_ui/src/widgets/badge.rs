//! Badge and status pill widgets.
//!
//! # Examples
//!
//! ```ignore
//! ui.row(|ui| {
//!     ui.label("Inbox");
//!     ui.badge(5);
//! });
//!
//! // Just a presence dot (no number)
//! ui.badge_dot();
//!
//! // Colored status pills
//! ui.status_pill_success("Active");
//! ui.status_pill_warning("Away");
//! ui.status_pill_error("Offline");
//! ```

use esox_gfx::{BorderRadius, Color, ShapeBuilder};

use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a notification badge with a count and custom colors. Shows "99+"
    /// for values > 99.
    pub fn badge_colored(&mut self, count: u32, bg: Color, fg: Color) {
        let text = if count > 99 {
            "99+".to_string()
        } else {
            count.to_string()
        };

        let font_size = self.theme.font_size * 0.75; // smaller text
        let pad_x = self.theme.badge_pad_x;
        let pad_y = self.theme.badge_pad_y;
        let text_w = self.text.measure_text(&text, font_size);
        let badge_w = (text_w + pad_x * 2.0).max(font_size + pad_y * 2.0); // at least circular
        let badge_h = font_size + pad_y * 2.0;
        let radius = badge_h / 2.0;

        let rect = self.allocate_rect(badge_w, badge_h);

        self.frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                .color(bg)
                .border_radius(BorderRadius::uniform(radius))
                .build(),
        );

        self.text.draw_text(
            &text,
            rect.x + (rect.w - text_w) / 2.0,
            rect.y + pad_y,
            font_size,
            fg,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a red notification badge with a count. Shows "99+" for values > 99.
    pub fn badge(&mut self, count: u32) {
        let bg = self.theme.red;
        let fg = self.theme.fg_on_accent;
        self.badge_colored(count, bg, fg);
    }

    /// Draw a small colored dot badge (no number) — a "new" indicator.
    pub fn badge_dot(&mut self) {
        let dot_size = 8.0;
        let rect = self.allocate_rect(dot_size, dot_size);
        let radius = dot_size / 2.0;

        self.frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                .color(self.theme.red)
                .border_radius(BorderRadius::uniform(radius))
                .build(),
        );
    }

    /// Draw a colored status pill with a text label.
    pub fn status_pill(&mut self, label: &str, bg: Color, fg: Color) {
        let font_size = self.theme.font_size * 0.75;
        let pad_x = self.theme.badge_pad_x;
        let pad_y = self.theme.badge_pad_y;
        let text_w = self.text.measure_text(label, font_size);
        let pill_w = text_w + pad_x * 2.0;
        let pill_h = font_size + pad_y * 2.0;
        let radius = pill_h / 2.0;

        let rect = self.allocate_rect(pill_w, pill_h);

        self.frame.push(
            ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                .color(bg)
                .border_radius(BorderRadius::uniform(radius))
                .build(),
        );

        self.text.draw_text(
            label,
            rect.x + (rect.w - text_w) / 2.0,
            rect.y + pad_y,
            font_size,
            fg,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Green status pill (e.g., "Active", "Online", "Success").
    pub fn status_pill_success(&mut self, label: &str) {
        let bg = self.theme.green;
        let fg = self.theme.fg_on_accent;
        self.status_pill(label, bg, fg);
    }

    /// Amber status pill (e.g., "Away", "Pending", "Warning").
    pub fn status_pill_warning(&mut self, label: &str) {
        let bg = self.theme.amber;
        // Dark text on amber for contrast.
        let fg = Color::new(0.15, 0.15, 0.15, 1.0);
        self.status_pill(label, bg, fg);
    }

    /// Red status pill (e.g., "Offline", "Error", "Critical").
    pub fn status_pill_error(&mut self, label: &str) {
        let bg = self.theme.red;
        let fg = self.theme.fg_on_accent;
        self.status_pill(label, bg, fg);
    }
}
