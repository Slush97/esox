//! Popover widget — positioned overlay anchored to a rect.
//!
//! # Examples
//!
//! ```ignore
//! let anchor = ui.allocate_rect(100.0, 32.0);
//! // ... draw the trigger widget at anchor ...
//!
//! ui.popover(id!("menu"), &mut popover_open, anchor, |ui| {
//!     ui.label("Popover content");
//!     ui.button(id!("action"), "Do Something");
//! });
//! ```

use esox_input::{Key, NamedKey};

use crate::id::fnv1a_mix;
use crate::layout::Rect;
use crate::paint;
use crate::response::Response;
use crate::state::{Easing, ModalState};
use crate::Ui;

const POPOVER_ANIM_SALT: u64 = 0xb0b0ce47a1f39e02;

impl<'f> Ui<'f> {
    /// Draw a popover anchored below (or above) the given rect.
    ///
    /// When `*open` is true, renders an elevated surface positioned relative to
    /// `anchor`. Content is drawn via the closure. Closes on Escape or click-outside.
    pub fn popover(
        &mut self,
        id: u64,
        open: &mut bool,
        anchor: Rect,
        f: impl FnOnce(&mut Self),
    ) -> Response {
        if !*open {
            return Response::default();
        }

        // Push onto modal stack for focus trapping.
        if !self.state.modal_stack.iter().any(|m| m.id == id) {
            self.state.modal_stack.push(ModalState {
                id,
                open: true,
                saved_focus: self.state.focused,
            });
        }

        let anim_id = fnv1a_mix(id, POPOVER_ANIM_SALT);
        let duration = 150.0;
        let t = self
            .state
            .anim_t(anim_id, 1.0, duration, Easing::EaseOutCubic);

        let vp = self.region;
        let pad = self.theme.padding;
        let corner = self.theme.corner_radius;
        let offset = 4.0; // gap between anchor and popover

        // Popover dimensions — estimate from available space.
        let popover_w = anchor.w.max(200.0).min(vp.w - 16.0);
        let max_h = 300.0;

        // Position: below anchor, or above if not enough space.
        let space_below = vp.y + vp.h - (anchor.y + anchor.h + offset);
        let space_above = anchor.y - vp.y - offset;
        let below = space_below >= max_h || space_below >= space_above;

        let popover_y = if below {
            anchor.y + anchor.h + offset
        } else {
            anchor.y - max_h - offset
        };

        // Center horizontally on anchor, clamped to viewport.
        let ideal_x = anchor.x + (anchor.w - popover_w) / 2.0;
        let popover_x = ideal_x.max(vp.x + 4.0).min(vp.x + vp.w - popover_w - 4.0);

        let popover_rect = Rect::new(popover_x, popover_y, popover_w, max_h);

        // Background with elevation shadow.
        let opacity = t;
        let elev = &self.theme.elevation_medium;
        paint::draw_styled_rect(
            self.frame,
            popover_rect,
            self.theme.bg_surface,
            None,
            esox_gfx::BorderRadius::uniform(corner),
            Some(self.theme.border),
            1.0,
            Some(elev),
            opacity,
        );

        // Escape to close.
        let mut close = false;
        for (event, _) in &self.state.keys.clone() {
            if event.pressed && event.key == Key::Named(NamedKey::Escape) {
                close = true;
            }
        }

        // Click outside to close.
        if let Some((cx, cy, ref mut consumed)) = self.state.mouse.pending_click {
            if !*consumed && !popover_rect.contains(cx, cy) && !anchor.contains(cx, cy) {
                close = true;
                *consumed = true;
            }
        }

        // Set layout state for content.
        let saved_cursor = self.cursor;
        let saved_region = self.region;
        let saved_hit_clip = self.hit_clip;

        let content_rect = Rect::new(
            popover_rect.x + pad,
            popover_rect.y + pad,
            popover_rect.w - pad * 2.0,
            popover_rect.h - pad * 2.0,
        );
        self.hit_clip = Some(popover_rect);
        self.cursor = crate::layout::Vec2 {
            x: content_rect.x,
            y: content_rect.y,
        };
        self.region = content_rect;

        f(self);

        // Restore.
        self.cursor = saved_cursor;
        self.region = saved_region;
        self.hit_clip = saved_hit_clip;

        let mut response = Response::default();
        if close {
            *open = false;
            if let Some(pos) = self.state.modal_stack.iter().position(|m| m.id == id) {
                let modal = self.state.modal_stack.remove(pos);
                self.state.focused = modal.saved_focus;
            }
            self.state.anims.remove(&anim_id);
            response.changed = true;
        }

        response
    }
}
