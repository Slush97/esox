//! Code block widget — monospace text on a dark background with optional copy button.
//!
//! # Examples
//!
//! ```ignore
//! // Simple code block
//! ui.code_block("let x = 42;\nprintln!(\"{x}\");");
//!
//! // With language label
//! ui.code_block_lang("rust", "fn main() {\n    println!(\"hello\");\n}");
//!
//! // Check if copy was clicked
//! if ui.code_block("some code").clicked {
//!     // user clicked the copy button — text is on clipboard
//! }
//! ```

use esox_gfx::{Color, ShapeBuilder};

use crate::response::Response;
use crate::state::WidgetKind;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a code block with monospace text on a contrasting background.
    ///
    /// Returns a response where `clicked` indicates the copy button was pressed.
    pub fn code_block(&mut self, id: u64, code: &str) -> Response {
        self.code_block_inner(id, None, code)
    }

    /// Draw a code block with a language label in the top-right corner.
    pub fn code_block_lang(&mut self, id: u64, language: &str, code: &str) -> Response {
        self.code_block_inner(id, Some(language), code)
    }

    fn code_block_inner(&mut self, id: u64, language: Option<&str>, code: &str) -> Response {
        let pad = self.theme.padding;
        let radius = self.theme.corner_radius;
        let font_size = self.theme.font_size * 0.9;
        let line_height = self.text.line_height(font_size);
        let line_spacing = 2.0;

        // Count lines and measure content height.
        let lines: Vec<&str> = code.lines().collect();
        let num_lines = lines.len().max(1);
        let content_h = num_lines as f32 * (line_height + line_spacing) - line_spacing;

        // Header height for language label / copy button row.
        let header_h = if language.is_some() {
            line_height + pad * 0.5
        } else {
            0.0
        };

        let total_h = pad + header_h + content_h + pad;

        // Background colors — dark surface for code.
        let bg = darken(self.theme.bg_surface, 0.15);
        let fg = self.theme.fg;
        let fg_muted = self.theme.fg_muted;

        // Allocate the full block rect.
        let rect = self.allocate_rect_keyed(id, self.region.w, total_h);
        self.register_widget(id, rect, WidgetKind::Button);

        // Background.
        self.frame.push(
            ShapeBuilder::rounded_rect(rect.x, rect.y, rect.w, rect.h, radius)
                .color(bg)
                .build(),
        );

        let mut y = rect.y + pad;

        // Language label + copy button.
        if let Some(lang) = language {
            let label_size = font_size * 0.85;
            self.text.draw_text(
                lang,
                rect.x + pad,
                y,
                label_size,
                fg_muted,
                self.frame,
                self.gpu,
                self.resources,
            );
            y += line_height + pad * 0.5;
        }

        // Copy button area (top-right).
        let copy_label = "Copy";
        let copy_size = font_size * 0.85;
        let copy_w = self.text.measure_text(copy_label, copy_size);
        let copy_pad = 6.0;
        let copy_rect_x = rect.x + rect.w - copy_w - copy_pad * 2.0 - pad;
        let copy_rect_y = rect.y + pad * 0.5;
        let copy_rect_w = copy_w + copy_pad * 2.0;
        let copy_rect_h = line_height + copy_pad;
        let copy_rect =
            crate::layout::Rect::new(copy_rect_x, copy_rect_y, copy_rect_w, copy_rect_h);

        let copy_hovered = copy_rect.contains(self.state.mouse.x, self.state.mouse.y);
        let copy_clicked = copy_hovered
            && self
                .state
                .mouse
                .pending_click
                .is_some_and(|(cx, cy, _)| copy_rect.contains(cx, cy));

        if copy_clicked {
            self.state.mouse.pending_click = None;
        }

        // Copy button background (subtle on hover).
        if copy_hovered {
            self.frame.push(
                ShapeBuilder::rounded_rect(copy_rect.x, copy_rect.y, copy_rect.w, copy_rect.h, 4.0)
                    .color(Color::new(1.0, 1.0, 1.0, 0.1))
                    .build(),
            );
        }

        self.text.draw_text(
            copy_label,
            copy_rect.x + copy_pad,
            copy_rect.y + copy_pad * 0.5,
            copy_size,
            if copy_hovered { fg } else { fg_muted },
            self.frame,
            self.gpu,
            self.resources,
        );

        // Render code lines.
        for (i, line) in lines.iter().enumerate() {
            let line_y = y + i as f32 * (line_height + line_spacing);
            self.text.draw_text(
                line,
                rect.x + pad,
                line_y,
                font_size,
                fg,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        // Add spacing after the block.
        self.cursor.y += self.theme.content_spacing;

        Response {
            clicked: copy_clicked,
            hovered: false,
            ..Response::default()
        }
    }
}

/// Darken a color by a factor (0.0 = no change, 1.0 = black).
fn darken(c: Color, amount: f32) -> Color {
    let f = 1.0 - amount;
    Color::new(c.r * f, c.g * f, c.b * f, c.a)
}
