//! Blockquote widget — left-border accent bar with indented text content.
//!
//! # Examples
//!
//! ```ignore
//! ui.blockquote(|ui| {
//!     ui.label("Someone once said something wise.");
//! });
//!
//! // Custom accent color
//! ui.blockquote_colored(Color::new(0.4, 0.6, 1.0, 1.0), |ui| {
//!     ui.label("A blue-accented quote.");
//! });
//! ```

use esox_gfx::{Color, ShapeBuilder};

use crate::layout::Rect;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a blockquote with the theme's accent color as the left border.
    pub fn blockquote(&mut self, f: impl FnOnce(&mut Self)) {
        let accent = self.theme.accent;
        self.blockquote_colored(accent, f);
    }

    /// Draw a blockquote with a custom accent color for the left border.
    pub fn blockquote_colored(&mut self, accent: Color, f: impl FnOnce(&mut Self)) {
        let bar_width = 3.0;
        let indent = bar_width + self.theme.padding;

        // Push a placeholder for the background bar (we don't know height yet).
        let placeholder_idx = self.frame.instance_len();
        self.frame.push(
            ShapeBuilder::rect(0.0, 0.0, 0.0, 0.0)
                .color(Color::new(0.0, 0.0, 0.0, 0.0))
                .build(),
        );

        let start_y = self.cursor.y;
        let saved_region = self.region;
        let saved_cursor_x = self.cursor.x;

        // Indent the content region.
        self.cursor.x += indent;
        self.region = Rect::new(
            self.region.x + indent,
            self.region.y,
            self.region.w - indent,
            self.region.h,
        );

        f(self);

        let end_y = self.cursor.y;
        let height = end_y - start_y;

        // Restore region.
        self.cursor.x = saved_cursor_x;
        self.region = saved_region;

        // Replace placeholder with the accent bar.
        let bar =
            ShapeBuilder::rounded_rect(self.region.x, start_y, bar_width, height, bar_width / 2.0)
                .color(accent)
                .build();
        self.frame.replace_instance(placeholder_idx, bar);

        // Spacing after the blockquote.
        self.cursor.y += self.theme.content_spacing;
    }
}
