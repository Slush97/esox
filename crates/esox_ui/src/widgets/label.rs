//! Label widgets — text display without interaction.

use esox_gfx::Color;

use crate::rich_text::RichText;
use crate::state::{A11yNode, A11yRole};
use crate::text::TruncationMode;
use crate::theme::{TextAlign, TextSize, TextTransform};
use crate::Ui;

/// Compute the x position for text given alignment, container origin, width, and text width.
fn align_text_x(align: TextAlign, rect_x: f32, rect_w: f32, text_w: f32) -> f32 {
    match align {
        TextAlign::Left => rect_x,
        TextAlign::Center => rect_x + (rect_w - text_w) * 0.5,
        TextAlign::Right => rect_x + rect_w - text_w,
    }
}

/// Apply a text transform to a string, returning a `Cow` to avoid allocation for `None`.
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
    /// Draw a label with the standard text color.
    pub fn label(&mut self, text: &str) {
        let font_size = self.resolve_font_size();
        let fg = self.resolve_fg();
        let align = self.resolve_text_align();
        let decoration = self.resolve_text_decoration();
        let transform = self.resolve_text_transform();
        let display = apply_transform(text, transform);
        let rect = self.allocate_rect(self.region.w, font_size + self.theme.label_pad_y);
        let x = align_text_x(
            align,
            rect.x,
            rect.w,
            self.text.measure_text(&display, font_size),
        );
        self.push_a11y_node(A11yNode {
            id: crate::id::fnv1a_runtime(text),
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
        self.text.draw_text_decorated(
            &display,
            x,
            rect.y,
            font_size,
            fg,
            decoration,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a label at a semantic text size.
    pub fn label_sized(&mut self, text: &str, size: TextSize) {
        let font_size = self.theme.resolve_text_size(size);
        let align = self.resolve_text_align();
        let rect = self.allocate_rect(self.region.w, font_size + self.theme.label_pad_y);
        let x = align_text_x(
            align,
            rect.x,
            rect.w,
            self.text.measure_text(text, font_size),
        );
        self.text.draw_text(
            text,
            x,
            rect.y,
            font_size,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a label with a custom color.
    pub fn label_colored(&mut self, text: &str, color: Color) {
        let font_size = self.theme.font_size;
        let align = self.resolve_text_align();
        let rect = self.allocate_rect(self.region.w, font_size + self.theme.label_pad_y);
        let x = align_text_x(
            align,
            rect.x,
            rect.w,
            self.text.measure_text(text, font_size),
        );
        self.text.draw_text(
            text,
            x,
            rect.y,
            font_size,
            color,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a heading (larger text).
    pub fn heading(&mut self, text: &str) {
        let font_size = self.theme.heading_font_size;
        let align = self.resolve_text_align();
        let rect = self.allocate_rect(self.region.w, self.theme.heading_height);
        let x = align_text_x(
            align,
            rect.x,
            rect.w,
            self.text.measure_text(text, font_size),
        );
        self.text.draw_text(
            text,
            x,
            rect.y,
            font_size,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a muted label (dimmer text).
    pub fn muted_label(&mut self, text: &str) {
        let font_size = self.theme.font_size;
        let align = self.resolve_text_align();
        let rect = self.allocate_rect(self.region.w, font_size + self.theme.label_pad_y);
        let x = align_text_x(
            align,
            rect.x,
            rect.w,
            self.text.measure_text(text, font_size),
        );
        self.text.draw_text(
            text,
            x,
            rect.y,
            font_size,
            self.theme.fg_muted,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a small header label (for categories).
    pub fn header_label(&mut self, text: &str) {
        let header_font_size = self.theme.header_font_size;
        let rect = self.allocate_rect(self.region.w, header_font_size + self.theme.label_pad_y);
        self.text.draw_header_text(
            text,
            rect.x,
            rect.y,
            self.theme.fg_muted,
            self.theme.header_letter_spacing,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a word-wrapped label. Height varies based on content.
    pub fn label_wrapped(&mut self, text: &str) {
        let size = self.theme.font_size;
        let align = self.resolve_text_align();
        let max_width = self.region.w;
        let line_height = self.text.line_height(size);
        let line_spacing = self.theme.line_spacing;
        let (_, measured_h) = self
            .text
            .measure_text_wrapped(text, size, max_width, line_spacing);
        let total_height = measured_h + self.theme.label_pad_y;
        let rect = self.allocate_rect(max_width, total_height);

        let lines = self.text.wrap_lines(text, size, max_width);
        let step = line_height + line_spacing;
        for (i, &(start, end)) in lines.iter().enumerate() {
            let line = &text[start..end].trim_start();
            let line_w = self.text.measure_text(line, size);
            let x = align_text_x(align, rect.x, rect.w, line_w);
            self.text.draw_text(
                line,
                x,
                rect.y + i as f32 * step,
                size,
                self.theme.fg,
                self.frame,
                self.gpu,
                self.resources,
            );
        }
    }

    /// Draw a single-line label truncated with "…" if it overflows.
    pub fn label_truncated(&mut self, text: &str) {
        let rect = self.allocate_rect(self.region.w, self.theme.font_size + self.theme.label_pad_y);
        self.text.draw_text_truncated(
            text,
            rect.x,
            rect.y,
            self.theme.font_size,
            rect.w,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a single-line label truncated with a specific truncation mode.
    pub fn label_truncated_mode(&mut self, text: &str, mode: TruncationMode) {
        let rect = self.allocate_rect(self.region.w, self.theme.font_size + self.theme.label_pad_y);
        self.text.draw_text_truncated_mode(
            text,
            rect.x,
            rect.y,
            self.theme.font_size,
            rect.w,
            self.theme.fg,
            mode,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw a single-line rich text label with styled spans.
    pub fn rich_label(&mut self, rich: &RichText<'_>) {
        let font_size = self.theme.font_size;
        let fg = self.theme.fg;

        // Measure total width.
        let mut total_w = 0.0f32;
        for span in &rich.spans {
            let size = span.size.unwrap_or(font_size);
            total_w += self.text.measure_text(span.text, size);
        }

        let rect = self.allocate_rect(
            total_w.min(self.region.w),
            font_size + self.theme.label_pad_y,
        );

        let mut pen_x = rect.x;
        for span in &rich.spans {
            let size = span.size.unwrap_or(font_size);
            let color = span.color.unwrap_or(fg);
            let style: u8 = if span.weight.is_some_and(|w| w.needs_faux_bold()) || span.bold {
                1
            } else {
                0
            };
            let ls = span.letter_spacing.unwrap_or(0.0);
            let advance = if ls != 0.0 {
                self.text.draw_text_spaced(
                    span.text,
                    pen_x,
                    rect.y,
                    size,
                    color,
                    ls,
                    self.frame,
                    self.gpu,
                    self.resources,
                )
            } else {
                self.text.draw_text_styled(
                    span.text,
                    pen_x,
                    rect.y,
                    size,
                    color,
                    style,
                    self.frame,
                    self.gpu,
                    self.resources,
                )
            };
            pen_x += advance;
        }
    }

    /// Draw a word-wrapped rich text label. Height varies based on content.
    pub fn rich_label_wrapped(&mut self, rich: &RichText<'_>) {
        let font_size = self.theme.font_size;
        let fg = self.theme.fg;
        let max_width = self.region.w;
        let line_height = self.text.line_height(font_size);

        // Flatten spans into words with style info.
        struct StyledWord<'a> {
            text: &'a str,
            color: Option<Color>,
            bold: bool,
            size: Option<f32>,
            width: f32,
            weight: Option<esox_font::FontWeight>,
        }

        let space_width = self.text.measure_text(" ", font_size);
        let mut words: Vec<StyledWord<'_>> = Vec::new();

        for span in &rich.spans {
            let size = span.size.unwrap_or(font_size);
            // Split span into words.
            for word in span.text.split_whitespace() {
                let w = self.text.measure_text(word, size);
                words.push(StyledWord {
                    text: word,
                    color: span.color,
                    bold: span.bold,
                    size: span.size,
                    width: w,
                    weight: span.weight,
                });
            }
        }

        // Break into lines.
        struct LinePlan {
            start: usize,
            end: usize, // exclusive
        }

        let mut lines: Vec<LinePlan> = Vec::new();
        let mut line_start = 0;
        let mut line_w = 0.0f32;

        for (i, word) in words.iter().enumerate() {
            if line_w == 0.0 {
                line_w = word.width;
            } else if line_w + space_width + word.width <= max_width {
                line_w += space_width + word.width;
            } else {
                lines.push(LinePlan {
                    start: line_start,
                    end: i,
                });
                line_start = i;
                line_w = word.width;
            }
        }
        if line_start < words.len() || words.is_empty() {
            lines.push(LinePlan {
                start: line_start,
                end: words.len(),
            });
        }

        let total_height = lines.len() as f32 * line_height + self.theme.label_pad_y;
        let rect = self.allocate_rect(max_width, total_height);

        for (line_idx, line) in lines.iter().enumerate() {
            let mut pen_x = rect.x;
            let pen_y = rect.y + line_idx as f32 * line_height;

            for (word_idx, word) in words.iter().enumerate().take(line.end).skip(line.start) {
                if word_idx > line.start {
                    pen_x += space_width;
                }
                let size = word.size.unwrap_or(font_size);
                let color = word.color.unwrap_or(fg);
                let style: u8 = if word.weight.is_some_and(|w| w.needs_faux_bold()) || word.bold {
                    1
                } else {
                    0
                };
                let advance = self.text.draw_text_styled(
                    word.text,
                    pen_x,
                    pen_y,
                    size,
                    color,
                    style,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
                pen_x += advance;
            }
        }
    }
}
