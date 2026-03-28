//! Rating widget — interactive star rating.
//!
//! # Examples
//!
//! ```ignore
//! // Interactive (click to set)
//! ui.rating(id!("review"), &mut stars, 5);
//!
//! // Read-only display
//! ui.rating_display(3.5, 5);
//! ```

use esox_gfx::{Color, ShapeBuilder};

use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, WidgetKind};
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw an interactive star rating. `value` is 0..=max. Returns Response
    /// with `changed = true` when the user clicks a star.
    pub fn rating(&mut self, id: u64, value: &mut u8, max: u8) -> Response {
        let star_size = self.theme.font_size + self.theme.spacing_unit;
        let gap = self.theme.spacing_unit;
        let max = max.max(1);
        let total_w = star_size * max as f32 + gap * (max as f32 - 1.0);
        let height = star_size;

        let rect = self.allocate_rect_keyed(id, total_w, height);
        self.register_widget(id, rect, WidgetKind::Slider);
        let response = self.widget_response(id, rect);

        // Hover preview: determine which star the mouse is over.
        let hover_star = if response.hovered {
            let rel_x = self.state.mouse.x - rect.x;
            let star_i = (rel_x / (star_size + gap)).floor() as u8;
            Some(star_i.min(max - 1))
        } else {
            None
        };

        // Click to set value.
        let mut changed = false;
        if response.clicked {
            if let Some(star_i) = hover_star {
                let new_val = star_i + 1;
                if new_val != *value {
                    *value = new_val;
                    changed = true;
                }
            }
        }

        // Keyboard: ArrowLeft/Right to adjust.
        if response.focused {
            use esox_input::{Key, NamedKey};
            for (event, _) in &self.state.keys {
                if !event.pressed {
                    continue;
                }
                match &event.key {
                    Key::Named(NamedKey::ArrowRight) if *value < max => {
                        *value += 1;
                        changed = true;
                    }
                    Key::Named(NamedKey::ArrowLeft) if *value > 0 => {
                        *value -= 1;
                        changed = true;
                    }
                    _ => {}
                }
            }
        }

        let filled_color = self.theme.amber;
        let empty_color = self.theme.fg_dim;

        // Draw stars.
        for i in 0..max {
            let cx = rect.x + star_size / 2.0 + (star_size + gap) * i as f32;
            let cy = rect.y + star_size / 2.0;
            let outer_r = star_size / 2.0 - 1.0;
            let inner_r = outer_r * 0.4;

            let filled = i < *value;
            let preview = hover_star.is_some_and(|h| i <= h);

            let color = if filled || preview {
                if preview && !filled {
                    // Preview highlight — slightly dimmer.
                    Color::new(filled_color.r, filled_color.g, filled_color.b, 0.5)
                } else {
                    filled_color
                }
            } else {
                empty_color
            };

            self.frame.push(
                ShapeBuilder::star(cx, cy, 5, inner_r, outer_r)
                    .color(color)
                    .build(),
            );
        }

        // Focus ring.
        if response.focused {
            paint::draw_focus_ring(
                self.frame,
                rect,
                self.theme.focus_ring_color,
                self.theme.corner_radius,
                self.theme.focus_ring_expand,
            );
        }

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Slider,
            label: format!("{} of {} stars", value, max),
            value: Some(format!("{}", value)),
            rect,
            focused: response.focused,
            disabled: response.disabled,
            expanded: None,
            selected: None,
            checked: None,
            value_range: Some((0.0, max as f32, *value as f32)),
            children: Vec::new(),
        });

        Response {
            changed,
            ..response
        }
    }

    /// Draw a read-only star rating display. `value` can be fractional (e.g., 3.5).
    pub fn rating_display(&mut self, value: f32, max: u8) {
        let star_size = self.theme.font_size;
        let gap = self.theme.spacing_unit * 0.75;
        let max = max.max(1);
        let total_w = star_size * max as f32 + gap * (max as f32 - 1.0);

        let rect = self.allocate_rect(total_w, star_size);

        let filled_color = self.theme.amber;
        let empty_color = self.theme.fg_dim;

        for i in 0..max {
            let cx = rect.x + star_size / 2.0 + (star_size + gap) * i as f32;
            let cy = rect.y + star_size / 2.0;
            let outer_r = star_size / 2.0 - 1.0;
            let inner_r = outer_r * 0.4;

            let fill_amount = (value - i as f32).clamp(0.0, 1.0);
            let color = if fill_amount >= 0.5 {
                filled_color
            } else {
                empty_color
            };

            self.frame.push(
                ShapeBuilder::star(cx, cy, 5, inner_r, outer_r)
                    .color(color)
                    .build(),
            );
        }
    }
}
