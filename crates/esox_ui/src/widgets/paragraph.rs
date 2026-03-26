//! Paragraph widget — word-wrapped, styled, hover-aware text block.

use crate::response::Response;
use crate::state::{A11yNode, A11yRole};
use crate::theme::{TextAlign, TextTransform};
use crate::Ui;

/// Apply a text transform (same logic as label.rs).
fn apply_transform<'a>(text: &'a str, transform: TextTransform) -> std::borrow::Cow<'a, str> {
    match transform {
        TextTransform::None => std::borrow::Cow::Borrowed(text),
        TextTransform::Uppercase => std::borrow::Cow::Owned(text.to_uppercase()),
        TextTransform::Lowercase => std::borrow::Cow::Owned(text.to_lowercase()),
        TextTransform::Capitalize => {
            let mut result = String::with_capacity(text.len());
            let mut capitalize_next = true;
            for c in text.chars() {
                if capitalize_next && c.is_alphabetic() {
                    result.extend(c.to_uppercase());
                    capitalize_next = false;
                } else {
                    result.push(c);
                    if c.is_whitespace() {
                        capitalize_next = true;
                    }
                }
            }
            std::borrow::Cow::Owned(result)
        }
    }
}

impl<'f> Ui<'f> {
    /// Draw a paragraph of wrapped text. Returns a `Response` with hover state.
    ///
    /// Measures text via `measure_text_wrapped`, allocates a rect, draws
    /// wrapped lines with `theme.line_spacing`, and registers for hit testing.
    pub fn paragraph(&mut self, id: u64, text: &str) -> Response {
        let size = self.resolve_font_size();
        let fg = self.resolve_fg();
        let align = self.resolve_text_align();
        let decoration = self.resolve_text_decoration();
        let transform = self.resolve_text_transform();
        let display = apply_transform(text, transform);
        let max_width = self.region.w;
        let line_spacing = self.theme.line_spacing;
        let line_height = self.text.line_height(size);

        let (_, measured_h) =
            self.text
                .measure_text_wrapped(&display, size, max_width, line_spacing);
        let total_height = measured_h + self.theme.label_pad_y;
        let rect = self.allocate_rect_keyed(id, max_width, total_height);

        let lines = self.text.wrap_lines(&display, size, max_width);
        let step = line_height + line_spacing;
        for (i, &(start, end)) in lines.iter().enumerate() {
            let line = &display[start..end].trim_start();
            let line_w = self.text.measure_text(line, size);
            let x = match align {
                TextAlign::Left => rect.x,
                TextAlign::Center => rect.x + (rect.w - line_w) * 0.5,
                TextAlign::Right => rect.x + rect.w - line_w,
            };
            self.text.draw_text_decorated(
                line,
                x,
                rect.y + i as f32 * step,
                size,
                fg,
                decoration,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        let hovered = self.is_hovered(rect);

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Label,
            label: text.to_string(),
            value: None,
            rect,
            focused: false,
            disabled: false,
            expanded: None,
            selected: None,
            checked: None,
            value_range: None,
            children: Vec::new(),
        });

        Response {
            clicked: false,
            right_clicked: false,
            hovered,
            pressed: false,
            focused: false,
            changed: false,
            disabled: false,
        }
    }
}
