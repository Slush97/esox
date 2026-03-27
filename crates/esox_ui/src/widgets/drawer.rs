//! Drawer widget — slide-in side panel with backdrop.
//!
//! # Examples
//!
//! ```ignore
//! ui.drawer(id!("nav"), &mut drawer_open, 300.0, |ui| {
//!     ui.heading("Navigation");
//!     ui.label("Menu items...");
//! });
//!
//! ui.drawer_right(id!("details"), &mut detail_open, 400.0, |ui| {
//!     ui.heading("Item Details");
//! });
//! ```

use esox_input::{Key, NamedKey};

use crate::id::fnv1a_mix;
use crate::layout::Rect;
use crate::paint;
use crate::response::Response;
use crate::state::{Easing, ModalState};
use crate::Ui;

const DRAWER_ANIM_SALT: u64 = 0xd4a7e3b1c6f85902;

impl<'f> Ui<'f> {
    /// Draw a left-side drawer panel. When `*open` is true, slides in from the left.
    pub fn drawer(
        &mut self,
        id: u64,
        open: &mut bool,
        width: f32,
        f: impl FnOnce(&mut Self),
    ) -> Response {
        self.drawer_inner(id, open, width, true, f)
    }

    /// Draw a right-side drawer panel. When `*open` is true, slides in from the right.
    pub fn drawer_right(
        &mut self,
        id: u64,
        open: &mut bool,
        width: f32,
        f: impl FnOnce(&mut Self),
    ) -> Response {
        self.drawer_inner(id, open, width, false, f)
    }

    fn drawer_inner(
        &mut self,
        id: u64,
        open: &mut bool,
        width: f32,
        from_left: bool,
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

        let anim_id = fnv1a_mix(id, DRAWER_ANIM_SALT);
        let duration = self.theme.modal_fade_duration_ms;
        let t = self
            .state
            .anim_t(anim_id, 1.0, duration, Easing::EaseOutCubic);

        let vp = self.region;
        let drawer_w = width.min(vp.w - 40.0);
        let pad = self.theme.padding;
        let _corner = self.theme.corner_radius;

        // Backdrop.
        let backdrop_color = esox_gfx::Color::new(
            self.theme.modal_backdrop.r,
            self.theme.modal_backdrop.g,
            self.theme.modal_backdrop.b,
            self.theme.modal_backdrop.a * t,
        );
        self.frame.push(
            esox_gfx::ShapeBuilder::rect(vp.x, vp.y, vp.w, vp.h)
                .color(backdrop_color)
                .build(),
        );

        // Drawer position — slides in from edge.
        let drawer_x = if from_left {
            vp.x - drawer_w * (1.0 - t) // slides from vp.x-drawer_w to vp.x
        } else {
            vp.x + vp.w - drawer_w * t // slides from vp.x+vp.w to vp.x+vp.w-drawer_w
        };
        let drawer_rect = Rect::new(drawer_x, vp.y, drawer_w, vp.h);

        // Drawer background with elevation shadow.
        let elev = &self.theme.elevation_high;
        paint::draw_styled_rect(
            self.frame,
            drawer_rect,
            self.theme.bg_surface,
            None,
            esox_gfx::BorderRadius::uniform(0.0),
            None,
            0.0,
            Some(elev),
            1.0,
        );

        // Escape to close.
        let mut close = false;
        for (event, _) in &self.state.keys.clone() {
            if event.pressed && event.key == Key::Named(NamedKey::Escape) {
                close = true;
            }
        }

        // Backdrop click to close.
        if let Some((cx, cy, ref mut consumed)) = self.state.mouse.pending_click {
            if !*consumed && !drawer_rect.contains(cx, cy) {
                close = true;
                *consumed = true;
            }
        }

        // Set layout state for content.
        let saved_cursor = self.cursor;
        let saved_region = self.region;
        let saved_hit_clip = self.hit_clip;

        let content_rect = Rect::new(
            drawer_rect.x + pad,
            drawer_rect.y + pad,
            drawer_rect.w - pad * 2.0,
            drawer_rect.h - pad * 2.0,
        );
        self.hit_clip = Some(drawer_rect);
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
