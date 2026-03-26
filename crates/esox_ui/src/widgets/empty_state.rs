//! Empty state widget — centered placeholder for empty lists/tables.
//!
//! # Examples
//!
//! ```ignore
//! if items.is_empty() {
//!     ui.empty_state("No items yet");
//! }
//!
//! // With an action button
//! if items.is_empty() {
//!     if ui.empty_state_with_action(id!("add"), "No items yet", "Add item").clicked {
//!         show_add_dialog = true;
//!     }
//! }
//! ```

use crate::response::Response;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw a centered empty-state message in muted, slightly larger text.
    pub fn empty_state(&mut self, message: &str) {
        let pad_y = self.theme.padding * 2.0;
        let font_size = self.theme.font_size * 1.15;
        let text_w = self.text.measure_text(message, font_size);

        self.add_space(pad_y);

        let rect = self.allocate_rect(self.region.w, font_size + self.theme.label_pad_y);
        let tx = rect.x + (rect.w - text_w) / 2.0;
        self.text.draw_text(
            message,
            tx,
            rect.y,
            font_size,
            self.theme.fg_muted,
            self.frame,
            self.gpu,
            self.resources,
        );

        self.add_space(pad_y);
    }

    /// Draw an empty-state message with an action button below. Returns a Response
    /// where `clicked` means the action button was pressed.
    pub fn empty_state_with_action(
        &mut self,
        id: u64,
        message: &str,
        action_label: &str,
    ) -> Response {
        let pad_y = self.theme.padding * 2.0;
        let font_size = self.theme.font_size * 1.15;
        let text_w = self.text.measure_text(message, font_size);

        self.add_space(pad_y);

        // Centered message.
        let msg_rect =
            self.allocate_rect_keyed(id, self.region.w, font_size + self.theme.label_pad_y);
        let tx = msg_rect.x + (msg_rect.w - text_w) / 2.0;
        self.text.draw_text(
            message,
            tx,
            msg_rect.y,
            font_size,
            self.theme.fg_muted,
            self.frame,
            self.gpu,
            self.resources,
        );

        self.add_space(self.theme.padding);

        // Centered ghost button.
        let btn_label_w = self.text.measure_text(action_label, self.theme.font_size);
        let btn_w =
            (btn_label_w + self.theme.input_padding * 4.0).max(self.theme.small_button_min_w);

        let mut response = Response::default();
        self.center_horizontal(btn_w, |ui| {
            response = ui.ghost_button(id, action_label);
        });

        self.add_space(pad_y);

        response
    }

    /// Draw a rich empty state with optional icon, title, subtitle, and action.
    ///
    /// `icon_fn` draws a decorative icon/illustration. Pass `None` to skip.
    /// `action` is an optional `(id, label)` for a ghost button. Returns the
    /// button's `Response` if present, otherwise a default response.
    pub fn empty_state_rich(
        &mut self,
        icon_fn: Option<&dyn Fn(&mut Self)>,
        title: &str,
        subtitle: Option<&str>,
        action: Option<(u64, &str)>,
    ) -> Response {
        let pad_y = self.theme.padding * 2.0;
        self.add_space(pad_y);

        // Icon.
        if let Some(draw_icon) = icon_fn {
            let icon_size = self.theme.heading_font_size * 2.0;
            self.center_horizontal(icon_size, |ui| {
                draw_icon(ui);
            });
            self.add_space(self.theme.padding);
        }

        // Title — larger, primary color.
        let title_size = self.theme.heading_font_size;
        let title_w = self.text.measure_text(title, title_size);
        let title_rect = self.allocate_rect(self.region.w, title_size + self.theme.label_pad_y);
        let tx = title_rect.x + (title_rect.w - title_w) / 2.0;
        self.text.draw_text(
            title,
            tx,
            title_rect.y,
            title_size,
            self.theme.fg,
            self.frame,
            self.gpu,
            self.resources,
        );

        // Subtitle — normal size, muted.
        if let Some(sub) = subtitle {
            self.add_space(4.0);
            let sub_size = self.theme.font_size;
            let sub_w = self.text.measure_text(sub, sub_size);
            let sub_rect = self.allocate_rect(self.region.w, sub_size + self.theme.label_pad_y);
            let sx = sub_rect.x + (sub_rect.w - sub_w) / 2.0;
            self.text.draw_text(
                sub,
                sx,
                sub_rect.y,
                sub_size,
                self.theme.fg_muted,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        // Action button.
        let mut response = Response::default();
        if let Some((id, label)) = action {
            self.add_space(self.theme.padding);
            let btn_label_w = self.text.measure_text(label, self.theme.font_size);
            let btn_w =
                (btn_label_w + self.theme.input_padding * 4.0).max(self.theme.small_button_min_w);
            self.center_horizontal(btn_w, |ui| {
                response = ui.ghost_button(id, label);
            });
        }

        self.add_space(pad_y);
        response
    }
}
