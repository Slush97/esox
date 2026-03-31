//! Pagination widget — page navigation controls for paged data.
//!
//! # Examples
//!
//! ```ignore
//! let mut state = PaginationState::new();
//! if ui.pagination(id!("pages"), &mut state, 10).changed {
//!     load_page(state.current_page);
//! }
//! ```

use esox_gfx::Color;

use crate::id::{HOVER_SALT, PRESS_SALT};
use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, WidgetKind};
use crate::Ui;

/// Persistent state for the pagination widget.
#[derive(Debug, Clone)]
pub struct PaginationState {
    /// Zero-indexed current page.
    pub current_page: usize,
}

impl PaginationState {
    pub fn new() -> Self {
        Self { current_page: 0 }
    }
}

impl Default for PaginationState {
    fn default() -> Self {
        Self::new()
    }
}

impl<'f> Ui<'f> {
    /// Draw pagination controls for `total_pages` pages.
    ///
    /// Renders: `<< < [page numbers] > >>` with ellipsis for large page counts.
    /// Returns a Response with `changed = true` when the page changes.
    #[allow(deprecated)]
    pub fn pagination(
        &mut self,
        id: u64,
        current_page: &mut usize,
        total_pages: usize,
    ) -> Response {
        let mut state = PaginationState {
            current_page: *current_page,
        };
        let response = self.pagination_state(id, &mut state, total_pages);
        *current_page = state.current_page;
        response
    }

    /// Draw pagination controls using `PaginationState`.
    #[deprecated(note = "use pagination() with &mut usize instead")]
    pub fn pagination_state(
        &mut self,
        id: u64,
        state: &mut PaginationState,
        total_pages: usize,
    ) -> Response {
        let mut response = Response::default();
        if total_pages == 0 {
            return response;
        }

        state.current_page = state.current_page.min(total_pages - 1);
        let cur = state.current_page;

        self.row(|ui| {
            // First page.
            if ui.page_button(id ^ 0xF001, "\u{00AB}", cur > 0).clicked {
                state.current_page = 0;
                response.changed = true;
            }
            // Previous.
            if ui.page_button(id ^ 0xF002, "<", cur > 0).clicked {
                state.current_page = cur.saturating_sub(1);
                response.changed = true;
            }

            // Page numbers.
            let pages = Self::page_window(cur, total_pages);
            for entry in pages {
                match entry {
                    PageEntry::Page(p) => {
                        let label = format!("{}", p + 1); // 1-indexed display
                        let is_current = p == cur;
                        let btn_id = id ^ (p as u64 + 1).wrapping_mul(crate::id::PAGE_BUTTON_SALT);
                        if ui.page_number_button(btn_id, &label, is_current).clicked && !is_current
                        {
                            state.current_page = p;
                            response.changed = true;
                        }
                    }
                    PageEntry::Ellipsis => {
                        ui.muted_label("\u{2026}");
                    }
                }
            }

            // Next.
            if ui
                .page_button(id ^ 0xF003, ">", cur + 1 < total_pages)
                .clicked
            {
                state.current_page = cur + 1;
                response.changed = true;
            }
            // Last page.
            if ui
                .page_button(id ^ 0xF004, "\u{00BB}", cur + 1 < total_pages)
                .clicked
            {
                state.current_page = total_pages - 1;
                response.changed = true;
            }
        });

        response
    }

    /// Compute which page numbers to display, with ellipsis for large counts.
    fn page_window(current: usize, total: usize) -> Vec<PageEntry> {
        if total <= 7 {
            return (0..total).map(PageEntry::Page).collect();
        }

        let mut pages = Vec::with_capacity(9);
        pages.push(PageEntry::Page(0));

        let start = current.saturating_sub(1).max(1);
        let end = (current + 2).min(total - 1);

        if start > 1 {
            pages.push(PageEntry::Ellipsis);
        }

        for p in start..end {
            pages.push(PageEntry::Page(p));
        }

        if end < total - 1 {
            pages.push(PageEntry::Ellipsis);
        }

        pages.push(PageEntry::Page(total - 1));

        pages
    }

    /// Small navigation button (<<, <, >, >>).
    fn page_button(&mut self, id: u64, label: &str, enabled: bool) -> Response {
        let fs = self.theme.font_size;
        let label_w = self.text.measure_text(label, fs);
        let btn_w = (label_w + self.theme.input_padding * 2.0).max(self.theme.small_button_min_w);
        let btn_h = self.theme.small_button_height;
        let rect = self.allocate_rect_keyed(id, btn_w, btn_h);
        self.register_widget(id, rect, WidgetKind::Button);

        let response = self.widget_response(id, rect);
        let disabled = !enabled;

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Button,
            label: label.to_string(),
            value: None,
            rect,
            focused: response.focused,
            disabled,
            expanded: None,
            selected: None,
            checked: None,
            value_range: None,
            children: Vec::new(),
        });

        let hover_t = if disabled {
            0.0
        } else {
            self.state.hover_t(
                id ^ HOVER_SALT,
                response.hovered,
                self.theme.hover_duration_ms,
            )
        };

        // Subtle hover fill.
        if hover_t > 0.0 {
            let fill = Color::new(
                self.theme.accent.r,
                self.theme.accent.g,
                self.theme.accent.b,
                0.10 * hover_t,
            );
            paint::draw_rounded_rect(self.frame, rect, fill, self.theme.corner_radius);
        }

        let text_color = if disabled {
            self.theme.disabled_fg
        } else {
            paint::lerp_color(self.theme.fg_muted, self.theme.accent, hover_t)
        };
        self.text.draw_ui_text(
            label,
            rect.x + (rect.w - label_w) / 2.0,
            rect.y + (rect.h - fs) / 2.0,
            text_color,
            self.frame,
            self.gpu,
            self.resources,
        );

        response
    }

    /// Page number button — filled accent background when current.
    fn page_number_button(&mut self, id: u64, label: &str, is_current: bool) -> Response {
        let fs = self.theme.font_size;
        let label_w = self.text.measure_text(label, fs);
        let btn_size =
            (label_w + self.theme.input_padding * 2.0).max(self.theme.small_button_height); // at least square
        let rect = self.allocate_rect_keyed(id, btn_size, self.theme.small_button_height);
        self.register_widget(id, rect, WidgetKind::Button);

        let response = self.widget_response(id, rect);

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Button,
            label: label.to_string(),
            value: None,
            rect,
            focused: response.focused,
            disabled: false,
            expanded: None,
            selected: Some(is_current),
            checked: None,
            value_range: None,
            children: Vec::new(),
        });

        let press_t = self.state.hover_t(
            id ^ PRESS_SALT,
            response.pressed,
            self.theme.press_duration_ms,
        );

        if is_current {
            // Filled accent background.
            let mut bg = self.theme.accent;
            if press_t > 0.0 {
                let d = self.theme.press_darken * press_t;
                bg = Color::new(bg.r * (1.0 - d), bg.g * (1.0 - d), bg.b * (1.0 - d), bg.a);
            }
            paint::draw_rounded_rect(self.frame, rect, bg, self.theme.corner_radius);

            let text_color = self.theme.fg_on_accent;
            self.text.draw_ui_text(
                label,
                rect.x + (rect.w - label_w) / 2.0,
                rect.y + (rect.h - fs) / 2.0,
                text_color,
                self.frame,
                self.gpu,
                self.resources,
            );
        } else {
            // Hover effect.
            let hover_t = self.state.hover_t(
                id ^ HOVER_SALT,
                response.hovered,
                self.theme.hover_duration_ms,
            );
            if hover_t > 0.0 {
                let fill = Color::new(
                    self.theme.accent.r,
                    self.theme.accent.g,
                    self.theme.accent.b,
                    0.10 * hover_t,
                );
                paint::draw_rounded_rect(self.frame, rect, fill, self.theme.corner_radius);
            }

            let text_color = paint::lerp_color(self.theme.fg, self.theme.accent, hover_t);
            self.text.draw_ui_text(
                label,
                rect.x + (rect.w - label_w) / 2.0,
                rect.y + (rect.h - fs) / 2.0,
                text_color,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        response
    }
}

enum PageEntry {
    Page(usize),
    Ellipsis,
}
