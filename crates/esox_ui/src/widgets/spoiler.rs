//! Spoiler widget — hidden content that reveals on click.
//!
//! # Examples
//!
//! ```ignore
//! ui.spoiler(id!("secret"), |ui| {
//!     ui.label("This was hidden until you clicked it!");
//! });
//! ```

use esox_gfx::{Color, ShapeBuilder};

use crate::layout::Rect;
use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, WidgetKind};
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a spoiler block. Content is hidden (opaque overlay) until clicked.
    ///
    /// State is tracked internally by `id` — once revealed, stays revealed until
    /// the widget is no longer drawn.
    pub fn spoiler(&mut self, id: u64, f: impl FnOnce(&mut Self)) -> Response {
        let revealed = self.state.spoiler_revealed(id);

        let start_y = self.cursor.y;

        // Always render content (so layout is stable).
        f(self);

        let end_y = self.cursor.y;
        let content_h = end_y - start_y;
        let content_rect = Rect::new(self.region.x, start_y, self.region.w, content_h);

        self.register_widget(id, content_rect, WidgetKind::Button);
        let response = self.widget_response(id, content_rect);

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Button,
            label: if revealed {
                "Spoiler (revealed)".to_string()
            } else {
                "Spoiler — click to reveal".to_string()
            },
            value: None,
            rect: content_rect,
            focused: response.focused,
            disabled: false,
            expanded: Some(revealed),
            selected: None,
            checked: None,
            value_range: None,
            children: Vec::new(),
        });

        if response.clicked && !revealed {
            self.state.reveal_spoiler(id);
        }

        // Focus ring when unrevealed.
        if response.focused && !revealed {
            paint::draw_focus_ring(
                self.frame,
                content_rect,
                self.theme.focus_ring_color,
                self.theme.corner_radius,
                self.theme.focus_ring_expand,
            );
        }

        if !revealed {
            // Draw the obscuring overlay *after* content so it renders on top.
            let overlay_color = Color::new(
                self.theme.bg_surface.r,
                self.theme.bg_surface.g,
                self.theme.bg_surface.b,
                1.0,
            );
            let radius = self.theme.corner_radius;

            self.frame.push(
                ShapeBuilder::rounded_rect(
                    content_rect.x,
                    content_rect.y,
                    content_rect.w,
                    content_rect.h,
                    radius,
                )
                .color(overlay_color)
                .build(),
            );

            // "Click to reveal" hint centered on the block.
            let hint = "Spoiler — click to reveal";
            let hint_size = self.theme.font_size * 0.85;
            let hint_w = self.text.measure_text(hint, hint_size);
            let hint_x = content_rect.x + (content_rect.w - hint_w) / 2.0;
            let hint_y = content_rect.y + (content_rect.h - hint_size) / 2.0;
            self.text.draw_text(
                hint,
                hint_x,
                hint_y,
                hint_size,
                self.theme.fg_muted,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        response
    }
}
