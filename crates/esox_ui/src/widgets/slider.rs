//! Slider widget — horizontal range control.

use esox_gfx::ShapeBuilder;

use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, InputState, WidgetKind};
use crate::Ui;

impl<'f> Ui<'f> {
    /// Typed f32 slider — takes `&mut f32` directly, no `InputState` boilerplate.
    pub fn slider_f32(&mut self, id: u64, value: &mut f32, min: f32, max: f32) -> Response {
        let mut input = InputState::new();
        input.text = if (max - min) >= 10.0 {
            format!("{}", (*value).round() as i32)
        } else {
            format!("{:.2}", *value)
        };
        let response = self.slider(id, &mut input, min, max);
        if response.changed {
            if let Ok(v) = input.text.parse::<f32>() {
                *value = v.clamp(min, max);
            }
        }
        response
    }

    /// Typed f64 slider — takes `&mut f64` directly, no `InputState` boilerplate.
    pub fn slider_f64(&mut self, id: u64, value: &mut f64, min: f64, max: f64) -> Response {
        let mut input = InputState::new();
        input.text = if (max - min) >= 10.0 {
            format!("{}", (*value).round() as i64)
        } else {
            format!("{:.2}", *value)
        };
        let response = self.slider(id, &mut input, min as f32, max as f32);
        if response.changed {
            if let Ok(v) = input.text.parse::<f64>() {
                *value = v.clamp(min, max);
            }
        }
        response
    }

    /// Draw a horizontal slider. Value is stored in `input.text` as a decimal string.
    /// Clicking anywhere on the track sets the value proportionally.
    pub fn slider(&mut self, id: u64, input: &mut InputState, min: f32, max: f32) -> Response {
        let rect = self.allocate_rect_keyed(id, self.region.w, self.theme.button_height);
        self.register_widget(id, rect, WidgetKind::Slider);

        let mut response = self.widget_response(id, rect);
        let disabled = response.disabled;

        // Parse current value, clamped to range.
        let current: f32 = input.text.parse().unwrap_or(min);
        let mut value = current.clamp(min, max);

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Slider,
            label: String::new(),
            value: Some(input.text.clone()),
            rect,
            focused: response.focused,
            disabled,
            expanded: None,
            selected: None,
            checked: None,
            value_range: Some((min, max, value)),
            children: Vec::new(),
        });

        // Handle click — map x to value.
        if response.clicked {
            let track_x = rect.x + self.theme.input_padding;
            let track_w = rect.w - self.theme.input_padding * 2.0;
            let rel = ((self.state.mouse.x - track_x) / track_w).clamp(0.0, 1.0);
            value = min + rel * (max - min);
            // Round to integer if range is large enough, otherwise 1 decimal.
            let formatted = if (max - min) >= 10.0 {
                format!("{}", value.round() as i32)
            } else {
                format!("{:.1}", value)
            };
            input.text = formatted;
            input.cursor = input.text.len();
            response.changed = true;
        }

        // Keyboard: arrow keys adjust value.
        if response.focused && !disabled {
            use esox_input::{Key, NamedKey};
            for (event, _) in &self.state.keys {
                if !event.pressed {
                    continue;
                }
                let step = if (max - min) >= 10.0 {
                    1.0
                } else {
                    (max - min) / 20.0
                };
                match &event.key {
                    Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => {
                        value = (value - step).clamp(min, max);
                        let formatted = if (max - min) >= 10.0 {
                            format!("{}", value.round() as i32)
                        } else {
                            format!("{:.1}", value)
                        };
                        input.text = formatted;
                        input.cursor = input.text.len();
                        response.changed = true;
                    }
                    Key::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => {
                        value = (value + step).clamp(min, max);
                        let formatted = if (max - min) >= 10.0 {
                            format!("{}", value.round() as i32)
                        } else {
                            format!("{:.1}", value)
                        };
                        input.text = formatted;
                        input.cursor = input.text.len();
                        response.changed = true;
                    }
                    _ => {}
                }
            }
        }

        // Focus ring.
        if response.focused && !disabled {
            paint::draw_focus_ring(
                self.frame,
                rect,
                self.theme.focus_ring_color,
                self.theme.corner_radius,
                self.theme.focus_ring_expand,
            );
        }

        // Draw background.
        let bg = if disabled {
            self.theme.disabled_bg
        } else {
            self.theme.bg_input
        };
        paint::draw_rounded_rect(self.frame, rect, bg, self.theme.corner_radius);

        // Border.
        if disabled {
            paint::draw_dashed_border(
                self.frame,
                rect,
                self.theme.disabled_border,
                self.theme.disabled_dash_len,
                self.theme.disabled_dash_gap,
                self.theme.disabled_dash_thickness,
            );
        } else {
            let border_color = if response.focused {
                self.theme.accent
            } else {
                self.theme.border
            };
            paint::draw_rounded_border(self.frame, rect, border_color, self.theme.corner_radius);
        }

        // Track area.
        let track_x = rect.x + self.theme.input_padding;
        let track_y = rect.y + rect.h / 2.0 - 2.0;
        let track_w = rect.w - self.theme.input_padding * 2.0;
        let track_h = self.theme.slider_track_height;

        // Track background.
        self.frame.push(
            ShapeBuilder::rect(track_x, track_y, track_w, track_h)
                .color(self.theme.bg_raised)
                .build(),
        );

        // Filled portion.
        let t = if (max - min).abs() < f32::EPSILON {
            0.0
        } else {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        };
        let filled_w = track_w * t;
        let fill_color = if disabled {
            self.theme.disabled_fg
        } else {
            self.theme.accent
        };
        if filled_w > 0.0 {
            self.frame.push(
                ShapeBuilder::rect(track_x, track_y, filled_w, track_h)
                    .color(fill_color)
                    .build(),
            );
        }

        // Thumb circle.
        let thumb_x = track_x + filled_w;
        let thumb_r = 6.0;
        let thumb_color = if disabled {
            self.theme.disabled_fg
        } else if response.hovered || response.focused {
            self.theme.accent_hover
        } else {
            self.theme.accent
        };
        self.frame.push(
            ShapeBuilder::circle(thumb_x, rect.y + rect.h / 2.0, thumb_r)
                .color(thumb_color)
                .build(),
        );

        // Value label on the right.
        let val_str = if input.text.is_empty() {
            format!("{}", min as i32)
        } else {
            input.text.clone()
        };
        let val_w = self.text.measure_text(&val_str, self.theme.font_size);
        self.text.draw_ui_text(
            &val_str,
            rect.x + rect.w - self.theme.input_padding - val_w,
            rect.y + (rect.h - self.theme.font_size) / 2.0,
            self.theme.fg_muted,
            self.frame,
            self.gpu,
            self.resources,
        );

        response
    }

    /// f32 slider with min/max endpoint labels and a unit suffix.
    ///
    /// Displays the current value with `unit` appended (e.g., "14 px") centered
    /// above the thumb, and `min`/`max` labels at the track endpoints.
    pub fn slider_f32_labeled(
        &mut self,
        id: u64,
        value: &mut f32,
        min: f32,
        max: f32,
        unit: &str,
    ) -> Response {
        let large_range = (max - min) >= 10.0;
        let mut input = InputState::new();
        input.text = if large_range {
            format!("{}", (*value).round() as i32)
        } else {
            format!("{:.2}", *value)
        };
        let response = self.slider_labeled_inner(id, &mut input, min, max, unit);
        if response.changed {
            if let Ok(v) = input.text.parse::<f32>() {
                *value = v.clamp(min, max);
            }
        }
        response
    }

    /// f64 slider with min/max endpoint labels and a unit suffix.
    pub fn slider_f64_labeled(
        &mut self,
        id: u64,
        value: &mut f64,
        min: f64,
        max: f64,
        unit: &str,
    ) -> Response {
        let large_range = (max - min) >= 10.0;
        let mut input = InputState::new();
        input.text = if large_range {
            format!("{}", (*value).round() as i64)
        } else {
            format!("{:.2}", *value)
        };
        let response = self.slider_labeled_inner(id, &mut input, min as f32, max as f32, unit);
        if response.changed {
            if let Ok(v) = input.text.parse::<f64>() {
                *value = v.clamp(min, max);
            }
        }
        response
    }

    fn slider_labeled_inner(
        &mut self,
        id: u64,
        input: &mut InputState,
        min: f32,
        max: f32,
        unit: &str,
    ) -> Response {
        // Extra height for value label above the track.
        let label_row_h = self.theme.font_size + 4.0;
        let total_h = self.theme.button_height + label_row_h;
        let rect = self.allocate_rect_keyed(id, self.region.w, total_h);

        // Slider occupies the bottom portion of the rect.
        let slider_rect = crate::layout::Rect::new(
            rect.x,
            rect.y + label_row_h,
            rect.w,
            self.theme.button_height,
        );

        self.register_widget(id, slider_rect, WidgetKind::Slider);
        let mut response = self.widget_response(id, slider_rect);
        let disabled = response.disabled;

        let current: f32 = input.text.parse().unwrap_or(min);
        let mut value = current.clamp(min, max);

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Slider,
            label: String::new(),
            value: Some(input.text.clone()),
            rect: slider_rect,
            focused: response.focused,
            disabled,
            expanded: None,
            selected: None,
            checked: None,
            value_range: Some((min, max, value)),
            children: Vec::new(),
        });

        // Click handling.
        if response.clicked {
            let track_x = slider_rect.x + self.theme.input_padding;
            let track_w = slider_rect.w - self.theme.input_padding * 2.0;
            let rel = ((self.state.mouse.x - track_x) / track_w).clamp(0.0, 1.0);
            value = min + rel * (max - min);
            let formatted = if (max - min) >= 10.0 {
                format!("{}", value.round() as i32)
            } else {
                format!("{:.1}", value)
            };
            input.text = formatted;
            input.cursor = input.text.len();
            response.changed = true;
        }

        // Keyboard.
        if response.focused && !disabled {
            use esox_input::{Key, NamedKey};
            for (event, _) in &self.state.keys {
                if !event.pressed {
                    continue;
                }
                let step = if (max - min) >= 10.0 {
                    1.0
                } else {
                    (max - min) / 20.0
                };
                match &event.key {
                    Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => {
                        value = (value - step).clamp(min, max);
                        let formatted = if (max - min) >= 10.0 {
                            format!("{}", value.round() as i32)
                        } else {
                            format!("{:.1}", value)
                        };
                        input.text = formatted;
                        input.cursor = input.text.len();
                        response.changed = true;
                    }
                    Key::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => {
                        value = (value + step).clamp(min, max);
                        let formatted = if (max - min) >= 10.0 {
                            format!("{}", value.round() as i32)
                        } else {
                            format!("{:.1}", value)
                        };
                        input.text = formatted;
                        input.cursor = input.text.len();
                        response.changed = true;
                    }
                    _ => {}
                }
            }
        }

        // Focus ring.
        if response.focused && !disabled {
            paint::draw_focus_ring(
                self.frame,
                slider_rect,
                self.theme.focus_ring_color,
                self.theme.corner_radius,
                self.theme.focus_ring_expand,
            );
        }

        // Background.
        let bg = if disabled {
            self.theme.disabled_bg
        } else {
            self.theme.bg_input
        };
        paint::draw_rounded_rect(self.frame, slider_rect, bg, self.theme.corner_radius);

        // Border.
        if disabled {
            paint::draw_dashed_border(
                self.frame,
                slider_rect,
                self.theme.disabled_border,
                self.theme.disabled_dash_len,
                self.theme.disabled_dash_gap,
                self.theme.disabled_dash_thickness,
            );
        } else {
            let border_color = if response.focused {
                self.theme.accent
            } else {
                self.theme.border
            };
            paint::draw_rounded_border(
                self.frame,
                slider_rect,
                border_color,
                self.theme.corner_radius,
            );
        }

        // Track area.
        let track_x = slider_rect.x + self.theme.input_padding;
        let track_y = slider_rect.y + slider_rect.h / 2.0 - 2.0;
        let track_w = slider_rect.w - self.theme.input_padding * 2.0;
        let track_h = self.theme.slider_track_height;

        self.frame.push(
            ShapeBuilder::rect(track_x, track_y, track_w, track_h)
                .color(self.theme.bg_raised)
                .build(),
        );

        // Filled portion.
        let t = if (max - min).abs() < f32::EPSILON {
            0.0
        } else {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        };
        let filled_w = track_w * t;
        let fill_color = if disabled {
            self.theme.disabled_fg
        } else {
            self.theme.accent
        };
        if filled_w > 0.0 {
            self.frame.push(
                ShapeBuilder::rect(track_x, track_y, filled_w, track_h)
                    .color(fill_color)
                    .build(),
            );
        }

        // Thumb.
        let thumb_x = track_x + filled_w;
        let thumb_r = 6.0;
        let thumb_color = if disabled {
            self.theme.disabled_fg
        } else if response.hovered || response.focused {
            self.theme.accent_hover
        } else {
            self.theme.accent
        };
        self.frame.push(
            ShapeBuilder::circle(thumb_x, slider_rect.y + slider_rect.h / 2.0, thumb_r)
                .color(thumb_color)
                .build(),
        );

        let fs = self.theme.font_size;
        let muted = self.theme.fg_muted;

        // Value + unit label centered above the thumb.
        let val_str = if input.text.is_empty() {
            format!("{}", min as i32)
        } else {
            input.text.clone()
        };
        let val_display = if unit.is_empty() {
            val_str
        } else {
            format!("{val_str} {unit}")
        };
        let val_w = self.text.measure_text(&val_display, fs);
        let val_x = (thumb_x - val_w / 2.0).clamp(rect.x, rect.x + rect.w - val_w);
        self.text.draw_text(
            &val_display,
            val_x,
            rect.y,
            fs,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );

        // Min label (left).
        let min_str = if (max - min) >= 10.0 {
            format!("{}", min as i32)
        } else {
            format!("{:.1}", min)
        };
        self.text.draw_text(
            &min_str,
            track_x,
            slider_rect.y + (slider_rect.h - fs) / 2.0,
            fs * 0.8,
            muted,
            self.frame,
            self.gpu,
            self.resources,
        );

        // Max label (right).
        let max_str = if (max - min) >= 10.0 {
            format!("{}", max as i32)
        } else {
            format!("{:.1}", max)
        };
        let max_w = self.text.measure_text(&max_str, fs * 0.8);
        self.text.draw_text(
            &max_str,
            track_x + track_w - max_w,
            slider_rect.y + (slider_rect.h - fs) / 2.0,
            fs * 0.8,
            muted,
            self.frame,
            self.gpu,
            self.resources,
        );

        response
    }
}
