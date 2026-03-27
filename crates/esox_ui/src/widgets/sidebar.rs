//! Sidebar widget — fixed header, scrollable body, fixed footer.
//!
//! A composable sidebar layout for navigation panels. The sidebar manages
//! a three-zone layout (header, scrollable body, footer) with proper
//! surface styling and consistent spacing.
//!
//! # Examples
//!
//! ```ignore
//! ui.sidebar(id!("nav"), |sb| {
//!     sb.header(|ui| {
//!         ui.rich_label(&brand_text);
//!         ui.spacer();
//!         ui.status_pill_success("Online");
//!     });
//!
//!     sb.section("CHANNELS", |ui| {
//!         if ui.sidebar_item(id!("ch1"), "general").selected(true).show().clicked {
//!             current = "general";
//!         }
//!         if ui.sidebar_item(id!("ch2"), "random").show().clicked {
//!             current = "random";
//!         }
//!     });
//!
//!     sb.footer(|ui| {
//!         ui.avatar("AB", 28.0);
//!         ui.label("username");
//!     });
//! });
//! ```

use esox_gfx::Color;

use crate::id::{fnv1a_mix, HOVER_SALT};
use crate::layout::Rect;
use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, WidgetKind};
use crate::theme::SpacingScale;
use crate::Ui;

// ── Sidebar container ─────────────────────────────────────────────────

/// Builder for the sidebar layout. Passed to the closure in `ui.sidebar()`.
pub struct SidebarBuilder<'a, 'f> {
    ui: &'a mut Ui<'f>,
}

impl<'a, 'f> SidebarBuilder<'a, 'f> {
    /// Draw a fixed header area at the top of the sidebar.
    pub fn header(&mut self, f: impl FnOnce(&mut Ui<'f>)) {
        self.ui.padding(SpacingScale::Lg, f);
        self.ui.separator();
    }

    /// Draw a labeled section within the sidebar body.
    /// The section header is rendered as a small uppercase label.
    pub fn section(&mut self, label: &str, f: impl FnOnce(&mut Ui<'f>)) {
        self.ui.add_space(self.ui.theme().content_spacing);
        self.ui
            .padding(SpacingScale::Lg, |ui| ui.header_label(label));
        self.ui.add_space(self.ui.theme().spacing_unit * 0.5);
        f(self.ui);
    }

    /// Draw a fixed footer area at the bottom of the sidebar.
    /// Call this last — it renders a separator then the footer content.
    pub fn footer(&mut self, f: impl FnOnce(&mut Ui<'f>)) {
        self.ui.separator();
        self.ui.padding(SpacingScale::Md, |ui| {
            ui.row_spaced(ui.theme().content_spacing, f);
        });
    }
}

impl<'f> Ui<'f> {
    /// Draw a sidebar with header, scrollable body, and footer zones.
    ///
    /// The sidebar uses a `surface` background and manages layout spacing
    /// automatically. Use the `SidebarBuilder` methods inside the closure
    /// to define the header, sections, and footer.
    pub fn sidebar(&mut self, _id: u64, f: impl FnOnce(&mut SidebarBuilder<'_, 'f>)) {
        // Draw surface background for the full sidebar region.
        let bg_rect = Rect::new(
            self.region.x - self.theme.spacing_unit,
            self.region.y,
            self.region.w + self.theme.spacing_unit * 2.0,
            self.region.h,
        );
        paint::draw_rounded_rect(self.frame, bg_rect, self.theme.bg_surface, 0.0);

        let mut builder = SidebarBuilder { ui: self };
        f(&mut builder);
    }

    /// Draw a sidebar with a scrollable body section between header and footer.
    ///
    /// This is the full-featured version: header and footer are fixed, the
    /// middle section scrolls. Use `sidebar_begin` / `sidebar_end` for the
    /// non-scrolling variant if you manage scrolling yourself.
    pub fn sidebar_scrollable(
        &mut self,
        id: u64,
        header: impl FnOnce(&mut Ui<'f>),
        body: impl FnOnce(&mut Ui<'f>),
        footer: impl FnOnce(&mut Ui<'f>),
    ) {
        self.surface(|ui| {
            // Fixed header.
            ui.padding(SpacingScale::Lg, |ui| {
                ui.row(header);
            });
            ui.separator();

            // Scrollable body.
            let scroll_id = fnv1a_mix(id, 0x5CDE_BA11);
            ui.scrollable_fill(scroll_id, body);

            // Fixed footer.
            ui.separator();
            ui.padding(SpacingScale::Md, |ui| {
                ui.row_spaced(ui.theme().content_spacing, footer);
            });
        });
    }
}

// ── Sidebar item ──────────────────────────────────────────────────────

/// A clickable item in a sidebar list. Supports selected state, optional
/// avatar/prefix, and optional badge count.
pub struct SidebarItemBuilder<'a> {
    id: u64,
    label: &'a str,
    prefix: Option<&'a str>,
    badge: Option<u32>,
    selected: bool,
    muted: bool,
}

impl<'a> SidebarItemBuilder<'a> {
    /// Set this item as selected (highlighted background).
    pub fn selected(mut self, sel: bool) -> Self {
        self.selected = sel;
        self
    }

    /// Add a short prefix string (rendered in dim color before the label).
    pub fn prefix(mut self, p: &'a str) -> Self {
        self.prefix = Some(p);
        self
    }

    /// Add a badge count on the right side.
    pub fn badge(mut self, count: u32) -> Self {
        self.badge = Some(count);
        self
    }

    /// Use muted text color (for non-selected items).
    pub fn muted(mut self, m: bool) -> Self {
        self.muted = m;
        self
    }

    /// Draw the sidebar item and return a Response.
    pub fn show(self, ui: &mut Ui<'_>) -> Response {
        ui.draw_sidebar_item(self)
    }
}

impl<'f> Ui<'f> {
    /// Create a sidebar item builder. Call `.show(ui)` to render it.
    pub fn sidebar_item<'a>(&self, id: u64, label: &'a str) -> SidebarItemBuilder<'a> {
        SidebarItemBuilder {
            id,
            label,
            prefix: None,
            badge: None,
            selected: false,
            muted: false,
        }
    }

    /// Internal: draw a sidebar item from its builder.
    fn draw_sidebar_item(&mut self, item: SidebarItemBuilder<'_>) -> Response {
        let pad_h = self.theme.space(SpacingScale::Lg);
        let pad_v = self.theme.space(SpacingScale::Sm);
        let font_size = self.theme.font_size;
        let item_h = font_size + pad_v * 2.0;

        let rect = self.allocate_rect_keyed(item.id, self.region.w, item_h);
        self.register_widget(item.id, rect, WidgetKind::Button);
        let response = self.widget_response(item.id, rect);

        self.push_a11y_node(A11yNode {
            id: item.id,
            role: A11yRole::Button,
            label: item.label.to_string(),
            value: None,
            rect,
            focused: response.focused,
            disabled: response.disabled,
            expanded: None,
            selected: Some(item.selected),
            checked: None,
            value_range: None,
            children: Vec::new(),
        });

        // Background: selected or hover.
        if item.selected {
            paint::draw_rounded_rect(self.frame, rect, self.theme.accent_dim, 0.0);
        } else {
            let hover_t = self.state.hover_t(
                item.id ^ HOVER_SALT,
                response.hovered,
                self.theme.hover_duration_ms,
            );
            if hover_t > 0.01 {
                let hover_bg = Color::new(
                    self.theme.fg.r,
                    self.theme.fg.g,
                    self.theme.fg.b,
                    0.05 * hover_t,
                );
                paint::draw_rounded_rect(self.frame, rect, hover_bg, 0.0);
            }
        }

        // Text content.
        let text_y = rect.y + (rect.h - font_size) / 2.0;
        let mut text_x = rect.x + pad_h;

        // Prefix (e.g., "#" for channels).
        if let Some(prefix) = item.prefix {
            let prefix_color = if item.selected {
                self.theme.accent
            } else {
                self.theme.fg_dim
            };
            let pw = self.text.draw_text(
                prefix,
                text_x,
                text_y,
                font_size,
                prefix_color,
                self.frame,
                self.gpu,
                self.resources,
            );
            text_x += pw + self.theme.spacing_unit;
        }

        // Label.
        let label_color = if item.selected {
            self.theme.accent
        } else if item.muted {
            self.theme.fg_muted
        } else {
            self.theme.fg
        };
        self.text.draw_ui_text(
            item.label,
            text_x,
            text_y,
            label_color,
            self.frame,
            self.gpu,
            self.resources,
        );

        // Badge on the right.
        if let Some(count) = item.badge {
            if count > 0 {
                let badge_text = if count > 99 {
                    "99+".to_string()
                } else {
                    count.to_string()
                };
                let badge_font = font_size * 0.75;
                let badge_pad = self.theme.spacing_unit;
                let badge_w = self.text.measure_text(&badge_text, badge_font) + badge_pad * 2.0;
                let badge_h = badge_font + badge_pad;
                let badge_x = rect.x + rect.w - pad_h - badge_w;
                let badge_y = rect.y + (rect.h - badge_h) / 2.0;
                let badge_r = badge_h / 2.0;

                paint::draw_rounded_rect(
                    self.frame,
                    crate::layout::Rect::new(badge_x, badge_y, badge_w, badge_h),
                    self.theme.red,
                    badge_r,
                );
                let tw = self.text.measure_text(&badge_text, badge_font);
                self.text.draw_text(
                    &badge_text,
                    badge_x + (badge_w - tw) / 2.0,
                    badge_y + (badge_h - badge_font) / 2.0,
                    badge_font,
                    self.theme.fg_on_accent,
                    self.frame,
                    self.gpu,
                    self.resources,
                );
            }
        }

        // Focus ring.
        if response.focused {
            paint::draw_focus_ring(
                self.frame,
                rect,
                self.theme.focus_ring_color,
                0.0,
                self.theme.focus_ring_expand,
            );
        }

        response
    }
}
