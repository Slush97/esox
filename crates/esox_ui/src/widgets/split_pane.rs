//! Split pane widget -- draggable divider between two resizable panels.
//!
//! # Examples
//!
//! ```ignore
//! ui.split_pane_h(id!("main_split"), 0.3, |ui| {
//!     ui.label("Left panel");
//! }, |ui| {
//!     ui.label("Right panel");
//! });
//!
//! ui.split_pane_v(id!("v_split"), 0.5, |ui| {
//!     ui.label("Top panel");
//! }, |ui| {
//!     ui.label("Bottom panel");
//! });
//! ```

use crate::layout::Rect;
use crate::state::WidgetKind;
use crate::Ui;

use esox_gfx::ShapeBuilder;

impl<'f> Ui<'f> {
    /// Horizontal split pane: left | right with a draggable vertical divider.
    ///
    /// `ratio` is the initial ratio (0.0-1.0) for the left panel, used only when
    /// no stored ratio exists for this `id`. The divider is draggable; the stored
    /// ratio is clamped to [0.05, 0.95].
    pub fn split_pane_h(
        &mut self,
        id: u64,
        ratio: f32,
        left: impl FnOnce(&mut Self),
        right: impl FnOnce(&mut Self),
    ) {
        let (first_rect, second_rect, total_h) = self.split_pane_core(id, ratio, true);
        let inset = self.theme.spacing_unit;
        self.sub_region(first_rect, inset, true, false, left);
        self.sub_region(second_rect, inset, true, false, right);
        self.cursor.y += total_h + self.spacing;
    }

    /// Single-callback variant of [`split_pane_h`] for cases where both panels
    /// need mutable access to the same outer state (avoiding the two-closure
    /// borrow conflict). The callback receives `(ui, panel_index)` where
    /// `panel_index` is 0 for left and 1 for right.
    pub fn split_pane_h_mut(&mut self, id: u64, ratio: f32, mut f: impl FnMut(&mut Self, usize)) {
        let (first_rect, second_rect, total_h) = self.split_pane_core(id, ratio, true);
        let inset = self.theme.spacing_unit;
        self.sub_region_indexed(first_rect, inset, true, 0, &mut f);
        self.sub_region_indexed(second_rect, inset, true, 1, &mut f);
        self.cursor.y += total_h + self.spacing;
    }

    /// Vertical split pane: top / bottom with a draggable horizontal divider.
    ///
    /// `ratio` is the initial ratio (0.0-1.0) for the top panel, used only when
    /// no stored ratio exists for this `id`. The divider is draggable; the stored
    /// ratio is clamped to [0.05, 0.95].
    pub fn split_pane_v(
        &mut self,
        id: u64,
        ratio: f32,
        top: impl FnOnce(&mut Self),
        bottom: impl FnOnce(&mut Self),
    ) {
        let (first_rect, second_rect, total_h) = self.split_pane_core(id, ratio, false);
        let inset = self.theme.spacing_unit;
        self.sub_region(first_rect, inset, true, false, top);
        self.sub_region(second_rect, inset, true, false, bottom);
        self.cursor.y += total_h + self.spacing;
    }

    /// Shared core for all split pane variants: computes geometry, handles
    /// divider interaction and drawing, and returns `(first_rect, second_rect,
    /// total_h)` for the caller to render panels into.
    fn split_pane_core(
        &mut self,
        id: u64,
        initial_ratio: f32,
        horizontal: bool,
    ) -> (Rect, Rect, f32) {
        // Read or initialize stored ratio.
        let ratio = *self
            .state
            .split_ratios
            .entry(id)
            .or_insert_with(|| initial_ratio.clamp(0.05, 0.95));

        // Compute the total available rect.
        let total_w = self.region.w;
        let total_h = self.region.h - (self.cursor.y - self.region.y);
        let origin_x = self.cursor.x;
        let origin_y = self.cursor.y;

        // Calculate panel rects and divider rect.
        let divider_size = self.theme.split_pane_divider;
        let (first_rect, divider_rect, second_rect) = if horizontal {
            let available = total_w - divider_size;
            let left_w = available * ratio;
            let right_w = available - left_w;
            (
                Rect::new(origin_x, origin_y, left_w, total_h),
                Rect::new(origin_x + left_w, origin_y, divider_size, total_h),
                Rect::new(origin_x + left_w + divider_size, origin_y, right_w, total_h),
            )
        } else {
            let available = total_h - divider_size;
            let top_h = available * ratio;
            let bottom_h = available - top_h;
            (
                Rect::new(origin_x, origin_y, total_w, top_h),
                Rect::new(origin_x, origin_y + top_h, total_w, divider_size),
                Rect::new(origin_x, origin_y + top_h + divider_size, total_w, bottom_h),
            )
        };

        // --- Divider interaction ---
        let divider_id = id.wrapping_add(1);
        let kind = if horizontal {
            WidgetKind::SplitDividerH
        } else {
            WidgetKind::SplitDividerV
        };
        self.state.hit_rects.push((divider_rect, divider_id, kind));

        // Start drag on click inside divider.
        if let Some((cx, cy, ref mut consumed)) = self.state.mouse.pending_click {
            if !*consumed && divider_rect.contains(cx, cy) {
                *consumed = true;
                self.state.split_drag = Some((id, horizontal));
            }
        }

        // Update ratio while dragging.
        if let Some((drag_id, _)) = self.state.split_drag {
            if drag_id == id && self.state.mouse_pressed {
                let new_ratio = if horizontal {
                    let available = total_w - divider_size;
                    if available > 0.0 {
                        (self.state.mouse.x - origin_x - divider_size / 2.0) / available
                    } else {
                        ratio
                    }
                } else {
                    let available = total_h - divider_size;
                    if available > 0.0 {
                        (self.state.mouse.y - origin_y - divider_size / 2.0) / available
                    } else {
                        ratio
                    }
                };
                self.state
                    .split_ratios
                    .insert(id, new_ratio.clamp(0.05, 0.95));
                self.state.damage.invalidate_all();
            }
        }

        // --- Draw divider ---
        let divider_hovered = divider_rect.contains(self.state.mouse.x, self.state.mouse.y);
        let is_dragging = self
            .state
            .split_drag
            .is_some_and(|(drag_id, _)| drag_id == id);

        let divider_color = if is_dragging {
            self.theme.accent
        } else if divider_hovered {
            self.theme.border
        } else {
            self.theme.bg_surface
        };

        self.frame.push(
            ShapeBuilder::rect(
                divider_rect.x,
                divider_rect.y,
                divider_rect.w,
                divider_rect.h,
            )
            .color(divider_color)
            .build(),
        );

        // Draw a 1px center line when idle so the divider is always visible.
        if !is_dragging && !divider_hovered {
            if horizontal {
                let cx = divider_rect.x + divider_rect.w / 2.0;
                self.frame.push(
                    ShapeBuilder::rect(cx, divider_rect.y, 1.0, divider_rect.h)
                        .color(self.theme.border)
                        .build(),
                );
            } else {
                let cy = divider_rect.y + divider_rect.h / 2.0;
                self.frame.push(
                    ShapeBuilder::rect(divider_rect.x, cy, divider_rect.w, 1.0)
                        .color(self.theme.border)
                        .build(),
                );
            }
        }

        (first_rect, second_rect, total_h)
    }
}
