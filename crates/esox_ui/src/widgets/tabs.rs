//! Tab bar widget — switchable tabs with optional content area and fade animation.
//!
//! # Examples
//!
//! ```ignore
//! let mut tab = TabState::new();
//! ui.tabs(id!("settings"), &mut tab, &["General", "Advanced"], |ui, idx| {
//!     match idx {
//!         0 => ui.label("General settings"),
//!         1 => ui.label("Advanced settings"),
//!         _ => {}
//!     }
//! });
//! ```

use esox_gfx::ShapeBuilder;
use esox_input::{Key, NamedKey};

use crate::id::{fnv1a_mix, TAB_SLIDE_SALT};
use crate::layout::Rect;
use crate::paint;
use crate::response::Response;
use crate::state::{Easing, TabState, WidgetKind};
use crate::Ui;

const TAB_FADE_SALT: u64 = 0xFADE_7AB5_0000_0001;
const TAB_SLIDE_W_SALT: u64 = 0x7AB5_01D7_0000_0002;

impl<'f> Ui<'f> {
    /// Tab bar + content area. Closure draws content for the selected tab.
    pub fn tabs(
        &mut self,
        id: u64,
        state: &mut TabState,
        labels: &[&str],
        content: impl FnOnce(&mut Self, usize),
    ) -> Response {
        let prev_selected = state.selected;
        let response = self.tab_bar(id, state, labels);

        // Tab content fade animation — restart when selection changes.
        let fade_id = fnv1a_mix(id, TAB_FADE_SALT);
        if state.selected != prev_selected || response.changed {
            if let Some(anim) = self.state.anims.get_mut(&fade_id) {
                anim.from = 0.0;
                anim.to = 1.0;
                anim.start = std::time::Instant::now();
            }
        }
        let _fade_t = self.state.anim_t(
            fade_id,
            1.0,
            self.theme.tab_fade_duration_ms,
            Easing::EaseOutCubic,
        );

        content(self, state.selected);
        response
    }

    /// Tab bar only (no content area).
    pub fn tab_bar(&mut self, id: u64, state: &mut TabState, labels: &[&str]) -> Response {
        let font_size = self.theme.font_size;
        let pad = self.theme.input_padding;
        let indicator_h = self.theme.tab_indicator_height;
        let bar_height = font_size + pad * 2.0 + indicator_h;

        let bar_rect = self.allocate_rect_keyed(id, self.region.w, bar_height);

        let mut response = Response::default();

        // Keyboard: Left/Right cycle tabs when focused.
        let bar_focused = self.state.focused == Some(id);
        if bar_focused {
            let keys: Vec<_> = self.state.keys.clone();
            for (event, _mods) in &keys {
                if !event.pressed {
                    continue;
                }
                match &event.key {
                    Key::Named(NamedKey::ArrowLeft) if state.selected > 0 => {
                        state.selected -= 1;
                        response.changed = true;
                    }
                    Key::Named(NamedKey::ArrowRight) if state.selected + 1 < labels.len() => {
                        state.selected += 1;
                        response.changed = true;
                    }
                    _ => {}
                }
            }
        }

        // Focus ring.
        if bar_focused {
            paint::draw_focus_ring(
                self.frame,
                bar_rect,
                self.theme.focus_ring_color,
                self.theme.corner_radius,
                self.theme.focus_ring_expand,
            );
        }

        // Draw separator line under tab bar.
        self.frame.push(
            ShapeBuilder::rect(bar_rect.x, bar_rect.y + bar_height - 1.0, bar_rect.w, 1.0)
                .color(self.theme.border)
                .build(),
        );

        // Pre-compute tab positions for the slide animation.
        let tab_positions: Vec<(f32, f32)> = {
            let mut positions = Vec::with_capacity(labels.len());
            let mut x = bar_rect.x;
            for label in labels {
                let text_w = self.text.measure_text(label, font_size);
                let tab_w = text_w + pad * 2.0;
                positions.push((x, tab_w));
                x += tab_w;
            }
            positions
        };

        // Draw each tab.
        for (i, label) in labels.iter().enumerate() {
            let (tab_x, tab_w) = tab_positions[i];
            let tab_rect = Rect::new(tab_x, bar_rect.y, tab_w, bar_height);
            let tab_id = fnv1a_mix(id, i as u64);

            self.register_widget(tab_id, tab_rect, WidgetKind::Tab);
            let tab_response = self.widget_response(tab_id, tab_rect);

            if tab_response.clicked {
                state.selected = i;
                response.changed = true;
                // Reset fade animation.
                let fade_id = fnv1a_mix(id, TAB_FADE_SALT);
                if let Some(anim) = self.state.anims.get_mut(&fade_id) {
                    anim.from = 0.0;
                    anim.to = 1.0;
                    anim.start = std::time::Instant::now();
                }
            }

            let selected = state.selected == i;

            self.push_a11y_node(crate::state::A11yNode {
                id: tab_id,
                role: crate::state::A11yRole::Tab,
                label: label.to_string(),
                value: None,
                rect: tab_rect,
                focused: tab_response.focused,
                disabled: false,
                expanded: None,
                selected: Some(selected),
                checked: None,
                value_range: None,
                children: Vec::new(),
            });

            // Hover animation.
            let hover_t = self.state.hover_t(
                tab_id,
                tab_response.hovered && !selected,
                self.theme.hover_duration_ms,
            );

            // Text color.
            let text_color = if selected {
                self.theme.accent
            } else {
                paint::lerp_color(self.theme.fg_muted, self.theme.fg, hover_t)
            };

            // Draw text.
            self.text.draw_text(
                label,
                tab_x + pad,
                bar_rect.y + pad,
                font_size,
                text_color,
                self.frame,
                self.gpu,
                self.resources,
            );
        }

        // Sliding indicator bar — animates between tab positions.
        if !tab_positions.is_empty() && state.selected < tab_positions.len() {
            let (target_x, target_w) = tab_positions[state.selected];
            let anim_x =
                self.state
                    .anim_t(id ^ TAB_SLIDE_SALT, target_x, 200.0, Easing::EaseOutCubic);
            let anim_w =
                self.state
                    .anim_t(id ^ TAB_SLIDE_W_SALT, target_w, 200.0, Easing::EaseOutCubic);
            self.frame.push(
                ShapeBuilder::rect(
                    anim_x,
                    bar_rect.y + bar_height - indicator_h,
                    anim_w,
                    indicator_h,
                )
                .color(self.theme.accent)
                .build(),
            );
        }

        response.focused = bar_focused;
        response
    }
}
