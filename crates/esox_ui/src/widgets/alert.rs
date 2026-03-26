//! Alert / banner widgets — status messages with colored backgrounds.
//!
//! # Examples
//!
//! ```ignore
//! ui.alert_info("Deployment is in progress.");
//! ui.alert_success("Changes saved successfully.");
//! ui.alert_warning("Your session will expire in 5 minutes.");
//! ui.alert_error("Failed to connect to the database.");
//! ```

use esox_gfx::Color;

use crate::paint;
use crate::response::Response;
use crate::state::{A11yNode, A11yRole, WidgetKind};
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw an info alert (blue-tinted background).
    pub fn alert_info(&mut self, message: &str) {
        self.alert_inner(message, self.theme.toast_info_bg, self.theme.accent);
    }

    /// Draw a success alert (green-tinted background).
    pub fn alert_success(&mut self, message: &str) {
        self.alert_inner(message, self.theme.toast_success_bg, self.theme.green);
    }

    /// Draw a warning alert (amber-tinted background).
    pub fn alert_warning(&mut self, message: &str) {
        self.alert_inner(message, self.theme.toast_warning_bg, self.theme.amber);
    }

    /// Draw an error alert (red-tinted background).
    pub fn alert_error(&mut self, message: &str) {
        self.alert_inner(message, self.theme.toast_error_bg, self.theme.red);
    }

    fn alert_inner(&mut self, message: &str, bg: Color, accent: Color) {
        let pad = self.theme.padding;
        let font_size = self.theme.font_size;
        let height = font_size + pad * 2.0;
        let accent_width = 3.0;

        let rect = self.allocate_rect(self.region.w, height);

        // Background.
        paint::draw_rounded_rect(self.frame, rect, bg, self.theme.corner_radius);

        // Left accent stripe.
        paint::draw_per_side_border(
            self.frame,
            rect,
            None,
            None,
            None,
            Some((accent, accent_width)),
        );

        // Message text.
        self.text.draw_text(
            message,
            rect.x + pad + accent_width,
            rect.y + pad,
            font_size,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );

        self.push_a11y_node(A11yNode {
            id: crate::id::fnv1a_runtime(message),
            role: A11yRole::Alert,
            label: message.to_string(),
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
    }

    /// Draw a dismissable alert with a close button. Sets `*visible = false` when dismissed.
    pub fn alert_dismissable(
        &mut self,
        id: u64,
        message: &str,
        visible: &mut bool,
        bg: Color,
        accent: Color,
    ) -> Response {
        let pad = self.theme.padding;
        let font_size = self.theme.font_size;
        let height = font_size + pad * 2.0;
        let accent_width = 3.0;
        let close_w = font_size + pad;

        let rect = self.allocate_rect_keyed(id, self.region.w, height);

        // Background.
        paint::draw_rounded_rect(self.frame, rect, bg, self.theme.corner_radius);

        // Left accent stripe.
        paint::draw_per_side_border(
            self.frame,
            rect,
            None,
            None,
            None,
            Some((accent, accent_width)),
        );

        // Message text.
        self.text.draw_text(
            message,
            rect.x + pad + accent_width,
            rect.y + pad,
            font_size,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );

        // Close button (X) on the right.
        let close_id = crate::id::fnv1a_mix(id, 0xC105E);
        let close_rect =
            crate::layout::Rect::new(rect.x + rect.w - close_w, rect.y, close_w, height);
        self.register_widget(close_id, close_rect, WidgetKind::Button);
        let close_response = self.widget_response(close_id, close_rect);

        let x_text = "\u{00D7}"; // multiplication sign as close icon
        let x_w = self.text.measure_text(x_text, font_size);
        let x_color = if close_response.hovered {
            self.theme.fg
        } else {
            self.theme.fg_muted
        };
        self.text.draw_text(
            x_text,
            close_rect.x + (close_rect.w - x_w) / 2.0,
            close_rect.y + pad,
            font_size,
            x_color,
            self.frame,
            self.gpu,
            self.resources,
        );

        if close_response.clicked {
            *visible = false;
        }

        self.push_a11y_node(A11yNode {
            id,
            role: A11yRole::Alert,
            label: message.to_string(),
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

        close_response
    }
}
