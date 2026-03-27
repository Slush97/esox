//! `esox_ui` — Immediate-mode GPU widget library for esox_platform apps.
//!
//! ## Pattern
//!
//! Widgets are method calls on [`Ui`] that return a [`Response`]. Layout is
//! cursor-based (vertical by default, horizontal via [`Ui::row`]). All mutable
//! state lives in the app; the library stores nothing between frames.
//!
//! ```ignore
//! let mut ui = Ui::begin(&mut frame, &gpu, &mut resources, &mut text, &mut state, &theme, viewport);
//!
//! ui.label("Hello");
//! if ui.button(id!("click"), "Click me").clicked {
//!     count += 1;
//! }
//! ui.text_input(id!("name"), &mut name, "placeholder…");
//!
//! ui.finish();
//! ```
//!
//! ## Layout model
//!
//! - **Vertical** (default): widgets stack top-to-bottom
//! - **Horizontal**: `ui.row(|ui| { ... })` places widgets left-to-right
//! - **Columns**: `ui.columns(&[2.0, 1.0], |ui, i| { ... })` for weighted flex
//! - **Constraints**: `ui.constrained(c, |ui| { ... })` for min/max/aspect
//! - **Scroll**: `ui.scrollable(id, height, |ui| { ... })` for clipped content
//!
//! ## Animation API
//!
//! - [`Ui::animate`] — drive a value towards a target with easing
//! - [`Ui::animate_bool`] — convenience for 0→1 toggle animations
//! - [`Ui::animate_spring`] — spring-physics animation (velocity-based, no fixed duration)
//! - [`Ui::animate_bool_spring`] — convenience for 0→1 spring toggle
//! - [`Ui::animate_keyframes`] — multi-step keyframe sequences with looping
//! - [`Ui::is_animating`] / [`Ui::is_spring_animating`] / [`Ui::is_keyframe_animating`]
//! - [`Easing`] — standard CSS curves, `CubicBezier`, `EaseOutBounce`, `EaseOutBack`, etc.
//! - [`SpringConfig`] — presets: `SNAPPY`, `GENTLE`, `BOUNCY`, `STIFF`
//! - [`lerp_color`] — interpolate between colors

pub mod a11y;
pub mod id;
#[cfg(feature = "markup")]
pub mod interpret;
pub mod layout;
pub mod layout_tree;
pub mod paint;
pub mod response;
pub mod rich_text;
pub mod state;
pub mod text;
pub mod theme;
mod widgets;

pub use id::{fnv1a_mix, fnv1a_runtime, HOVER_SALT};
pub use layout::{
    Align, Constraints, FlexItem, FlexWrap, GridPlacement, GridTrack, Justify, Rect, Spacing,
};
pub use paint::lerp_color;
pub use response::Response;
pub use rich_text::FontWeight;
pub use rich_text::{RichText, Span};
pub use state::{
    A11yNode, A11yRole, A11yTree, ClipboardProvider, DragPayload, DropZoneState, Easing, ImeState,
    InputState, Keyframe, KeyframeSequence, ModalAction, PlaybackMode, SelectState, SortDirection,
    SpringConfig, TabState, TableState, ToastKind, ToastQueue, TooltipState, TreeState, UiState,
    VirtualScrollState, WidgetKind,
};
pub use text::{TextRenderer, TruncationMode};
pub use theme::{
    Elevation, Gradient, IntoPadding, SpacingScale, StyleState, TextAlign, TextDecoration,
    TextSize, TextTransform, Theme, ThemeBuilder, ThemeTransition, Transform2D, WidgetStyle,
};
pub use widgets::avatar::Status;
pub use widgets::form::FieldStatus;
pub use widgets::image::{ImageCache, ImageHandle};
pub use widgets::menu_bar::{Menu, MenuEntry, MenuItem};
pub use widgets::pagination::PaginationState;
pub use widgets::table::{ColumnWidth, TableColumn};
pub use widgets::tree::TreeNodeResponse;

use esox_gfx::{Color, Frame, GpuContext, RenderResources};
use layout::{Direction, LayoutContext, Vec2};
use layout_tree::{LayoutStyle, LayoutTree, TreeBuildContext};

/// Responsive width classification for container queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthClass {
    /// Width < `theme.breakpoint_compact` (default 600px).
    Compact,
    /// Width between compact and expanded breakpoints.
    Medium,
    /// Width >= `theme.breakpoint_expanded` (default 1200px).
    Expanded,
}

/// Rendering access for custom widgets.
///
/// Obtained via [`Ui::painter()`], provides mutable access to the frame and
/// text renderer plus shared access to GPU context, resources, and theme.
pub struct Painter<'a> {
    pub frame: &'a mut Frame,
    pub text: &'a mut text::TextRenderer,
    pub gpu: &'a GpuContext,
    pub resources: &'a mut RenderResources,
    pub theme: &'a theme::Theme,
}

/// Builder for flex row/column layouts with gap, alignment, and justification.
pub struct FlexBuilder<'a, 'f> {
    ui: &'a mut Ui<'f>,
    direction: Direction,
    gap: f32,
    align: layout::Align,
    justify: layout::Justify,
    wrap: layout::FlexWrap,
}

impl<'a, 'f> FlexBuilder<'a, 'f> {
    /// Set the gap between children.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set cross-axis alignment.
    pub fn align(mut self, align: layout::Align) -> Self {
        self.align = align;
        self
    }

    /// Set main-axis justification.
    pub fn justify(mut self, justify: layout::Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Set flex wrap mode.
    pub fn wrap(mut self, wrap: layout::FlexWrap) -> Self {
        self.wrap = wrap;
        self
    }

    /// Draw a flex layout with per-child grow/shrink via `FlexUi`.
    ///
    /// The tree solver handles grow/shrink distribution and justification.
    /// On frame 1 (no prev_layout), children render at natural cursor positions.
    /// On frame 2+, `allocate_rect` picks up solved positions from the tree.
    pub fn show_flex(self, id: u64, f: impl FnOnce(&mut FlexUi<'_, 'f>)) {
        self.ui.tree_build.open_container(
            Some(id),
            LayoutStyle {
                direction: self.direction,
                gap: self.gap,
                align_items: self.align,
                justify_content: self.justify,
                flex_wrap: self.wrap,
                ..Default::default()
            },
        );

        let saved_cursor = self.ui.cursor;
        let saved_region = self.ui.region;
        let saved_spacing = self.ui.spacing;

        // Set up cursor direction for fallback positioning.
        if self.direction == Direction::Horizontal {
            self.ui.spacing = self.gap;
            let ctx = LayoutContext {
                direction: Direction::Horizontal,
                origin: self.ui.cursor,
                region: self.ui.region,
                saved_cursor: self.ui.cursor,
                spacing: self.gap,
                max_cross: 0.0,
                clip_rect: None,
            };
            self.ui.layout_stack.push(ctx);
        } else {
            self.ui.spacing = self.gap;
        }

        let mut flex_ui = FlexUi {
            ui: self.ui,
            direction: self.direction,
            child_index: 0,
        };

        f(&mut flex_ui);

        let child_count = flex_ui.child_index;

        if self.direction == Direction::Horizontal {
            let ctx = self.ui.layout_stack.pop().unwrap();
            let max_cross = ctx.max_cross;
            self.ui.cursor.x = saved_cursor.x;
            self.ui.cursor.y = saved_cursor.y + max_cross + saved_spacing;
            // If no children advanced the cursor, keep y stable.
            if child_count == 0 {
                self.ui.cursor.y = saved_cursor.y;
            }
        }

        self.ui.region = saved_region;
        self.ui.spacing = saved_spacing;
        self.ui.tree_build.close_container();
    }

    /// Draw the flex layout with the given content closure.
    ///
    /// Opens a tree container so the solver handles justification on frame 2+.
    /// Falls back to cursor-based positioning on frame 1.
    pub fn show(self, f: impl FnOnce(&mut Ui<'f>)) {
        let is_horizontal = self.direction == Direction::Horizontal;

        self.ui.tree_build.open_container(
            None,
            LayoutStyle {
                direction: self.direction,
                gap: self.gap,
                align_items: self.align,
                justify_content: self.justify,
                flex_wrap: self.wrap,
                ..Default::default()
            },
        );

        if is_horizontal {
            let saved_spacing = self.ui.spacing;
            self.ui.spacing = self.gap;
            let ctx = LayoutContext {
                direction: Direction::Horizontal,
                origin: self.ui.cursor,
                region: self.ui.region,
                saved_cursor: self.ui.cursor,
                spacing: self.ui.spacing,
                max_cross: 0.0,
                clip_rect: None,
            };
            self.ui.layout_stack.push(ctx);
            f(self.ui);
            let ctx = self.ui.layout_stack.pop().unwrap();
            self.ui.cursor.x = ctx.saved_cursor.x;
            self.ui.cursor.y = ctx.saved_cursor.y + ctx.max_cross + saved_spacing;
            self.ui.spacing = saved_spacing;
        } else {
            let saved_spacing = self.ui.spacing;
            self.ui.spacing = self.gap;
            f(self.ui);
            self.ui.spacing = saved_spacing;
        }

        self.ui.tree_build.close_container();
    }
}

/// Context for placing flex items with grow/shrink within a `FlexBuilder::show_flex()`.
pub struct FlexUi<'a, 'f> {
    ui: &'a mut Ui<'f>,
    direction: Direction,
    child_index: usize,
}

impl<'a, 'f> FlexUi<'a, 'f> {
    /// Place a flex item with default properties (no grow/shrink).
    pub fn item_default(&mut self, f: impl FnOnce(&mut Ui<'f>)) {
        self.item(layout::FlexItem::default(), f);
    }

    /// Place a flex item with explicit grow/shrink/alignment properties.
    pub fn item(&mut self, props: layout::FlexItem, f: impl FnOnce(&mut Ui<'f>)) {
        let cross_dir = if self.direction == Direction::Horizontal {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };

        self.ui.tree_build.open_container(
            None,
            LayoutStyle {
                direction: cross_dir,
                flex_grow: props.grow,
                flex_shrink: props.shrink,
                flex_basis: props.basis,
                align_self: props.align_self,
                margin: props.margin,
                gap: self.ui.spacing,
                ..Default::default()
            },
        );

        // Apply margin for cursor fallback.
        if self.direction == Direction::Horizontal {
            self.ui.cursor.x += props.margin.left;
        } else {
            self.ui.cursor.y += props.margin.top;
        }

        f(self.ui);

        if self.direction == Direction::Horizontal {
            self.ui.cursor.x += props.margin.right;
        } else {
            self.ui.cursor.y += props.margin.bottom;
        }

        self.ui.tree_build.close_container();
        self.child_index += 1;
    }

    /// Access the inner `Ui` for direct widget calls outside flex items.
    pub fn ui(&mut self) -> &mut Ui<'f> {
        self.ui
    }
}

/// Builder for CSS Grid-like layouts with column/row track definitions.
pub struct GridBuilder<'a, 'f> {
    ui: &'a mut Ui<'f>,
    columns: Vec<layout::GridTrack>,
    rows: Vec<layout::GridTrack>,
    col_gap: f32,
    row_gap: f32,
}

impl<'a, 'f> GridBuilder<'a, 'f> {
    /// Set the gap between columns.
    pub fn col_gap(mut self, gap: f32) -> Self {
        self.col_gap = gap;
        self
    }

    /// Set the gap between rows.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = gap;
        self
    }

    /// Set both column and row gaps to the same value.
    pub fn gap(mut self, gap: f32) -> Self {
        self.col_gap = gap;
        self.row_gap = gap;
        self
    }

    /// Draw the grid layout. The closure receives a `GridUi` for placing cells.
    pub fn show(self, f: impl FnOnce(&mut GridUi<'_, 'f>)) {
        self.ui.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                grid_columns: Some(self.columns.clone()),
                grid_rows: Some(self.rows.clone()),
                grid_column_gap: Some(self.col_gap),
                grid_row_gap: Some(self.row_gap),
                ..Default::default()
            },
        );

        let saved_cursor = self.ui.cursor;
        let saved_region = self.ui.region;
        let saved_spacing = self.ui.spacing;

        // Compute track sizes inline so the grid works inside scrollables,
        // where prev_layout lookups are disabled.
        let available_w = self.ui.region.w;
        let available_h = self.ui.region.h;
        let col_sizes = resolve_grid_tracks_inline(&self.columns, available_w, self.col_gap);
        let row_sizes = resolve_grid_tracks_inline(&self.rows, available_h, self.row_gap);

        let col_positions = grid_track_positions(&col_sizes, self.col_gap, saved_cursor.x);
        let row_positions = grid_track_positions(&row_sizes, self.row_gap, saved_cursor.y);

        let mut grid_ui = GridUi {
            ui: self.ui,
            col_sizes,
            row_sizes,
            col_positions,
            row_positions,
            col_gap: self.col_gap,
            row_gap: self.row_gap,
            saved_cursor,
            saved_region,
        };
        f(&mut grid_ui);

        // Restore cursor and advance by total grid height.
        let total_row_gap = self.row_gap * grid_ui.row_sizes.len().saturating_sub(1) as f32;
        let total_h: f32 = grid_ui.row_sizes.iter().sum::<f32>() + total_row_gap;

        self.ui.cursor = saved_cursor;
        self.ui.cursor.y += total_h + self.ui.spacing;
        self.ui.region = saved_region;
        self.ui.spacing = saved_spacing;

        self.ui.tree_build.close_container();
    }
}

/// Resolve grid track sizes from definitions and available space (inline version).
fn resolve_grid_tracks_inline(defs: &[layout::GridTrack], available: f32, gap: f32) -> Vec<f32> {
    if defs.is_empty() {
        return vec![available];
    }

    let total_gap = gap * (defs.len().saturating_sub(1)) as f32;
    let mut remaining = (available - total_gap).max(0.0);
    let mut sizes = vec![0.0f32; defs.len()];
    let mut total_fr = 0.0f32;

    // First pass: resolve Fixed tracks, accumulate Fr.
    for (i, track) in defs.iter().enumerate() {
        match *track {
            layout::GridTrack::Fixed(px) => {
                sizes[i] = px;
                remaining -= px;
            }
            layout::GridTrack::Auto => {
                // Without child measurement, Auto tracks get no space here.
                // The tree solver handles Auto in arrange_grid for non-scroll contexts.
            }
            layout::GridTrack::MinMax(min, _) => {
                sizes[i] = min;
                remaining -= min;
                total_fr += 1.0;
            }
            layout::GridTrack::Fr(fr) => {
                total_fr += fr;
            }
        }
    }

    // Second pass: distribute remaining space to Fr and MinMax tracks.
    remaining = remaining.max(0.0);
    if total_fr > 0.0 {
        let per_fr = remaining / total_fr;
        for (i, track) in defs.iter().enumerate() {
            match *track {
                layout::GridTrack::Fr(fr) => sizes[i] = per_fr * fr,
                layout::GridTrack::MinMax(min, max) => sizes[i] = (min + per_fr).min(max),
                _ => {}
            }
        }
    }

    sizes
}

/// Compute cumulative start positions from track sizes and gap.
fn grid_track_positions(sizes: &[f32], gap: f32, origin: f32) -> Vec<f32> {
    let mut positions = Vec::with_capacity(sizes.len());
    let mut pos = origin;
    for &size in sizes {
        positions.push(pos);
        pos += size + gap;
    }
    positions
}

/// Context for placing cells within a grid layout.
pub struct GridUi<'a, 'f> {
    ui: &'a mut Ui<'f>,
    col_sizes: Vec<f32>,
    row_sizes: Vec<f32>,
    col_positions: Vec<f32>,
    row_positions: Vec<f32>,
    col_gap: f32,
    row_gap: f32,
    saved_cursor: layout::Vec2,
    saved_region: layout::Rect,
}

impl<'a, 'f> GridUi<'a, 'f> {
    /// Place a cell at the given grid position. Content is rendered inside the cell.
    pub fn cell(&mut self, placement: layout::GridPlacement, f: impl FnOnce(&mut Ui<'f>)) {
        let c = placement.column as usize;
        let r = placement.row as usize;
        let cs = placement.col_span as usize;
        let rs = placement.row_span as usize;

        // Compute cell rect from precomputed track positions.
        let (x, w) = if c < self.col_positions.len() {
            let x = self.col_positions[c];
            let end_c = (c + cs).min(self.col_sizes.len());
            let w: f32 = self.col_sizes[c..end_c].iter().sum::<f32>()
                + self.col_gap * (end_c - c).saturating_sub(1) as f32;
            (x, w)
        } else {
            (self.saved_cursor.x, self.saved_region.w)
        };

        let (y, h) = if r < self.row_positions.len() {
            let y = self.row_positions[r];
            let end_r = (r + rs).min(self.row_sizes.len());
            let h: f32 = self.row_sizes[r..end_r].iter().sum::<f32>()
                + self.row_gap * (end_r - r).saturating_sub(1) as f32;
            (y, h)
        } else {
            (self.ui.cursor.y, 0.0)
        };

        self.ui.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap: self.ui.spacing,
                grid_placement: Some(placement),
                ..Default::default()
            },
        );

        let prev_cursor = self.ui.cursor;
        let prev_region = self.ui.region;

        self.ui.cursor = layout::Vec2 { x, y };
        self.ui.region = layout::Rect::new(x, y, w, h);

        f(self.ui);

        self.ui.cursor = prev_cursor;
        self.ui.region = prev_region;

        self.ui.tree_build.close_container();
    }

    /// Access the inner `Ui` for direct widget calls.
    pub fn ui(&mut self) -> &mut Ui<'f> {
        self.ui
    }
}

/// Snapshot of all layout state that container widgets need to save/restore.
///
/// Captured by [`Ui::sub_region`] and restored automatically when the closure
/// returns. Widget authors should never need to construct this manually.
struct SavedLayoutState {
    cursor: Vec2,
    region: Rect,
    spacing: f32,
    gpu_clip: Option<[f32; 4]>,
    hit_clip: Option<Rect>,
}

/// The main UI context. Created each frame, consumed by `finish()`.
pub struct Ui<'f> {
    pub(crate) frame: &'f mut Frame,
    pub(crate) gpu: &'f GpuContext,
    pub(crate) resources: &'f mut RenderResources,
    pub(crate) text: &'f mut TextRenderer,
    pub(crate) state: &'f mut UiState,
    pub(crate) theme: &'f Theme,

    // Layout cursor.
    cursor: Vec2,
    region: Rect,
    layout_stack: Vec<LayoutContext>,
    spacing: f32,
    /// Active hit-test clip rect. Widgets outside this rect won't receive clicks.
    hit_clip: Option<Rect>,
    /// Whether widgets are currently disabled (no interaction).
    disabled: bool,
    /// Style override stack for per-widget styling.
    style_stack: Vec<WidgetStyle>,
    /// Layout tree being built this frame.
    tree_build: TreeBuildContext,
    /// Solved layout tree from the previous frame (for position lookups).
    prev_layout: Option<LayoutTree>,
    /// Nesting depth inside scroll containers. Cache lookups are skipped when > 0
    /// because cached absolute positions don't account for scroll offset changes.
    scroll_depth: u32,
}

impl<'f> Ui<'f> {
    /// Begin a new UI frame within the given viewport rectangle.
    pub fn begin(
        frame: &'f mut Frame,
        gpu: &'f GpuContext,
        resources: &'f mut RenderResources,
        text: &'f mut TextRenderer,
        state: &'f mut UiState,
        theme: &'f Theme,
        viewport: Rect,
    ) -> Self {
        text.advance_generation();
        text.set_ui_font_size(theme.font_size);
        text.set_header_font_size(theme.header_font_size);
        state.begin_frame(theme.scroll_friction);

        // Set up tile grid for partial redraw if available.
        if let Some(ref mut grid) = state.tile_grid {
            grid.resize(viewport.w as u32, viewport.h as u32);
            grid.begin_frame(&state.damage);
            frame.begin_partial(grid);
        }

        // Move the solved layout tree from last frame into prev_layout.
        // Cache lookups are skipped inside scroll containers (via scroll_depth)
        // because cached absolute positions don't account for scroll offset
        // changes between frames.
        let prev_layout = state.layout_cache.take();

        let mut tree_build = TreeBuildContext::new();
        tree_build.open_container(
            Some(u64::MAX),
            LayoutStyle {
                direction: Direction::Vertical,
                gap: theme.padding,
                ..Default::default()
            },
        );

        Self {
            frame,
            gpu,
            resources,
            text,
            state,
            theme,
            cursor: Vec2 {
                x: viewport.x,
                y: viewport.y,
            },
            region: viewport,
            layout_stack: Vec::new(),
            spacing: theme.padding,
            hit_clip: None,
            disabled: false,
            style_stack: Vec::new(),
            tree_build,
            prev_layout,
            scroll_depth: 0,
        }
    }

    /// Enable tile-based partial redraw. Call once (e.g. after first resize)
    /// to opt in to tile caching. The grid is created lazily from the viewport.
    pub fn enable_partial_redraw(state: &mut UiState, viewport_w: u32, viewport_h: u32) {
        state.tile_grid = Some(esox_gfx::TileGrid::new(viewport_w, viewport_h));
    }

    /// Finish the frame — draw modals, overlays, toasts, tooltips, clean up per-frame state.
    /// Returns any overlay selection that occurred: (id, selected_index).
    pub fn finish(mut self) -> Option<(u64, usize)> {
        // Switch to overlay mode so modals/toasts/tooltips bypass tile routing.
        if self.state.tile_grid.is_some() {
            self.frame.set_overlay_mode(true);
        }

        // Draw order: normal content (already drawn) → modals → dropdowns → toasts → tooltips
        self.draw_modals();
        let selection = self.draw_overlay();
        self.draw_deferred_menu_bar();
        self.draw_toasts();
        self.draw_tooltip();
        self.draw_debug_overlay();

        // Finalize tile grid: merge dirty/clean tiles + overlay.
        if let Some(ref mut grid) = self.state.tile_grid {
            self.frame.finalize_partial(grid);
        }

        // Close root container and solve the layout tree.
        self.tree_build.close_container();
        debug_assert_eq!(
            self.tree_build.open_stack_len(),
            0,
            "unclosed layout container"
        );
        let viewport = self.region;
        let mut tree = self.tree_build.tree;
        tree.solve(viewport);
        self.state.layout_cache = Some(tree);

        self.state.end_frame();
        selection
    }

    // ── Layout ──

    /// Allocate a rectangle in the current layout direction.
    pub fn allocate_rect(&mut self, w: f32, h: f32) -> Rect {
        // Record this leaf in the layout tree.
        let node_id = self.tree_build.add_leaf(None, w, h);
        let node_key = self.tree_build.tree.node(node_id).key;

        let rect = if let Some(solved) = self
            .prev_layout
            .as_ref()
            .filter(|_| self.scroll_depth == 0)
            .and_then(|t| t.lookup(node_key))
        {
            // Use solved position from previous frame. Advance cursor for compatibility.
            match self.layout_stack.last() {
                Some(ctx) if ctx.direction == Direction::Horizontal => {
                    self.cursor.x = solved.x + solved.w + self.spacing;
                }
                _ => {
                    self.cursor.y = solved.y + solved.h + self.spacing;
                }
            }
            solved
        } else {
            // Cursor fallback (first frame / new widgets).
            match self.layout_stack.last() {
                Some(ctx) if ctx.direction == Direction::Horizontal => {
                    let remaining = (self.region.w - (self.cursor.x - self.region.x)).max(0.0);
                    let actual_w = w.min(remaining);
                    let r = Rect::new(self.cursor.x, self.cursor.y, actual_w, h);
                    self.cursor.x += actual_w + self.spacing;
                    r
                }
                _ => {
                    let actual_w = w.min(self.region.w);
                    let r = Rect::new(self.cursor.x, self.cursor.y, actual_w, h);
                    self.cursor.y += h + self.spacing;
                    r
                }
            }
        };

        // Track max_cross for horizontal layouts.
        if let Some(ctx) = self.layout_stack.last_mut() {
            if ctx.direction == Direction::Horizontal && rect.h > ctx.max_cross {
                ctx.max_cross = rect.h;
            }
        }
        // Debug overlay: collect layout bounds.
        if self.state.debug_overlay {
            let depth = self.layout_stack.len();
            let kind = match depth {
                0 => "root",
                1 => "child",
                _ => "nested",
            };
            self.state.debug_widget_rects.push((rect, 0, kind));
        }
        rect
    }

    /// Set a key override for the next `allocate_rect` call.
    pub fn keyed(&mut self, key: u64) {
        self.tree_build.push_key_scope(key);
    }

    /// Allocate a rectangle with an explicit key for cross-frame stability.
    pub fn allocate_rect_keyed(&mut self, key: u64, w: f32, h: f32) -> Rect {
        self.tree_build.push_key_scope(key);
        self.allocate_rect(w, h)
    }

    /// Run a closure in a horizontal row layout.
    pub fn row(&mut self, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Horizontal,
                gap: self.spacing,
                ..Default::default()
            },
        );
        let ctx = LayoutContext {
            direction: Direction::Horizontal,
            origin: self.cursor,
            region: self.region,
            saved_cursor: self.cursor,
            spacing: self.spacing,
            max_cross: 0.0,
            clip_rect: None,
        };
        self.layout_stack.push(ctx);
        f(self);
        let ctx = self.layout_stack.pop().unwrap();
        // Restore cursor to below the tallest child.
        self.cursor.x = ctx.saved_cursor.x;
        self.cursor.y = ctx.saved_cursor.y + ctx.max_cross + self.spacing;
        self.tree_build.close_container();
    }

    /// Run a closure in a horizontal row, then vertically center all children.
    ///
    /// Uses a two-pass approach: renders children top-aligned, then shifts
    /// the entire content block down so it is vertically centered within the
    /// tallest child's height.
    pub fn row_centered(&mut self, f: impl FnOnce(&mut Self)) {
        let inst_start = self.frame.instance_len();

        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Horizontal,
                gap: self.spacing,
                ..Default::default()
            },
        );
        let ctx = LayoutContext {
            direction: Direction::Horizontal,
            origin: self.cursor,
            region: self.region,
            saved_cursor: self.cursor,
            spacing: self.spacing,
            max_cross: 0.0,
            clip_rect: None,
        };
        self.layout_stack.push(ctx);
        f(self);
        let ctx = self.layout_stack.pop().unwrap();
        let max_cross = ctx.max_cross;
        self.cursor.x = ctx.saved_cursor.x;
        self.cursor.y = ctx.saved_cursor.y + max_cross + self.spacing;
        self.tree_build.close_container();

        // Vertically center each instance within max_cross.
        // Instances shorter than max_cross get shifted down by half the
        // difference; instances at full height stay in place.
        let inst_end = self.frame.instance_len();
        if max_cross > 0.0 {
            for i in inst_start..inst_end {
                let h = self.frame.instance_data()[i].rect[3];
                if h > 0.0 && h < max_cross {
                    let dy = (max_cross - h) / 2.0;
                    self.frame.offset_instances_y(i, i + 1, dy);
                }
            }
        }
    }

    /// Set spacing between subsequent widgets.
    pub fn spacing(&mut self, amount: f32) {
        self.spacing = amount;
    }

    /// Add extra vertical space.
    pub fn add_space(&mut self, amount: f32) {
        self.cursor.y += amount;
    }

    /// Add a section-level gap. Use between major page areas.
    pub fn section_break(&mut self) {
        self.cursor.y += self.theme.section_gap;
    }

    /// Run a closure within a max-width container, centered horizontally.
    pub fn max_width(&mut self, max_w: f32, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                max_width: Some(max_w),
                direction: Direction::Vertical,
                gap: self.spacing,
                ..Default::default()
            },
        );
        let col_w = self.region.w.min(max_w);
        let col_x = self.cursor.x + (self.region.w - col_w) / 2.0;

        let saved_cursor = self.cursor;
        let saved_region = self.region;

        self.cursor.x = col_x;
        self.region = Rect::new(col_x, self.cursor.y, col_w, self.region.h);

        f(self);

        let new_y = self.cursor.y;
        self.cursor = saved_cursor;
        self.cursor.y = new_y;
        self.region = saved_region;
        self.tree_build.close_container();
    }

    /// Run a closure with padding on all sides.
    ///
    /// Accepts either a raw `f32` pixel value or a [`SpacingScale`] token
    /// (resolved via [`Theme::space`]).
    pub fn padding(&mut self, amount: impl theme::IntoPadding, f: impl FnOnce(&mut Self)) {
        let amount = amount.resolve(self.theme);
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                padding: Spacing::all(amount),
                gap: self.spacing,
                ..Default::default()
            },
        );
        let saved_cursor = self.cursor;
        let saved_region = self.region;

        self.cursor.x += amount;
        self.cursor.y += amount;
        self.region = Rect::new(
            self.cursor.x,
            self.cursor.y,
            self.region.w - amount * 2.0,
            self.region.h - amount * 2.0,
        );

        f(self);

        let new_y = self.cursor.y + amount;
        self.cursor = saved_cursor;
        self.cursor.y = new_y;
        self.region = saved_region;
        self.tree_build.close_container();
    }

    /// Get the current cursor X position.
    pub fn cursor_x(&self) -> f32 {
        self.cursor.x
    }

    /// Get the current cursor Y position (useful for tracking content height).
    pub fn cursor_y(&self) -> f32 {
        self.cursor.y
    }

    /// Get the current region width.
    pub fn region_width(&self) -> f32 {
        self.region.w
    }

    /// Whether the current layout context is horizontal (inside a `row`).
    pub fn is_in_row(&self) -> bool {
        self.layout_stack
            .last()
            .is_some_and(|ctx| ctx.direction == Direction::Horizontal)
    }

    /// Compute label allocation width: measured text width in horizontal
    /// layouts (inline), full region width in vertical layouts (block).
    pub(crate) fn label_alloc_width(&self, text_w: f32) -> f32 {
        if self.is_in_row() {
            text_w
        } else {
            self.region.w
        }
    }

    /// Get the current region height.
    pub fn region_height(&self) -> f32 {
        self.region.h
    }

    /// Get the remaining vertical space from the cursor to the bottom of the region.
    pub fn remaining_height(&self) -> f32 {
        ((self.region.y + self.region.h) - self.cursor.y).max(0.0)
    }

    /// Responsive width classification based on the current region width.
    pub fn width_class(&self) -> WidthClass {
        if self.region.w < self.theme.breakpoint_compact {
            WidthClass::Compact
        } else if self.region.w >= self.theme.breakpoint_expanded {
            WidthClass::Expanded
        } else {
            WidthClass::Medium
        }
    }

    /// Narrow the region: offset cursor.x and reduce region.w.
    /// Useful for centering content without a closure.
    pub fn indent(&mut self, offset: f32, width: f32) {
        self.cursor.x += offset;
        self.region = Rect::new(self.cursor.x, self.region.y, width, self.region.h);
    }

    /// Run a closure with a temporary spacing value. Restores original spacing after.
    pub fn with_spacing(&mut self, gap: f32, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap,
                ..Default::default()
            },
        );
        let saved = self.spacing;
        self.spacing = gap;
        f(self);
        self.spacing = saved;
        self.tree_build.close_container();
    }

    /// Run a closure in a horizontal row with a specific inter-widget gap.
    pub fn row_spaced(&mut self, gap: f32, f: impl FnOnce(&mut Self)) {
        let saved = self.spacing;
        self.spacing = gap;
        self.row(|ui| {
            f(ui);
        });
        self.spacing = saved;
    }

    /// Run a closure in an explicit vertical column container.
    ///
    /// Vertical is already the default layout direction, so this is primarily
    /// useful for creating a tree container node that the flex solver can
    /// reason about (enabling [`spacer`](Self::spacer) and flex distribution).
    pub fn col(&mut self, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap: self.spacing,
                ..Default::default()
            },
        );
        f(self);
        self.tree_build.close_container();
    }

    /// Run a closure in a vertical column with a specific inter-widget gap.
    pub fn col_spaced(&mut self, gap: f32, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap,
                ..Default::default()
            },
        );
        let saved = self.spacing;
        self.spacing = gap;
        f(self);
        self.spacing = saved;
        self.tree_build.close_container();
    }

    /// Run a closure in a vertical column, then horizontally center all
    /// children within the available region width.
    pub fn col_centered(&mut self, f: impl FnOnce(&mut Self)) {
        let inst_start = self.frame.instance_len();

        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap: self.spacing,
                align_items: layout::Align::Center,
                ..Default::default()
            },
        );
        f(self);
        self.tree_build.close_container();

        // Horizontally center each instance within the region.
        let inst_end = self.frame.instance_len();
        if self.region.w > 0.0 {
            for i in inst_start..inst_end {
                let w = self.frame.instance_data()[i].rect[2];
                if w > 0.0 && w < self.region.w {
                    let dx = (self.region.w - w) / 2.0;
                    self.frame.offset_instances_x(i, i + 1, dx);
                }
            }
        }
    }

    // ── Flex Layout ──

    /// Create a horizontal flex layout builder.
    pub fn flex_row(&mut self) -> FlexBuilder<'_, 'f> {
        FlexBuilder {
            ui: self,
            direction: Direction::Horizontal,
            gap: 0.0,
            align: layout::Align::Start,
            justify: layout::Justify::Start,
            wrap: layout::FlexWrap::NoWrap,
        }
    }

    /// Create a vertical flex layout builder.
    pub fn flex_col(&mut self) -> FlexBuilder<'_, 'f> {
        FlexBuilder {
            ui: self,
            direction: Direction::Vertical,
            gap: 0.0,
            align: layout::Align::Start,
            justify: layout::Justify::Start,
            wrap: layout::FlexWrap::NoWrap,
        }
    }

    /// Create a CSS Grid layout builder with column and row track definitions.
    ///
    /// # Example
    /// ```ignore
    /// ui.grid(
    ///     &[GridTrack::Fr(1.0), GridTrack::Fr(1.0), GridTrack::Fixed(200.0)],
    ///     &[GridTrack::Auto, GridTrack::Fr(1.0)],
    /// ).gap(12.0).show(|grid| {
    ///     grid.cell(GridPlacement::at(0, 0), |ui| { ui.label("A"); });
    ///     grid.cell(GridPlacement::at(1, 0).span(2, 1), |ui| { ui.label("B"); });
    /// });
    /// ```
    pub fn grid(
        &mut self,
        columns: &[layout::GridTrack],
        rows: &[layout::GridTrack],
    ) -> GridBuilder<'_, 'f> {
        GridBuilder {
            ui: self,
            columns: columns.to_vec(),
            rows: rows.to_vec(),
            col_gap: 0.0,
            row_gap: 0.0,
        }
    }

    /// Center content horizontally within the current region.
    ///
    /// `content_width` is the expected width of the content inside.
    pub fn center_horizontal(&mut self, content_width: f32, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                max_width: Some(content_width),
                direction: Direction::Vertical,
                gap: self.spacing,
                ..Default::default()
            },
        );
        let cw = content_width.min(self.region.w);
        let offset = (self.region.w - cw) / 2.0;

        let saved_cursor = self.cursor;
        let saved_region = self.region;

        self.cursor.x += offset;
        self.region = Rect::new(self.cursor.x, self.cursor.y, cw, self.region.h);

        f(self);

        let new_y = self.cursor.y;
        self.cursor = saved_cursor;
        self.cursor.y = new_y;
        self.region = saved_region;
        self.tree_build.close_container();
    }

    /// In a row, advance cursor.x to right-align the remaining content.
    ///
    /// `reserve_right` is the width to reserve for trailing widgets.
    pub fn fill_space(&mut self, reserve_right: f32) {
        let target_x = self.region.x + self.region.w - reserve_right;
        if target_x > self.cursor.x {
            self.cursor.x = target_x;
        }
    }

    /// Insert a flexible spacer that absorbs remaining space in the parent.
    ///
    /// Works in both [`row`](Self::row) (horizontal) and [`col`](Self::col) /
    /// default vertical contexts. The spacer is a zero-size tree leaf with
    /// `flex_grow: 1.0` — on frame 2+ the tree solver distributes remaining
    /// parent space to it.
    ///
    /// On frame 1 (before the tree solver has run), the spacer has zero size.
    /// For immediate frame-1 effect in horizontal rows, use
    /// [`fill_space`](Self::fill_space) instead.
    pub fn spacer(&mut self) {
        let node_id = self.tree_build.add_leaf(None, 0.0, 0.0);
        self.tree_build.set_flex(node_id, 1.0, 0.0, Some(0.0));
        let node_key = self.tree_build.tree.node(node_id).key;

        // On frame 2+, advance cursor past the solved spacer rect.
        if let Some(solved) = self
            .prev_layout
            .as_ref()
            .filter(|_| self.scroll_depth == 0)
            .and_then(|t| t.lookup(node_key))
        {
            match self.layout_stack.last() {
                Some(ctx) if ctx.direction == Direction::Horizontal => {
                    self.cursor.x = solved.x + solved.w + self.spacing;
                }
                _ => {
                    self.cursor.y = solved.y + solved.h + self.spacing;
                }
            }
        }
        // Frame 1: zero size, no cursor advance — tree solver corrects on frame 2.
    }

    /// Measure the size a closure would occupy without actually drawing.
    ///
    /// Runs `f` to populate GPU instances, records cursor delta, then truncates
    /// the instance buffer to discard all generated instances. Use only for
    /// small subtrees — the closure is fully executed (double-render cost).
    pub fn measure(&mut self, f: impl FnOnce(&mut Self)) -> (f32, f32) {
        let saved_cursor = self.cursor;
        let start_len = self.frame.instance_len();

        f(self);

        let dx = self.cursor.x - saved_cursor.x;
        let dy = self.cursor.y - saved_cursor.y;

        // Discard all GPU instances generated by f.
        self.frame.truncate_instances(start_len);
        self.cursor = saved_cursor;

        (dx, dy)
    }

    // ── Flex/Weighted Columns ──

    /// Weighted column layout. Calls `f(ui, col_index)` for each column.
    /// Weights are relative: &[2.0, 1.0] -> 2/3 + 1/3 of available width.
    pub fn columns(&mut self, weights: &[f32], f: impl FnMut(&mut Self, usize)) {
        self.columns_spaced(0.0, weights, f);
    }

    /// Same as `columns` with explicit inter-column gap.
    pub fn columns_spaced(
        &mut self,
        gap: f32,
        weights: &[f32],
        mut f: impl FnMut(&mut Self, usize),
    ) {
        if weights.is_empty() {
            return;
        }
        let total_weight: f32 = weights.iter().sum();
        if total_weight <= 0.0 {
            return;
        }

        let n = weights.len();
        let total_gap = gap * (n as f32 - 1.0).max(0.0);
        let available = self.region.w - total_gap;

        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Horizontal,
                gap,
                ..Default::default()
            },
        );

        let saved_cursor = self.cursor;
        let saved_region = self.region;
        let saved_spacing = self.spacing;

        let mut col_x = self.cursor.x;
        let mut max_height: f32 = 0.0;

        for (i, &w) in weights.iter().enumerate() {
            let col_w = available * w / total_weight;

            self.tree_build.open_container(
                None,
                LayoutStyle {
                    direction: Direction::Vertical,
                    gap: self.spacing,
                    flex_grow: w / total_weight,
                    ..Default::default()
                },
            );

            self.cursor = Vec2 {
                x: col_x,
                y: saved_cursor.y,
            };
            self.region = Rect::new(col_x, saved_region.y, col_w, saved_region.h);
            self.spacing = saved_spacing;

            let start_y = self.cursor.y;
            f(self, i);
            let col_height = self.cursor.y - start_y;
            if col_height > max_height {
                max_height = col_height;
            }

            col_x += col_w + gap;
            self.tree_build.close_container();
        }

        self.cursor = saved_cursor;
        self.cursor.y += max_height;
        self.region = saved_region;
        self.spacing = saved_spacing;
        self.tree_build.close_container();
    }

    // ── Constrained layout ──

    /// Run a closure within layout constraints.
    pub fn constrained(&mut self, c: layout::Constraints, f: impl FnOnce(&mut Self)) {
        self.tree_build.open_container(
            None,
            LayoutStyle {
                min_width: c.min_width,
                max_width: c.max_width,
                min_height: c.min_height,
                max_height: c.max_height,
                direction: Direction::Vertical,
                gap: self.spacing,
                ..Default::default()
            },
        );
        let saved_cursor = self.cursor;
        let saved_region = self.region;

        let (cw, _) = c.apply(self.region.w, self.region.h);
        self.region = Rect::new(self.cursor.x, self.cursor.y, cw, self.region.h);

        f(self);

        let consumed_h = self.cursor.y - saved_cursor.y;
        let (_, ch) = c.apply(cw, consumed_h);

        self.cursor.x = saved_cursor.x;
        self.cursor.y = saved_cursor.y + ch;
        self.region = saved_region;
        self.tree_build.close_container();
    }

    // ── Tree indent ──

    /// Indent children of an expanded tree node.
    pub fn tree_indent(&mut self, f: impl FnOnce(&mut Self)) {
        let indent = self.theme.tree_indent;
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                padding: Spacing {
                    left: indent,
                    ..Default::default()
                },
                gap: self.spacing,
                ..Default::default()
            },
        );
        let saved_cursor_x = self.cursor.x;
        let saved_region = self.region;

        let guide_x = self.cursor.x + indent / 2.0;
        let guide_start_y = self.cursor.y;

        self.cursor.x += indent;
        self.region = Rect::new(
            self.cursor.x,
            self.region.y,
            self.region.w - indent,
            self.region.h,
        );

        f(self);

        // Draw vertical guide line spanning the children.
        let guide_h = self.cursor.y - guide_start_y;
        if guide_h > 0.0 {
            paint::draw_vline(
                self.frame,
                guide_x,
                guide_start_y,
                guide_h,
                self.theme.border,
            );
        }

        self.cursor.x = saved_cursor_x;
        self.region = saved_region;
        self.tree_build.close_container();
    }

    /// Animated tree indent — draws children with a clip-rect height animation.
    ///
    /// `anim_id` should be a unique ID for this tree node (used for the animation).
    /// `expanded` is the current expand state. Children are always drawn (for
    /// measurement), but clipped during the animation.
    pub fn animated_tree_indent(
        &mut self,
        anim_id: u64,
        expanded: bool,
        f: impl FnOnce(&mut Self),
    ) {
        let indent = self.theme.tree_indent;
        let duration = self.theme.tree_expand_duration_ms;
        let target = if expanded { 1.0 } else { 0.0 };
        let t = self
            .state
            .anim_t(anim_id, target, duration, state::Easing::EaseOutCubic);

        // If fully collapsed and animation settled, skip drawing entirely.
        if t < 0.001 && !expanded {
            return;
        }

        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                padding: Spacing {
                    left: indent,
                    ..Default::default()
                },
                gap: self.spacing,
                ..Default::default()
            },
        );
        let saved_cursor_x = self.cursor.x;
        let saved_cursor_y = self.cursor.y;
        let saved_region = self.region;
        let saved_clip = self.frame.active_clip();

        self.cursor.x += indent;
        self.region = Rect::new(
            self.cursor.x,
            self.region.y,
            self.region.w - indent,
            self.region.h,
        );

        let start_y = self.cursor.y;

        // Set clip rect if animating (0 < t < 1).
        let cached_h = self
            .state
            .tree_children_heights
            .get(&anim_id)
            .copied()
            .unwrap_or(0.0);
        if t < 0.999 {
            let clip_h = cached_h * t;
            self.frame
                .set_active_clip(Some([saved_cursor_x, start_y, saved_region.w, clip_h]));
        }

        let guide_x = saved_cursor_x + indent / 2.0;

        f(self);

        let children_h = self.cursor.y - start_y;
        self.state.tree_children_heights.insert(anim_id, children_h);

        // Draw vertical guide line spanning the children (inside clip rect).
        let guide_h = self.cursor.y - start_y;
        if guide_h > 0.0 {
            let border_color = self.theme.border;
            paint::draw_vline(self.frame, guide_x, start_y, guide_h, border_color);
        }

        // Restore clip.
        self.frame.set_active_clip(saved_clip);

        // Advance cursor by animated height.
        let visible_h = if t >= 0.999 { children_h } else { cached_h * t };

        self.cursor.x = saved_cursor_x;
        self.cursor.y = saved_cursor_y + visible_h;
        self.region = saved_region;
        self.tree_build.close_container();
    }

    // ── Drag and Drop ──

    /// Make a widget draggable. Call after the widget.
    /// Returns true when drag starts this frame.
    pub fn drag_source(&mut self, id: u64, payload: u64) -> bool {
        if self.disabled {
            return false;
        }

        // On mouse press, record drag start position.
        if let Some((cx, cy, _)) = self.state.mouse.pending_click {
            // Check if click is on this widget.
            if let Some((rect, _, _)) = self.state.hit_rects.iter().find(|(_, wid, _)| *wid == id) {
                if rect.contains(cx, cy) && self.state.drag.is_none() {
                    self.state.drag_start = Some((cx, cy));
                }
            }
        }

        // Check dead zone — start drag when mouse moves >4px from press.
        if self.state.drag.is_none() && self.state.mouse_pressed {
            if let Some((sx, sy)) = self.state.drag_start {
                if let Some((rect, _, _)) =
                    self.state.hit_rects.iter().find(|(_, wid, _)| *wid == id)
                {
                    let dx = self.state.mouse.x - sx;
                    let dy = self.state.mouse.y - sy;
                    if dx * dx + dy * dy > 16.0 {
                        self.state.drag = Some(state::DragPayload {
                            source_id: id,
                            payload,
                            x: self.state.mouse.x,
                            y: self.state.mouse.y,
                            offset_x: sx - rect.x,
                            offset_y: sy - rect.y,
                        });
                        return true;
                    }
                }
            }
        }

        // Update drag position.
        if let Some(ref mut d) = self.state.drag {
            if d.source_id == id {
                d.x = self.state.mouse.x;
                d.y = self.state.mouse.y;
            }
        }

        false
    }

    /// Check if a drag is hovering over this rect. Returns payload if so.
    pub fn drop_target(&self, rect: Rect) -> Option<u64> {
        if let Some(ref d) = self.state.drag {
            if rect.contains(d.x, d.y) {
                return Some(d.payload);
            }
        }
        None
    }

    /// Check if a drop just completed on this rect. Returns payload.
    /// Only returns Some on the frame when mouse was released over target.
    pub fn accept_drop(&self, rect: Rect) -> Option<u64> {
        if let Some(ref d) = self.state.drag {
            if !self.state.mouse_pressed && rect.contains(d.x, d.y) {
                return Some(d.payload);
            }
        }
        None
    }

    // ── Interaction helpers (used by widgets) ──

    /// Register a widget for hit testing and focus chain.
    ///
    /// When `hit_clip` is active, the hit rect is intersected with it so
    /// widgets scrolled out of view don't receive clicks. The widget is
    /// still added to the focus chain (Tab still works).
    ///
    /// When disabled, skips both hit_rects and focus_chain — no cursor
    /// icon change, no Tab focus, no click consumption.
    pub fn register_widget(&mut self, id: u64, rect: Rect, kind: state::WidgetKind) {
        if self.disabled {
            return;
        }
        if let Some(clip) = &self.hit_clip {
            if let Some(clipped) = rect.intersect(clip) {
                self.state.hit_rects.push((clipped, id, kind));
            }
            // Skip hit_rects push if no intersection, but still add to focus chain.
        } else {
            self.state.hit_rects.push((rect, id, kind));
        }
        self.state.focus_chain.push(id);

        // Track the focused widget's kind for keyboard activation.
        if self.state.focused == Some(id) {
            self.state.focused_kind = Some(kind);
        }

        // Track rects for widgets with active animations (for targeted damage).
        if self.state.hover_anims.contains_key(&id) || self.state.anims.contains_key(&id) {
            self.state.anim_rects.insert(id, rect);
        }
    }

    /// Compute the Response for a widget given its ID and rect.
    /// When disabled, returns an inert Response with `disabled: true`.
    pub fn widget_response(&mut self, id: u64, rect: Rect) -> response::Response {
        if self.disabled {
            return response::Response {
                clicked: false,
                right_clicked: false,
                hovered: false,
                pressed: false,
                focused: false,
                changed: false,
                disabled: true,
            };
        }
        // Intersect with hit_clip so widgets outside the visible scroll area
        // don't respond to hover/click.
        let effective = match &self.hit_clip {
            Some(clip) => match rect.intersect(clip) {
                Some(r) => r,
                None => {
                    // Completely clipped — not hovered, not clickable.
                    return response::Response {
                        clicked: false,
                        right_clicked: false,
                        hovered: false,
                        pressed: false,
                        focused: self.state.focused == Some(id),
                        changed: false,
                        disabled: false,
                    };
                }
            },
            None => rect,
        };
        let hovered = effective.contains(self.state.mouse.x, self.state.mouse.y);
        let focused = self.state.focused == Some(id);

        let mut clicked = false;
        if let Some((cx, cy, ref mut consumed)) = self.state.mouse.pending_click {
            if !*consumed && effective.contains(cx, cy) {
                clicked = true;
                *consumed = true;
                self.state.focused = Some(id);
                self.state.reset_blink();
            }
        }

        let mut right_clicked = false;
        if let Some((cx, cy, ref mut consumed)) = self.state.mouse.pending_right_click {
            if !*consumed && effective.contains(cx, cy) {
                right_clicked = true;
                *consumed = true;
            }
        }

        // Keyboard activation: Enter/Space triggers click for activatable widgets.
        if !clicked && focused && !self.disabled {
            use esox_input::{Key, NamedKey};
            let activatable = matches!(
                self.state.focused_kind,
                Some(state::WidgetKind::Button)
                    | Some(state::WidgetKind::Checkbox)
                    | Some(state::WidgetKind::Toggle)
                    | Some(state::WidgetKind::Radio)
                    | Some(state::WidgetKind::Hyperlink)
                    | Some(state::WidgetKind::Tab)
            );
            if activatable {
                for (event, _) in &self.state.keys {
                    if event.pressed
                        && matches!(
                            event.key,
                            Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space)
                        )
                    {
                        clicked = true;
                        self.state.focused = Some(id);
                        self.state.reset_blink();
                        break;
                    }
                }
            }
        }

        let pressed = hovered && self.state.mouse_pressed;

        response::Response {
            clicked,
            right_clicked,
            hovered,
            pressed,
            focused,
            changed: false,
            disabled: false,
        }
    }

    /// Check if a point is hovered over a rect.
    pub fn is_hovered(&self, rect: Rect) -> bool {
        rect.contains(self.state.mouse.x, self.state.mouse.y)
    }

    /// Borrow rendering resources for custom widget drawing.
    pub fn painter(&mut self) -> Painter<'_> {
        Painter {
            frame: self.frame,
            text: self.text,
            gpu: self.gpu,
            resources: self.resources,
            theme: self.theme,
        }
    }

    /// Get a hover animation value for a custom widget.
    ///
    /// Convenience wrapper around [`UiState::hover_t`]. Pass a unique `id`
    /// (typically `widget_id ^ HOVER_SALT`), the current `hovered` state,
    /// and the animation duration in milliseconds. Returns 0.0–1.0.
    pub fn hover_t(&mut self, id: u64, hovered: bool, duration_ms: f32) -> f32 {
        self.state.hover_t(id, hovered, duration_ms)
    }

    /// Set the disabled flag directly.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Run a closure with widgets disabled (or enabled). Restores previous state after.
    pub fn disabled(&mut self, disabled: bool, f: impl FnOnce(&mut Self)) {
        let prev = self.disabled;
        self.disabled = disabled;
        f(self);
        self.disabled = prev;
    }

    /// Whether the UI is currently in disabled mode.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    // ── Widget Style Overrides ──

    /// Run a closure with a widget style override pushed onto the stack.
    pub fn with_style(&mut self, style: WidgetStyle, f: impl FnOnce(&mut Self)) {
        self.style_stack.push(style);
        f(self);
        self.style_stack.pop();
    }

    /// Run a closure with a 2D transform applied to all emitted GPU instances.
    ///
    /// Translation is always applied. Scaling is relative to the visual center
    /// of the content emitted by the closure.
    pub fn with_transform(&mut self, t: theme::Transform2D, f: impl FnOnce(&mut Self)) {
        let start = self.frame.instance_len();
        let start_y = self.cursor.y;
        f(self);
        let end = self.frame.instance_len();

        if start == end {
            return;
        }

        let center_x = self.region.x + self.region.w * 0.5;
        let center_y = (start_y + self.cursor.y) * 0.5;

        self.frame.transform_instances(
            start,
            end,
            t.translate_x,
            t.translate_y,
            t.scale_x,
            t.scale_y,
            center_x,
            center_y,
        );
    }

    /// Run a closure with GPU clipping: children that overflow the current
    /// region are visually clipped (like CSS `overflow: hidden`).
    pub fn clip_children(&mut self, f: impl FnOnce(&mut Self)) {
        let clip_rect =
            layout::Rect::new(self.cursor.x, self.cursor.y, self.region.w, self.region.h);
        let saved_clip = self.frame.active_clip();
        let gpu_clip = Self::intersect_gpu_clip(saved_clip, clip_rect);
        self.frame.set_active_clip(Some(gpu_clip.to_clip_array()));
        f(self);
        self.frame.set_active_clip(saved_clip);
    }

    // ── Scroll Helpers ──

    /// Draw top/bottom scroll fade gradients for a scrollable container.
    ///
    /// Uses the container's own clip (not the parent-intersected clip) so
    /// fade overlays aren't incorrectly shrunk by ancestor clips.
    ///
    /// * `container_clip` — the unclipped container rect (used as clip for fades)
    /// * `content_x` — left edge of the content area (excludes scrollbar)
    /// * `content_w` — width of the content area (excludes scrollbar)
    /// * `visible_h` — viewport height
    /// * `scroll_offset` — current scroll position
    /// * `max_scroll` — maximum scroll offset
    #[allow(dead_code)]
    pub(crate) fn draw_scroll_fades(
        &mut self,
        container_clip: Rect,
        content_x: f32,
        content_w: f32,
        visible_h: f32,
        scroll_offset: f32,
        max_scroll: f32,
    ) {
        let fade_h = self.theme.scroll_fade_height;
        if fade_h <= 0.0 {
            return;
        }

        self.frame
            .set_active_clip(Some(container_clip.to_clip_array()));

        let bg = self.theme.bg_base;
        // Top fade: visible when scrolled down.
        if scroll_offset > 0.5 {
            paint::draw_scroll_fade(
                self.frame,
                Rect::new(content_x, container_clip.y, content_w, fade_h),
                bg,
                bg.with_alpha(0.0),
                std::f32::consts::FRAC_PI_2,
            );
        }
        // Bottom fade: visible when content extends below viewport.
        if scroll_offset < max_scroll - 0.5 {
            paint::draw_scroll_fade(
                self.frame,
                Rect::new(
                    content_x,
                    container_clip.y + visible_h - fade_h,
                    content_w,
                    fade_h,
                ),
                bg.with_alpha(0.0),
                bg,
                std::f32::consts::FRAC_PI_2,
            );
        }
    }

    // ── Sub-Region API ──

    /// Intersect a rect with the current GPU clip, returning the visible portion.
    ///
    /// If there is no active clip, `rect` is returned unchanged. When the rects
    /// don't overlap at all, returns `Rect::ZERO` — this prevents completely
    /// clipped widgets from rendering or receiving input.
    ///
    /// This is the single source of truth for GPU clip intersection — all
    /// container widgets should use this instead of hand-rolling the intersection.
    fn intersect_gpu_clip(saved_clip: Option<[f32; 4]>, rect: Rect) -> Rect {
        match saved_clip {
            Some(prev) => rect
                .intersect(&Rect::from_clip_array(prev))
                .unwrap_or(Rect::ZERO),
            None => rect,
        }
    }

    /// Intersect a rect with the current hit-test clip, returning the visible
    /// portion. If there is no active hit clip, `rect` is returned unchanged.
    /// Returns `Rect::ZERO` when the rects don't overlap.
    fn intersect_hit_clip(saved_hit_clip: Option<Rect>, rect: Rect) -> Rect {
        match saved_hit_clip {
            Some(prev) => rect.intersect(&prev).unwrap_or(Rect::ZERO),
            None => rect,
        }
    }

    /// Save a complete snapshot of the current layout state.
    fn save_layout_state(&self) -> SavedLayoutState {
        SavedLayoutState {
            cursor: self.cursor,
            region: self.region,
            spacing: self.spacing,
            gpu_clip: self.frame.active_clip(),
            hit_clip: self.hit_clip,
        }
    }

    /// Restore layout state from a snapshot.
    fn restore_layout_state(&mut self, saved: &SavedLayoutState) {
        self.frame.set_active_clip(saved.gpu_clip);
        self.hit_clip = saved.hit_clip;
        self.cursor = saved.cursor;
        self.region = saved.region;
        self.spacing = saved.spacing;
    }

    /// Run a closure inside a scoped sub-region with automatic save/restore of
    /// all layout state (cursor, region, spacing, GPU clip, hit clip).
    ///
    /// This is the primary building block for container widgets. It:
    /// 1. Saves all layout state
    /// 2. Sets cursor/region to the given `rect` (with optional `inset`)
    /// 3. Sets GPU clip and hit clip to `rect` (intersected with parent clips)
    /// 4. Runs the closure
    /// 5. Restores all layout state
    /// 6. Advances the cursor past the sub-region (vertical direction only,
    ///    by `rect.h + spacing`), unless `advance` is `false`
    ///
    /// # Arguments
    /// * `rect` — The bounding rectangle for this sub-region.
    /// * `inset` — Horizontal inset applied to cursor and region within `rect`
    ///   (e.g. `theme.spacing_unit` to prevent glyph clipping at scissor edges).
    /// * `clip` — Whether to set GPU clip and hit clip to `rect`.
    /// * `advance` — Whether to advance `cursor.y` past the region on exit.
    /// * `f` — The content closure.
    pub fn sub_region(
        &mut self,
        rect: Rect,
        inset: f32,
        clip: bool,
        advance: bool,
        f: impl FnOnce(&mut Self),
    ) {
        let saved = self.save_layout_state();

        // Open a tree container so flex children (e.g. scrollable_fill) can
        // properly fill remaining space within this sub-region.
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap: self.spacing,
                max_height: Some(rect.h),
                ..Default::default()
            },
        );

        // Set up clip regions.
        if clip {
            let gpu_clip = Self::intersect_gpu_clip(saved.gpu_clip, rect);
            self.frame.set_active_clip(Some(gpu_clip.to_clip_array()));
            self.hit_clip = Some(Self::intersect_hit_clip(saved.hit_clip, rect));
        }

        // Set cursor and region with optional inset.
        self.cursor = Vec2 {
            x: rect.x + inset,
            y: rect.y,
        };
        self.region = Rect::new(
            rect.x + inset,
            rect.y,
            (rect.w - inset * 2.0).max(0.0),
            rect.h,
        );

        f(self);

        self.tree_build.close_container();
        self.restore_layout_state(&saved);

        if advance {
            self.cursor.y += rect.h + self.spacing;
        }
    }

    /// Variant of [`sub_region`](Ui::sub_region) that calls a `FnMut` closure
    /// with a panel index, for cases like split panes where both panels share
    /// mutable state.
    pub fn sub_region_indexed(
        &mut self,
        rect: Rect,
        inset: f32,
        clip: bool,
        index: usize,
        f: &mut dyn FnMut(&mut Self, usize),
    ) {
        let saved = self.save_layout_state();

        // Open a tree container so flex children (e.g. scrollable_fill) can
        // properly fill remaining space within this sub-region.
        self.tree_build.open_container(
            None,
            LayoutStyle {
                direction: Direction::Vertical,
                gap: self.spacing,
                max_height: Some(rect.h),
                ..Default::default()
            },
        );

        if clip {
            let gpu_clip = Self::intersect_gpu_clip(saved.gpu_clip, rect);
            self.frame.set_active_clip(Some(gpu_clip.to_clip_array()));
            self.hit_clip = Some(Self::intersect_hit_clip(saved.hit_clip, rect));
        }

        self.cursor = Vec2 {
            x: rect.x + inset,
            y: rect.y,
        };
        self.region = Rect::new(
            rect.x + inset,
            rect.y,
            (rect.w - inset * 2.0).max(0.0),
            rect.h,
        );

        f(self, index);

        self.tree_build.close_container();
        self.restore_layout_state(&saved);
    }

    /// Resolve foreground color: style stack override or theme default.
    pub(crate) fn resolve_fg(&self) -> Color {
        for s in self.style_stack.iter().rev() {
            if let Some(c) = s.fg {
                return c;
            }
        }
        // Fallback: check inherited text_color for label-type widgets.
        for s in self.style_stack.iter().rev() {
            if let Some(c) = s.text_color {
                return c;
            }
        }
        // Default to fg_on_accent for interactive widgets (buttons) since
        // resolve_bg defaults to the accent color.
        self.theme.fg_on_accent
    }

    /// Resolve background color for interactive widgets.
    pub(crate) fn resolve_bg(&self) -> Color {
        for s in self.style_stack.iter().rev() {
            if let Some(c) = s.bg {
                return c;
            }
        }
        self.theme.accent
    }

    /// Resolve font size: style stack override or theme default.
    pub(crate) fn resolve_font_size(&self) -> f32 {
        for s in self.style_stack.iter().rev() {
            if let Some(v) = s.font_size {
                return v;
            }
        }
        self.theme.font_size
    }

    /// Resolve corner radius: style stack override or theme default.
    pub(crate) fn resolve_corner_radius(&self) -> f32 {
        for s in self.style_stack.iter().rev() {
            if let Some(v) = s.corner_radius {
                return v;
            }
        }
        self.theme.corner_radius
    }

    /// Resolve button height: style stack override or theme default.
    pub(crate) fn resolve_height(&self) -> f32 {
        for s in self.style_stack.iter().rev() {
            if let Some(v) = s.height {
                return v;
            }
        }
        self.theme.button_height
    }

    /// Resolve border color: style stack override or theme default.
    #[allow(dead_code)]
    pub(crate) fn resolve_border_color(&self) -> Color {
        for s in self.style_stack.iter().rev() {
            if let Some(c) = s.border_color {
                return c;
            }
        }
        self.theme.border
    }

    /// Resolve padding override from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_padding(&self) -> Option<layout::Spacing> {
        for s in self.style_stack.iter().rev() {
            if let Some(p) = s.padding {
                return Some(p);
            }
        }
        None
    }

    /// Resolve margin override from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_margin(&self) -> Option<layout::Spacing> {
        for s in self.style_stack.iter().rev() {
            if let Some(m) = s.margin {
                return Some(m);
            }
        }
        None
    }

    /// Resolve border stroke width: style stack override or 1.0.
    #[allow(dead_code)]
    pub(crate) fn resolve_border_width(&self) -> f32 {
        for s in self.style_stack.iter().rev() {
            if let Some(w) = s.border_width {
                return w;
            }
        }
        1.0
    }

    /// Resolve opacity: style stack override or 1.0 (fully opaque).
    #[allow(dead_code)]
    pub(crate) fn resolve_opacity(&self) -> f32 {
        for s in self.style_stack.iter().rev() {
            if let Some(o) = s.opacity {
                return o;
            }
        }
        1.0
    }

    /// Resolve elevation/shadow override from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_elevation(&self) -> Option<&theme::Elevation> {
        for s in self.style_stack.iter().rev() {
            if let Some(ref e) = s.elevation {
                return Some(e);
            }
        }
        None
    }

    /// Resolve text alignment: style stack override or Left.
    pub(crate) fn resolve_text_align(&self) -> theme::TextAlign {
        for s in self.style_stack.iter().rev() {
            if let Some(a) = s.text_align {
                return a;
            }
        }
        theme::TextAlign::Left
    }

    /// Resolve minimum width constraint from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_min_width(&self) -> Option<f32> {
        for s in self.style_stack.iter().rev() {
            if s.min_width.is_some() {
                return s.min_width;
            }
        }
        None
    }

    /// Resolve maximum width constraint from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_max_width(&self) -> Option<f32> {
        for s in self.style_stack.iter().rev() {
            if s.max_width.is_some() {
                return s.max_width;
            }
        }
        None
    }

    /// Resolve minimum height constraint from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_min_height(&self) -> Option<f32> {
        for s in self.style_stack.iter().rev() {
            if s.min_height.is_some() {
                return s.min_height;
            }
        }
        None
    }

    /// Resolve maximum height constraint from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_max_height(&self) -> Option<f32> {
        for s in self.style_stack.iter().rev() {
            if s.max_height.is_some() {
                return s.max_height;
            }
        }
        None
    }

    /// Resolve gradient override from the style stack.
    #[allow(dead_code)]
    pub(crate) fn resolve_gradient(&self) -> Option<theme::Gradient> {
        for s in self.style_stack.iter().rev() {
            if let Some(g) = s.gradient {
                return Some(g);
            }
        }
        None
    }

    /// Resolve per-corner border radius: style stack override, or uniform from corner_radius.
    #[allow(dead_code)]
    pub(crate) fn resolve_border_radius(&self) -> esox_gfx::BorderRadius {
        for s in self.style_stack.iter().rev() {
            if let Some([tl, tr, bl, br]) = s.per_corner_radius {
                return esox_gfx::BorderRadius {
                    top_left: tl,
                    top_right: tr,
                    bottom_left: bl,
                    bottom_right: br,
                };
            }
        }
        esox_gfx::BorderRadius::uniform(self.resolve_corner_radius())
    }

    /// Resolve text decoration: style stack override or None.
    pub(crate) fn resolve_text_decoration(&self) -> theme::TextDecoration {
        for s in self.style_stack.iter().rev() {
            if let Some(d) = s.text_decoration {
                return d;
            }
        }
        theme::TextDecoration::None
    }

    /// Resolve text transform: style stack override or None.
    pub(crate) fn resolve_text_transform(&self) -> theme::TextTransform {
        for s in self.style_stack.iter().rev() {
            if let Some(t) = s.text_transform {
                return t;
            }
        }
        theme::TextTransform::None
    }

    /// Access the theme.
    /// Whether the debug overlay is currently enabled.
    pub fn is_debug_overlay(&self) -> bool {
        self.state.debug_overlay
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    // ── Convenience Combinators ──

    /// Scrollable + padding + max_width in one call (common page layout).
    pub fn page(&mut self, id: u64, scroll_h: f32, max_w: f32, f: impl FnOnce(&mut Self)) {
        self.scrollable(id, scroll_h, |ui| {
            ui.max_width(max_w, |ui| {
                ui.padding(ui.theme.padding, f);
            });
        });
    }

    /// Label-widget pair in a row: "Label    [widget]"
    pub fn labeled(&mut self, label: &str, f: impl FnOnce(&mut Self)) {
        self.row(|ui| {
            let label_w = ui.text.measure_text(label, ui.theme.font_size);
            let rect = ui.allocate_rect(label_w, ui.theme.button_height);
            ui.text.draw_ui_text(
                label,
                rect.x,
                rect.y + (rect.h - ui.theme.font_size) / 2.0,
                ui.theme.fg_label,
                ui.frame,
                ui.gpu,
                ui.resources,
            );
            f(ui);
        });
    }

    // ── Tooltip ──

    /// Attach a tooltip to the widget with the given ID. Call after the widget.
    pub fn tooltip(&mut self, id: u64, text: &str) {
        // Find widget rect from hit_rects.
        let anchor = match self.state.hit_rects.iter().find(|(_, wid, _)| *wid == id) {
            Some((rect, _, _)) => *rect,
            None => return, // disabled or not found
        };

        let hovered = anchor.contains(self.state.mouse.x, self.state.mouse.y);

        if hovered {
            match &mut self.state.tooltip {
                Some(tt) if tt.widget_id == id => {
                    // Same widget — check delay.
                    if !tt.visible {
                        let elapsed = tt.hover_start.elapsed().as_millis() as u64;
                        if elapsed >= self.theme.tooltip_delay_ms {
                            tt.visible = true;
                        }
                    }
                    tt.anchor = anchor;
                }
                _ => {
                    // New widget or no tooltip — reset timer.
                    self.state.tooltip = Some(state::TooltipState {
                        widget_id: id,
                        hover_start: std::time::Instant::now(),
                        anchor,
                        text: text.to_string(),
                        visible: false,
                    });
                }
            }
        } else if self
            .state
            .tooltip
            .as_ref()
            .is_some_and(|tt| tt.widget_id == id)
        {
            self.state.tooltip = None;
        }
    }

    /// Draw the tooltip if visible. Called from `finish()`.
    fn draw_tooltip(&mut self) {
        let (text, anchor) = match &self.state.tooltip {
            Some(tt) if tt.visible => (tt.text.clone(), tt.anchor),
            _ => return,
        };

        let font_size = self.theme.tooltip_font_size;
        let pad = self.theme.tooltip_padding;
        let text_w = self.text.measure_text(&text, font_size);
        let tooltip_w = text_w + pad * 2.0;
        let tooltip_h = font_size + pad * 2.0;

        // Position below the anchor, centered, clamped to viewport.
        let gap = 4.0;
        let mut tx = anchor.x + (anchor.w - tooltip_w) / 2.0;
        let mut ty = anchor.y + anchor.h + gap;

        // Clamp to viewport.
        if tx < self.region.x {
            tx = self.region.x;
        }
        if tx + tooltip_w > self.region.x + self.region.w {
            tx = self.region.x + self.region.w - tooltip_w;
        }
        if ty + tooltip_h > self.region.y + self.region.h {
            // Show above instead.
            ty = anchor.y - tooltip_h - gap;
        }

        let tt_rect = Rect::new(tx, ty, tooltip_w, tooltip_h);

        // Background + elevation shadow.
        paint::draw_elevated_rect(
            self.frame,
            tt_rect,
            self.theme.tooltip_bg,
            4.0,
            &self.theme.elevation_medium,
        );

        // Text.
        self.text.draw_text(
            &text,
            tx + pad,
            ty + pad,
            font_size,
            self.theme.tooltip_fg,
            self.frame,
            self.gpu,
            self.resources,
        );
    }

    /// Draw debug overlay outlines around all allocated rects. Called from `finish()`.
    fn draw_debug_overlay(&mut self) {
        if !self.state.debug_overlay {
            return;
        }

        // Depth-cycling colors for layout bounds.
        const DEPTH_COLORS: [[f32; 4]; 4] = [
            [0.2, 0.8, 0.2, 0.3], // green
            [0.2, 0.5, 1.0, 0.3], // blue
            [1.0, 0.6, 0.1, 0.3], // orange
            [0.8, 0.2, 0.8, 0.3], // purple
        ];

        let rects: Vec<_> = self.state.debug_widget_rects.drain(..).collect();
        for (i, (rect, _id, _kind)) in rects.iter().enumerate() {
            let color_idx = i % DEPTH_COLORS.len();
            let [r, g, b, a] = DEPTH_COLORS[color_idx];
            let outline_color = Color::new(r, g, b, a);
            // Draw 1px outline.
            let t = 1.0;
            // Top
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x, rect.y, rect.w, t),
                outline_color,
                0.0,
            );
            // Bottom
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x, rect.y + rect.h - t, rect.w, t),
                outline_color,
                0.0,
            );
            // Left
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x, rect.y, t, rect.h),
                outline_color,
                0.0,
            );
            // Right
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x + rect.w - t, rect.y, t, rect.h),
                outline_color,
                0.0,
            );
        }

        // Hovered widget gets a tooltip with rect info.
        let mouse_x = self.state.mouse.x;
        let mouse_y = self.state.mouse.y;
        if let Some((rect, id, kind)) = rects
            .iter()
            .rev()
            .find(|(r, _, _)| r.contains(mouse_x, mouse_y))
        {
            // Alt+click: log widget info instead of sending click.
            if self.state.modifiers.alt() {
                if let Some((_, _, ref mut consumed)) = self.state.mouse.pending_click {
                    if !*consumed {
                        *consumed = true;
                        tracing::info!(
                            id = id,
                            kind = kind,
                            x = rect.x,
                            y = rect.y,
                            w = rect.w,
                            h = rect.h,
                            "debug: click-to-inspect widget"
                        );
                    }
                }
            }

            // Draw highlighted outline on hovered widget.
            let highlight = Color::new(1.0, 1.0, 0.0, 0.5);
            let t = 2.0;
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x, rect.y, rect.w, t),
                highlight,
                0.0,
            );
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x, rect.y + rect.h - t, rect.w, t),
                highlight,
                0.0,
            );
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x, rect.y, t, rect.h),
                highlight,
                0.0,
            );
            paint::draw_rounded_rect(
                self.frame,
                Rect::new(rect.x + rect.w - t, rect.y, t, rect.h),
                highlight,
                0.0,
            );

            // Info tooltip.
            let info = format!(
                "{kind} [{:.0}x{:.0}] @({:.0},{:.0})",
                rect.w, rect.h, rect.x, rect.y
            );
            let font_size = self.theme.tooltip_font_size;
            let pad = 4.0;
            let text_w = self.text.measure_text(&info, font_size);
            let tw = text_w + pad * 2.0;
            let th = font_size + pad * 2.0;
            let tx = (mouse_x + 12.0).min(self.region.x + self.region.w - tw);
            let ty = (mouse_y + 12.0).min(self.region.y + self.region.h - th);

            paint::draw_rounded_rect(
                self.frame,
                Rect::new(tx, ty, tw, th),
                Color::new(0.0, 0.0, 0.0, 0.85),
                3.0,
            );
            self.text.draw_text(
                &info,
                tx + pad,
                ty + pad,
                font_size,
                Color::new(1.0, 1.0, 0.0, 1.0),
                self.frame,
                self.gpu,
                self.resources,
            );
        }
    }

    // ── Context Menu ──

    // ── Toast convenience ──

    /// Show an info toast notification.
    pub fn toast_info(&mut self, msg: &str) {
        let dur = self.theme.toast_duration_ms;
        self.state
            .toasts
            .push(state::ToastKind::Info, msg.to_string(), dur);
    }

    /// Show a success toast notification.
    pub fn toast_success(&mut self, msg: &str) {
        let dur = self.theme.toast_duration_ms;
        self.state
            .toasts
            .push(state::ToastKind::Success, msg.to_string(), dur);
    }

    /// Show an error toast notification.
    pub fn toast_error(&mut self, msg: &str) {
        let dur = self.theme.toast_duration_ms;
        self.state
            .toasts
            .push(state::ToastKind::Error, msg.to_string(), dur);
    }

    /// Show a warning toast notification.
    pub fn toast_warning(&mut self, msg: &str) {
        let dur = self.theme.toast_duration_ms;
        self.state
            .toasts
            .push(state::ToastKind::Warning, msg.to_string(), dur);
    }

    /// Show a toast with custom kind and duration.
    pub fn toast_custom(&mut self, kind: state::ToastKind, msg: &str, duration_ms: u64) {
        self.state.toasts.push(kind, msg.to_string(), duration_ms);
    }

    // ── Accessibility ──

    /// Set a pending accessibility label for the next widget.
    pub fn a11y_label(&mut self, label: &str) {
        if self.state.a11y_enabled {
            self.state.a11y_pending_label = Some(label.to_string());
        }
    }

    /// Set a pending accessibility role for the next widget.
    pub fn a11y_role(&mut self, role: state::A11yRole) {
        if self.state.a11y_enabled {
            self.state.a11y_pending_role = Some(role);
        }
    }

    /// Consume pending a11y label/role (called by widgets after register_widget).
    #[allow(dead_code)]
    pub(crate) fn consume_a11y(&mut self) -> (Option<String>, Option<state::A11yRole>) {
        (
            self.state.a11y_pending_label.take(),
            self.state.a11y_pending_role.take(),
        )
    }

    /// Push an accessibility node into the frame's a11y tree.
    ///
    /// Widgets call this after `register_widget` to emit their a11y representation.
    /// If a11y is disabled, this is a no-op.
    pub fn push_a11y_node(&mut self, node: state::A11yNode) {
        if self.state.a11y_enabled {
            self.state.a11y_tree.push(node);
        }
    }

    // ── Animation API ──

    /// Drive a custom animation. Returns the current interpolated value.
    ///
    /// `id` identifies the animation (use `id!()` or `fnv1a_mix`).
    /// `target` is the value to animate towards.
    /// On first call, starts settled at `target`. When `target` changes,
    /// restarts from the current value (smooth retargeting).
    pub fn animate(&mut self, id: u64, target: f32, duration_ms: f32, easing: Easing) -> f32 {
        self.state.anim_t(id, target, duration_ms, easing)
    }

    /// Boolean animation helper. Returns 0.0→1.0 interpolation.
    ///
    /// Equivalent to `animate(id, if active { 1.0 } else { 0.0 }, ...)`.
    pub fn animate_bool(&mut self, id: u64, active: bool, duration_ms: f32, easing: Easing) -> f32 {
        let target = if active { 1.0 } else { 0.0 };
        self.state.anim_t(id, target, duration_ms, easing)
    }

    /// Whether the given animation is currently in-flight (not settled).
    pub fn is_animating(&self, id: u64) -> bool {
        self.state.anim_active(id)
    }

    /// Drive a spring-based animation. Returns the current value.
    ///
    /// Unlike duration-based `animate()`, springs settle naturally based on
    /// physics. Changing `target` mid-flight preserves velocity for a smooth
    /// feel.
    ///
    /// Use [`SpringConfig`] presets for common behaviors:
    /// - `SpringConfig::SNAPPY` — fast, no overshoot (toggles, hovers)
    /// - `SpringConfig::GENTLE` — smooth, slower (layout transitions)
    /// - `SpringConfig::BOUNCY` — visible overshoot (enter/exit)
    /// - `SpringConfig::STIFF` — very fast (micro-interactions)
    pub fn animate_spring(&mut self, id: u64, target: f32, config: SpringConfig) -> f32 {
        self.state.spring_t(id, target, config)
    }

    /// Boolean spring helper. Returns 0.0→1.0 interpolation via spring physics.
    pub fn animate_bool_spring(&mut self, id: u64, active: bool, config: SpringConfig) -> f32 {
        let target = if active { 1.0 } else { 0.0 };
        self.state.spring_t(id, target, config)
    }

    /// Whether the given spring animation is currently in-flight.
    pub fn is_spring_animating(&self, id: u64) -> bool {
        self.state.spring_active(id)
    }

    /// Play a keyframe sequence. Returns the current interpolated value.
    ///
    /// The animation starts on the first frame it's called and advances
    /// each frame. Playback mode controls looping and ping-pong behavior.
    ///
    /// ```ignore
    /// let pulse = KeyframeSequence::new(600.0)
    ///     .stop(0.0, 1.0, Easing::Linear)
    ///     .stop(0.5, 1.3, Easing::EaseOutCubic)
    ///     .stop(1.0, 1.0, Easing::EaseInCubic);
    ///
    /// let scale = ui.animate_keyframes(id!("pulse"), &pulse, PlaybackMode::Infinite);
    /// ```
    pub fn animate_keyframes(
        &mut self,
        id: u64,
        sequence: &KeyframeSequence,
        mode: PlaybackMode,
    ) -> f32 {
        self.state.keyframe_t(id, sequence, mode)
    }

    /// Whether a keyframe animation is currently playing (not finished).
    pub fn is_keyframe_animating(&self, id: u64) -> bool {
        self.state.keyframe_active(id)
    }

    // ── Focus control ──

    /// Set focus to the given widget ID.
    pub fn request_focus(&mut self, id: u64) {
        self.state.focused = Some(id);
        self.state.reset_blink();
    }

    /// Remove focus from any widget.
    pub fn clear_focus(&mut self) {
        self.state.focused = None;
    }

    /// Check if the given widget ID currently has focus.
    pub fn has_focus(&self, id: u64) -> bool {
        self.state.focused == Some(id)
    }

    /// Move focus to the next widget in the focus chain.
    pub fn focus_next(&mut self) {
        self.state.focus_next();
    }

    /// Move focus to the previous widget in the focus chain.
    pub fn focus_prev(&mut self) {
        self.state.focus_prev();
    }

    // ── Damage tracking ──

    /// Whether the UI has damage that requires a redraw.
    ///
    /// Returns `true` if any widget state changed (hover, focus, animation,
    /// scroll, overlay) since the last frame. When this returns `false`,
    /// the platform layer can skip GPU submission to save power.
    pub fn needs_redraw(&self) -> bool {
        self.state.needs_redraw()
    }

    // ── Context Menu ──

    /// Open a context menu at the current mouse position. Call when `response.right_clicked`.
    pub fn context_menu(&mut self, id: u64, items: &[&str]) {
        let mx = self.state.mouse.x;
        let my = self.state.mouse.y;

        // Measure menu width.
        let pad = self.theme.input_padding;
        let font_size = self.theme.font_size;
        let mut max_w: f32 = 0.0;
        for item in items {
            let w = self.text.measure_text(item, font_size);
            if w > max_w {
                max_w = w;
            }
        }
        let menu_w = (max_w + pad * 2.0).max(self.theme.context_menu_min_w);
        let menu_h = items.len() as f32 * self.theme.item_height;

        // Clamp to viewport.
        let mut px = mx;
        let mut py = my;
        if px + menu_w > self.region.x + self.region.w {
            px = self.region.x + self.region.w - menu_w;
        }
        if py + menu_h > self.region.y + self.region.h {
            py = self.region.y + self.region.h - menu_h;
        }

        self.state.overlay = Some(state::Overlay::ContextMenu {
            id,
            position: Rect::new(px, py, menu_w, menu_h),
            items: items.iter().map(|s| s.to_string()).collect(),
            hovered: None,
        });
    }
}
