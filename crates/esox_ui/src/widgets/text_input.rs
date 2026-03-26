//! Text input widget — single-line with cursor, selection, scroll.
//!
//! # Examples
//!
//! ```ignore
//! let response = ui.text_input(id!("name"), &mut name_state, "Enter name…");
//! if response.changed {
//!     validate_name(&name_state.text);
//! }
//! ```

use esox_gfx::ShapeBuilder;
use esox_input::{Key, NamedKey};

use crate::layout::Rect;
use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, InputState, WidgetKind};
use crate::widgets::form::FieldStatus;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a text input field. The `InputState` is app-owned.
    pub fn text_input(&mut self, id: u64, input: &mut InputState, placeholder: &str) -> Response {
        self.text_input_inner(id, input, placeholder, None)
    }

    /// Draw a text input with a validation border color.
    pub fn text_input_validated(
        &mut self,
        id: u64,
        input: &mut InputState,
        placeholder: &str,
        status: FieldStatus,
    ) -> Response {
        self.text_input_inner(id, input, placeholder, Some(status))
    }

    fn text_input_inner(
        &mut self,
        id: u64,
        input: &mut InputState,
        placeholder: &str,
        status: Option<FieldStatus>,
    ) -> Response {
        let rect = self.allocate_rect_keyed(id, self.region.w, self.theme.button_height);
        self.register_widget(id, rect, WidgetKind::TextInput);

        let mut response = self.widget_response(id, rect);
        let disabled = response.disabled;

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::TextInput,
            label: placeholder.to_string(),
            value: Some(input.text.clone()),
            rect,
            focused: response.focused,
            disabled,
            expanded: None,
            selected: None,
            checked: None,
            value_range: None,
            children: Vec::new(),
        });

        let fs = self.theme.font_size;

        if disabled {
            // ── Disabled draw ──
            paint::draw_rounded_rect(
                self.frame,
                rect,
                self.theme.disabled_bg,
                self.theme.corner_radius,
            );
            paint::draw_dashed_border(
                self.frame,
                rect,
                self.theme.disabled_border,
                self.theme.disabled_dash_len,
                self.theme.disabled_dash_gap,
                self.theme.disabled_dash_thickness,
            );
            let text_x = rect.x + self.theme.input_padding;
            let text_y = rect.y + (rect.h - fs) / 2.0;
            if input.text.is_empty() {
                self.text.draw_text(
                    placeholder,
                    text_x,
                    text_y,
                    fs,
                    self.theme.disabled_fg,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            } else {
                self.text.draw_text(
                    &input.text,
                    text_x,
                    text_y,
                    fs,
                    self.theme.disabled_fg,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            }
            return response;
        }

        // Handle click — place cursor.
        if response.clicked {
            let click_x = self.state.mouse.x;
            input.cursor = x_to_cursor(
                input,
                self.text,
                rect,
                click_x,
                self.theme.font_size,
                self.theme.input_padding,
            );
            input.selection = None;
        }

        // Consume IME committed text.
        if response.focused {
            if let Some(committed) = self.state.ime.committed.take() {
                input.save_undo();
                input.insert_str(&committed);
                response.changed = true;
                self.state.reset_blink();
            }
        }

        // Process buffered keys when focused.
        if response.focused {
            let keys: Vec<_> = self.state.keys.clone();
            for (event, modifiers) in &keys {
                if !event.pressed {
                    continue;
                }
                let ctrl = modifiers.ctrl();
                let shift = modifiers.shift();
                // Handle clipboard shortcuts.
                if ctrl {
                    if let Key::Character(ch) = &event.key {
                        match ch.as_str() {
                            "c" => {
                                if let (Some(sel_text), Some(clip)) =
                                    (input.selected_text(), &self.state.clipboard)
                                {
                                    clip.write_text(sel_text);
                                }
                                continue;
                            }
                            "x" => {
                                if let Some(clip) = &self.state.clipboard {
                                    if let Some(sel_text) = input.selected_text() {
                                        clip.write_text(sel_text);
                                    }
                                    input.save_undo();
                                    input.delete_selection();
                                    response.changed = true;
                                    self.state.reset_blink();
                                }
                                continue;
                            }
                            "v" => {
                                if let Some(clip) = &self.state.clipboard {
                                    if let Some(text) = clip.read_text() {
                                        input.save_undo();
                                        input.insert_str(&text);
                                        response.changed = true;
                                        self.state.reset_blink();
                                    }
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                let changed = process_text_key(input, &event.key, ctrl, shift);
                if changed {
                    response.changed = true;
                    self.state.reset_blink();
                }
            }

            // Update scroll offset.
            let inner_w = rect.w - self.theme.input_padding * 2.0;
            update_scroll(input, self.text, inner_w, self.theme.font_size);
        }

        // ── Draw ──

        // Focus ring.
        if response.focused {
            paint::draw_focus_ring(
                self.frame,
                rect,
                self.theme.focus_ring_color,
                self.theme.corner_radius,
                self.theme.focus_ring_expand, // smaller expand for text inputs
            );
        }

        // Background.
        paint::draw_rounded_rect(
            self.frame,
            rect,
            self.theme.bg_input,
            self.theme.corner_radius,
        );

        // Border — validation status takes precedence, then animated focus, then default.
        let focus_t = self.state.hover_t(
            id ^ crate::id::FOCUS_SALT,
            response.focused,
            self.theme.hover_duration_ms,
        );
        let border_color = match status {
            Some(FieldStatus::Error) => self.theme.red,
            Some(FieldStatus::Success) => self.theme.green,
            Some(FieldStatus::Warning) => self.theme.amber,
            _ => paint::lerp_color(self.theme.border, self.theme.accent, focus_t),
        };
        paint::draw_rounded_border(self.frame, rect, border_color, self.theme.corner_radius);

        let text_x = rect.x + self.theme.input_padding;
        let text_y = rect.y + (rect.h - fs) / 2.0;
        let inner_w = rect.w - self.theme.input_padding * 2.0;

        if input.text.is_empty() && !response.focused {
            // Placeholder.
            self.text.draw_text(
                placeholder,
                text_x,
                text_y,
                fs,
                self.theme.fg_dim,
                self.frame,
                self.gpu,
                self.resources,
            );
            return response;
        }

        let scroll = input.scroll_offset;

        // Selection highlight.
        if let Some((sel_start, sel_end)) = input.selection {
            let sel_x0 = self.text.measure_cursor_x(&input.text, fs, sel_start) - scroll;
            let sel_x1 = self.text.measure_cursor_x(&input.text, fs, sel_end) - scroll;
            let sel_left = sel_x0.max(0.0);
            let sel_right = sel_x1.min(inner_w);
            if sel_right > sel_left {
                self.frame.push(
                    ShapeBuilder::rect(
                        text_x + sel_left,
                        rect.y + self.theme.label_pad_y,
                        sel_right - sel_left,
                        rect.h - self.theme.label_pad_y * 2.0,
                    )
                    .color(self.theme.accent_dim)
                    .build(),
                );
            }
        }

        // Text content — use error color when validation fails.
        let text_color = match status {
            Some(FieldStatus::Error) => self.theme.red,
            _ => self.theme.fg,
        };
        if !input.text.is_empty() {
            self.text.draw_text(
                &input.text,
                text_x - scroll,
                text_y,
                fs,
                text_color,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        // IME preedit rendering.
        if response.focused && !self.state.ime.preedit.is_empty() {
            let cursor_x_in_text = self.text.measure_cursor_x(&input.text, fs, input.cursor);
            let preedit_x = text_x + cursor_x_in_text - scroll;
            let preedit_w = self.text.measure_text(&self.state.ime.preedit, fs);
            // Underline.
            self.frame.push(
                ShapeBuilder::rect(
                    preedit_x,
                    rect.y + rect.h - self.theme.label_pad_y - 1.0,
                    preedit_w,
                    1.0,
                )
                .color(self.theme.fg_dim)
                .build(),
            );
            // Preedit text.
            self.text.draw_text(
                &self.state.ime.preedit,
                preedit_x,
                text_y,
                fs,
                self.theme.fg_dim,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        // Cursor.
        if response.focused && self.state.cursor_blink {
            let cursor_x_in_text = self.text.measure_cursor_x(&input.text, fs, input.cursor);
            let cx = text_x + cursor_x_in_text - scroll;
            // Offset cursor past preedit if active.
            let preedit_offset = if !self.state.ime.preedit.is_empty() {
                self.text.measure_text(&self.state.ime.preedit, fs)
            } else {
                0.0
            };
            let cx = cx + preedit_offset;
            if cx >= text_x - 1.0 && cx <= text_x + inner_w + 1.0 {
                let line_h = self.text.line_height(fs);
                let cursor_top = rect.y + (rect.h - line_h) / 2.0;
                self.frame.push(
                    ShapeBuilder::rect(cx, cursor_top, self.theme.cursor_width, line_h)
                        .color(self.theme.fg)
                        .build(),
                );
            }
        }

        response
    }
}

/// Process a key event for text input. Returns true if the input was modified.
fn process_text_key(input: &mut InputState, key: &Key, ctrl: bool, shift: bool) -> bool {
    match key {
        Key::Named(NamedKey::Backspace) => {
            input.save_undo();
            if ctrl {
                input.delete_word_back();
            } else {
                input.delete_back();
            }
            true
        }
        Key::Named(NamedKey::Delete) => {
            input.save_undo();
            if ctrl {
                input.delete_word_forward();
            } else {
                input.delete_forward();
            }
            true
        }
        Key::Named(NamedKey::ArrowLeft) => {
            if ctrl && shift {
                input.move_word_left_extend();
            } else if ctrl {
                input.move_word_left();
            } else if shift {
                input.move_left_extend();
            } else {
                input.move_left();
            }
            true
        }
        Key::Named(NamedKey::ArrowRight) => {
            if ctrl && shift {
                input.move_word_right_extend();
            } else if ctrl {
                input.move_word_right();
            } else if shift {
                input.move_right_extend();
            } else {
                input.move_right();
            }
            true
        }
        Key::Named(NamedKey::Home) => {
            if shift {
                input.home_extend();
            } else {
                input.home();
            }
            true
        }
        Key::Named(NamedKey::End) => {
            if shift {
                input.end_extend();
            } else {
                input.end();
            }
            true
        }
        Key::Named(NamedKey::Space) => {
            input.save_undo();
            input.insert_char(' ');
            true
        }
        Key::Character(ch) if ctrl && ch.as_str() == "a" => {
            input.select_all();
            true
        }
        Key::Character(ch) if ctrl && ch.as_str() == "z" => {
            if shift {
                input.redo();
            } else {
                input.undo();
            }
            true
        }
        Key::Character(ch) if ctrl && ch.as_str() == "Z" => {
            input.redo();
            true
        }
        Key::Character(_) if ctrl => {
            // Ctrl+C, Ctrl+V, etc. — handled by clipboard layer.
            false
        }
        Key::Character(ch) => {
            input.save_undo();
            for c in ch.chars() {
                if !c.is_control() {
                    input.insert_char(c);
                }
            }
            true
        }
        _ => false,
    }
}

/// Compute scroll offset so the cursor stays visible.
fn update_scroll(
    input: &mut InputState,
    text: &mut crate::text::TextRenderer,
    inner_w: f32,
    font_size: f32,
) {
    let cursor_x = text.measure_cursor_x(&input.text, font_size, input.cursor);
    if cursor_x - input.scroll_offset > inner_w {
        input.scroll_offset = cursor_x - inner_w;
    }
    if cursor_x < input.scroll_offset {
        input.scroll_offset = cursor_x;
    }
    if input.scroll_offset < 0.0 {
        input.scroll_offset = 0.0;
    }
}

/// Map a click x-coordinate to a cursor byte position.
/// Uses `x_to_byte_offset` to walk cached shaped glyphs in O(glyphs)
/// instead of calling `measure_text` per character boundary.
fn x_to_cursor(
    input: &InputState,
    text: &mut crate::text::TextRenderer,
    rect: Rect,
    click_x: f32,
    font_size: f32,
    input_padding: f32,
) -> usize {
    let text_x = rect.x + input_padding;
    let rel_x = click_x - text_x + input.scroll_offset;
    text.x_to_byte_offset(&input.text, font_size, rel_x)
}
