//! Stepper widget — horizontal workflow step indicator.
//!
//! # Examples
//!
//! ```ignore
//! if let Some(clicked) = ui.stepper(id!("wizard"), &["Account", "Profile", "Review"], 1) {
//!     current_step = clicked;
//! }
//! ```

use esox_gfx::ShapeBuilder;

use crate::id::{fnv1a_mix, HOVER_SALT};
use crate::paint;
use crate::state::{A11yNode, A11yRole, WidgetKind};
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a horizontal step indicator. Returns `Some(index)` if a step was clicked.
    ///
    /// Steps before `current` are shown as completed (filled accent + checkmark),
    /// the `current` step is highlighted (accent border), and future steps are dimmed.
    pub fn stepper(&mut self, id: u64, labels: &[&str], current: usize) -> Option<usize> {
        if labels.is_empty() {
            return None;
        }

        let circle_size =
            self.theme.space(crate::theme::SpacingScale::Xl) + self.theme.spacing_unit;
        let circle_r = circle_size / 2.0;
        let line_h = self.theme.spacing_unit * 0.5;
        let font_size = self.theme.font_size * 0.85;
        let gap = self.theme.content_spacing;
        let label_h = font_size + self.theme.label_pad_y;
        let total_h = circle_size + gap + label_h;

        let rect = self.allocate_rect(self.region.w, total_h);

        let n = labels.len();
        // Distribute circles evenly across available width.
        let step_spacing = if n > 1 {
            (rect.w - circle_size) / (n - 1) as f32
        } else {
            0.0
        };

        let mut clicked_index = None;

        for (i, &label) in labels.iter().enumerate() {
            let cx = rect.x + circle_r + step_spacing * i as f32;
            let cy = rect.y + circle_r;

            let is_completed = i < current;
            let is_current = i == current;
            let step_id = fnv1a_mix(id, i as u64);

            // Hit area for the step.
            let hit_pad = self.theme.spacing_unit;
            let hit_rect = crate::layout::Rect::new(
                cx - circle_r - hit_pad,
                rect.y,
                circle_size + hit_pad * 2.0,
                total_h,
            );
            self.register_widget(step_id, hit_rect, WidgetKind::Button);
            let response = self.widget_response(step_id, hit_rect);

            let hover_t = self.state.hover_t(
                step_id ^ HOVER_SALT,
                response.hovered,
                self.theme.hover_duration_ms,
            );

            if response.clicked {
                clicked_index = Some(i);
            }

            // Connecting line to the next step.
            if i + 1 < n {
                let line_gap = self.theme.spacing_unit;
                let line_x = cx + circle_r + line_gap;
                let next_cx = rect.x + circle_r + step_spacing * (i + 1) as f32;
                let line_w = next_cx - circle_r - line_gap - line_x;
                let line_color = if is_completed {
                    self.theme.accent
                } else {
                    self.theme.border
                };
                self.frame.push(
                    ShapeBuilder::rect(line_x, cy - line_h / 2.0, line_w.max(0.0), line_h)
                        .color(line_color)
                        .build(),
                );
            }

            // Circle.
            if is_completed {
                // Filled accent circle.
                self.frame.push(
                    ShapeBuilder::circle(cx, cy, circle_r)
                        .color(self.theme.accent)
                        .build(),
                );
                // Checkmark (simple text).
                let check = "\u{2713}";
                let check_w = self.text.measure_text(check, font_size);
                self.text.draw_text(
                    check,
                    cx - check_w / 2.0,
                    cy - font_size / 2.0,
                    font_size,
                    self.theme.fg_on_accent,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            } else if is_current {
                // Accent-bordered circle with number.
                self.frame.push(
                    ShapeBuilder::circle(cx, cy, circle_r)
                        .color(self.theme.accent)
                        .stroke(2.0)
                        .build(),
                );
                let num = format!("{}", i + 1);
                let num_w = self.text.measure_text(&num, font_size);
                self.text.draw_text(
                    &num,
                    cx - num_w / 2.0,
                    cy - font_size / 2.0,
                    font_size,
                    self.theme.accent,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            } else {
                // Dimmed circle with number.
                let border_color =
                    paint::lerp_color(self.theme.border, self.theme.fg_muted, hover_t);
                self.frame.push(
                    ShapeBuilder::circle(cx, cy, circle_r)
                        .color(border_color)
                        .stroke(1.5)
                        .build(),
                );
                let num = format!("{}", i + 1);
                let num_w = self.text.measure_text(&num, font_size);
                let num_color = paint::lerp_color(self.theme.fg_dim, self.theme.fg_muted, hover_t);
                self.text.draw_text(
                    &num,
                    cx - num_w / 2.0,
                    cy - font_size / 2.0,
                    font_size,
                    num_color,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            }

            // Label below circle.
            let label_w = self.text.measure_text(label, font_size);
            let label_color = if is_current {
                self.theme.fg
            } else if is_completed {
                self.theme.fg_muted
            } else {
                self.theme.fg_dim
            };
            self.text.draw_text(
                label,
                cx - label_w / 2.0,
                rect.y + circle_size + gap,
                font_size,
                label_color,
                self.frame,
                self.gpu,
                self.resources,
            );

            self.push_a11y_node(A11yNode {
                id: step_id,
                role: A11yRole::Button,
                label: format!("Step {}: {}", i + 1, label),
                value: None,
                rect: hit_rect,
                focused: response.focused,
                disabled: false,
                expanded: None,
                selected: Some(is_current),
                checked: Some(is_completed),
                value_range: None,
                children: Vec::new(),
            });

            // Focus ring.
            if response.focused {
                paint::draw_focus_ring(
                    self.frame,
                    crate::layout::Rect::new(
                        cx - circle_r,
                        cy - circle_r,
                        circle_size,
                        circle_size,
                    ),
                    self.theme.focus_ring_color,
                    circle_r,
                    self.theme.focus_ring_expand,
                );
            }
        }

        clicked_index
    }
}
