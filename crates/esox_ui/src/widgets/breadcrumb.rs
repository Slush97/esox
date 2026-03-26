//! Breadcrumb navigation trail widget.
//!
//! # Examples
//!
//! ```ignore
//! if let Some(clicked) = ui.breadcrumb(id!("nav"), &["Home", "Settings", "Profile"]) {
//!     navigate_to(clicked);
//! }
//! ```

use crate::id::{fnv1a_mix, HOVER_SALT};
use crate::state::WidgetKind;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a breadcrumb trail. Returns `Some(index)` if a segment was clicked.
    ///
    /// The last segment is rendered as the current page (non-clickable).
    /// All preceding segments are interactive links.
    pub fn breadcrumb(&mut self, id: u64, segments: &[&str]) -> Option<usize> {
        if segments.is_empty() {
            return None;
        }

        let font_size = self.theme.font_size;
        let sep = " \u{203A} "; // single right-pointing angle quotation mark
        let sep_w = self.text.measure_text(sep, font_size);

        // Measure total width.
        let mut total_w = 0.0f32;
        for (i, seg) in segments.iter().enumerate() {
            total_w += self.text.measure_text(seg, font_size);
            if i + 1 < segments.len() {
                total_w += sep_w;
            }
        }

        let height = font_size + self.theme.label_pad_y;
        let rect = self.allocate_rect(total_w.min(self.region.w), height);
        let mut pen_x = rect.x;
        let mut clicked_index = None;

        for (i, seg) in segments.iter().enumerate() {
            let seg_w = self.text.measure_text(seg, font_size);
            let is_last = i + 1 == segments.len();

            if is_last {
                // Current page — non-clickable, standard color.
                self.text.draw_text(
                    seg,
                    pen_x,
                    rect.y,
                    font_size,
                    self.theme.fg,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            } else {
                // Clickable segment — accent colored, hover underline.
                let seg_id = fnv1a_mix(id, i as u64);
                let seg_rect = crate::layout::Rect::new(pen_x, rect.y, seg_w, height);
                self.register_widget(seg_id, seg_rect, WidgetKind::Hyperlink);
                let response = self.widget_response(seg_id, seg_rect);

                let hover_t = self.state.hover_t(
                    seg_id ^ HOVER_SALT,
                    response.hovered,
                    self.theme.hover_duration_ms,
                );
                let color =
                    crate::paint::lerp_color(self.theme.accent, self.theme.accent_hover, hover_t);

                self.text.draw_text(
                    seg,
                    pen_x,
                    rect.y,
                    font_size,
                    color,
                    self.frame,
                    self.gpu,
                    self.resources,
                );

                // Underline on hover.
                if hover_t > 0.0 {
                    let uy = rect.y + font_size + 1.0;
                    let underline_color =
                        esox_gfx::Color::new(color.r, color.g, color.b, color.a * hover_t);
                    self.frame.push(
                        esox_gfx::ShapeBuilder::rect(pen_x, uy, seg_w, 1.0)
                            .color(underline_color)
                            .build(),
                    );
                }

                if response.clicked {
                    clicked_index = Some(i);
                }
            }

            pen_x += seg_w;

            // Separator between segments.
            if !is_last {
                self.text.draw_text(
                    sep,
                    pen_x,
                    rect.y,
                    font_size,
                    self.theme.fg_dim,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
                pen_x += sep_w;
            }
        }

        clicked_index
    }
}
