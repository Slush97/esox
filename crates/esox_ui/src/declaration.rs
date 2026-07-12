//! Current-frame production declarations backed by [`crate::frame_core::FrameCore`].
//!
//! This is intentionally a small vertical slice. Declarations only build an
//! owned element tree; measurement, layout, paint record creation, hit testing,
//! semantics, and damage are resolved after the application closure returns.

use crate::frame_core::{
    Axis, CheckboxDeclarationError, Color, CommittedScene, Element, FrameCore, FrameError,
    GenerationAttempt, IntrinsicMeasurer, PaintPrimitive, PointerEventKind,
    ProgressDeclarationError, SceneConsumer, SemanticProperties, SemanticRole, SemanticValueRange,
    SeparatorDeclarationError, TableDeclarationError, TextProperties, VirtualListSpec,
    VirtualWindow, WidgetId, WidgetStateStore,
};
use crate::response::Response;
use esox_input::CursorIcon;
use std::collections::HashSet;

pub use crate::frame_core::{CrossAxisAlignment, GridTrack, LogicalTransform, MainAxisAlignment};

/// Direction in which a separator extends through its parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Renderer-neutral visual and layout properties for a separator leaf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeparatorStyle {
    pub layout: DeclarationStyle,
    pub orientation: SeparatorOrientation,
    pub thickness: f32,
    pub color: Color,
}

impl SeparatorStyle {
    /// Create a one-logical-unit horizontal separator in the given color.
    pub const fn new(color: Color) -> Self {
        Self {
            layout: DeclarationStyle::new(),
            orientation: SeparatorOrientation::Horizontal,
            thickness: 1.0,
            color,
        }
    }

    /// Replace layout properties for this leaf.
    pub const fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Extend vertically through a row instead of horizontally through a column.
    pub const fn vertical(mut self) -> Self {
        self.orientation = SeparatorOrientation::Vertical;
        self
    }

    /// Set the separator's authoritative cross-axis thickness.
    pub const fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

/// Renderer-neutral paint, layout, and range properties for determinate progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressStyle {
    pub layout: DeclarationStyle,
    pub minimum: f32,
    pub maximum: f32,
    pub value: f32,
    pub track_color: Color,
    pub fill_color: Color,
    pub radius: f32,
}

impl ProgressStyle {
    /// Create a unit-range progress indicator with square corners.
    pub const fn new(value: f32, track_color: Color, fill_color: Color) -> Self {
        Self {
            layout: DeclarationStyle::new(),
            minimum: 0.0,
            maximum: 1.0,
            value,
            track_color,
            fill_color,
            radius: 0.0,
        }
    }

    pub const fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Replace the determinate range and current value without clamping.
    pub const fn range(mut self, minimum: f32, maximum: f32, value: f32) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self.value = value;
        self
    }

    /// Set the exact uniform logical corner radius used for track and fill.
    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

/// Layout properties shared by the declarations in this production slice.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeclarationStyle {
    gap: f32,
    padding: f32,
    width: Option<f32>,
    height: Option<f32>,
    min_width: Option<f32>,
    min_height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    flex_grow: f32,
    flex_basis: Option<f32>,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    clip_children: bool,
    scrollable: bool,
    scroll_offset: Option<(f32, f32)>,
    absolute_position: Option<(f32, f32)>,
    blocking_overlay: bool,
    hidden: bool,
    disabled: bool,
    transform: LogicalTransform,
    background: Option<Color>,
}

impl DeclarationStyle {
    /// Create an unconstrained style.
    pub const fn new() -> Self {
        Self {
            gap: 0.0,
            padding: 0.0,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            flex_grow: 0.0,
            flex_basis: None,
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            clip_children: false,
            scrollable: false,
            scroll_offset: None,
            absolute_position: None,
            blocking_overlay: false,
            hidden: false,
            disabled: false,
            transform: LogicalTransform::new(0.0, 0.0, 1.0, 1.0),
            background: None,
        }
    }

    /// Set the logical gap between row or column children.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set uniform logical padding.
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set a fixed logical width and height.
    pub const fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set only the fixed logical width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set only the fixed logical height.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set minimum logical dimensions independently.
    pub const fn min_size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    /// Set maximum logical dimensions independently.
    pub const fn max_size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }

    /// Consume remaining space on the parent's main axis.
    pub const fn flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Set the logical flex basis used before free space is distributed.
    pub const fn flex_basis(mut self, basis: f32) -> Self {
        self.flex_basis = Some(basis);
        self
    }

    /// Align children along this row, column, or grid's main axis.
    pub const fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = alignment;
        self
    }

    /// Align children along this row, column, or grid's cross axis.
    pub const fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    /// Clip descendants to this declaration's current resolved bounds.
    pub const fn clip_children(mut self) -> Self {
        self.clip_children = true;
        self
    }

    /// Retain this viewport's last successfully applied FrameCore scroll offset.
    ///
    /// A newly introduced stable ID starts at zero. Pair this with
    /// [`Self::clip_children`] for a clipped scroll viewport.
    pub const fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Explicitly request this scroll viewport's current-frame offset.
    ///
    /// The offset affects resolved scene products, not Taffy's layout result.
    /// It takes precedence over retained FrameCore state for this generation.
    /// Use [`Self::scrollable`] instead when wheel input should drive the next
    /// declaration from the last successfully applied offset.
    pub const fn scroll_offset(mut self, x: f32, y: f32) -> Self {
        self.scrollable = true;
        self.scroll_offset = Some((x, y));
        self
    }

    /// Remove this declaration from normal flow and position it logically.
    pub const fn absolute_position(mut self, x: f32, y: f32) -> Self {
        self.absolute_position = Some((x, y));
        self
    }

    /// Make this declaration a semantic, focus-scoped blocking overlay.
    ///
    /// FrameCore retains ownership of blocking, focus trapping and restoration,
    /// and capture cancellation when this declaration closes or stops
    /// participating in interaction.
    pub const fn blocking_overlay(mut self) -> Self {
        self.blocking_overlay = true;
        self
    }

    /// Paint a solid background behind this container's children.
    ///
    /// Containers without a background stay excluded from paint, so they never
    /// reach the renderer boundary as unsupported `Box` primitives.
    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Collapse this declaration and its descendants out of layout and scene products.
    pub const fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Set whether this declaration is collapsed out of layout and scene products.
    pub const fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Keep this declaration painted and laid out while disabling descendant interaction.
    pub const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Set whether this declaration stays painted and laid out but is non-interactive.
    pub const fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Apply a renderer-neutral transform after current-generation layout.
    ///
    /// The transform affects this declaration and its descendants. Scaling is
    /// relative to this declaration's resolved logical center.
    pub const fn transform(mut self, transform: LogicalTransform) -> Self {
        self.transform = transform;
        self
    }

    fn apply(self, mut element: Element) -> Element {
        element = element
            .with_padding(self.padding)
            .with_size(self.width, self.height)
            .with_min_size(self.min_width, self.min_height)
            .with_max_size(self.max_width, self.max_height)
            .with_flex_grow(self.flex_grow)
            .with_main_axis_alignment(self.main_axis_alignment)
            .with_cross_axis_alignment(self.cross_axis_alignment)
            .with_hidden(self.hidden)
            .with_disabled(self.disabled)
            .with_transform(self.transform);
        if let Some(basis) = self.flex_basis {
            element = element.with_flex_basis(basis);
        }
        if let Some((x, y)) = self.scroll_offset {
            element = element.with_scroll_offset(x, y);
        } else if self.scrollable {
            element = element.scrollable();
        }
        if self.clip_children {
            element = element.clip_children();
        }
        if let Some((x, y)) = self.absolute_position {
            element = element.with_absolute_position(x, y);
        }
        if self.blocking_overlay {
            element = element.blocking_overlay();
        }
        element
    }
}

/// Grid-specific properties paired with the shared declaration layout style.
#[derive(Clone, Debug, PartialEq)]
pub struct GridStyle {
    pub layout: DeclarationStyle,
    pub columns: Vec<GridTrack>,
    pub column_gap: f32,
    pub row_gap: f32,
}

impl GridStyle {
    /// Create a grid with explicit fixed, fractional, or auto columns.
    ///
    /// Tracks are preserved exactly so invalid declarations produce FrameCore's
    /// typed frame error instead of being silently normalized by the facade.
    pub fn new(columns: impl Into<Vec<GridTrack>>) -> Self {
        Self {
            layout: DeclarationStyle::new(),
            columns: columns.into(),
            column_gap: 0.0,
            row_gap: 0.0,
        }
    }

    /// Replace the shared container layout properties.
    pub const fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Set the independent horizontal gap between grid columns.
    pub const fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = gap;
        self
    }

    /// Set the independent vertical gap between implicit grid rows.
    pub const fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = gap;
        self
    }

    /// Set horizontal and vertical gaps together.
    pub const fn gaps(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }
}

/// Renderer-neutral text declaration properties.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub layout: DeclarationStyle,
    pub properties: TextProperties,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            layout: DeclarationStyle::new(),
            properties: TextProperties::default(),
            color: Color::BLACK,
        }
    }
}

impl TextStyle {
    /// Replace layout properties for this text leaf.
    pub fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Replace shaping and measurement properties for this text leaf.
    pub fn properties(mut self, properties: TextProperties) -> Self {
        self.properties = properties;
        self
    }

    /// Set the backend-neutral text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Renderer-neutral properties for an intrinsically measured image leaf.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageStyle {
    pub layout: DeclarationStyle,
    pub label: Option<String>,
    pub disabled: bool,
}

impl ImageStyle {
    /// Replace layout properties for this image leaf.
    pub const fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Set the accessible label without coupling the declaration to a backend.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Keep the image visible while excluding it from interaction.
    pub const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// Visual and layout properties for the basic production button.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonStyle {
    pub layout: DeclarationStyle,
    pub fill: Color,
    pub border: Color,
    pub border_width: f32,
    pub text: TextStyle,
    pub disabled: bool,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            layout: DeclarationStyle::new().size(96.0, 36.0),
            fill: Color::rgba(0.18, 0.38, 0.72, 1.0),
            border: Color::rgba(0.10, 0.22, 0.45, 1.0),
            border_width: 1.0,
            text: TextStyle::default().color(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            disabled: false,
        }
    }
}

impl ButtonStyle {
    pub fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    pub fn fill(mut self, fill: Color) -> Self {
        self.fill = fill;
        self
    }

    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.border = color;
        self.border_width = width;
        self
    }

    pub fn text(mut self, text: TextStyle) -> Self {
        self.text = text;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// Stable scene identities reserved by a button declaration.
///
/// Callers should not reuse `fill` or `label` for sibling declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonIds {
    pub root: WidgetId,
    pub fill: WidgetId,
    pub label: WidgetId,
}

/// Renderer-neutral layout and paint properties for a production checkbox.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckboxStyle {
    pub layout: DeclarationStyle,
    pub indicator_size: f32,
    pub gap: f32,
    pub radius: f32,
    pub unchecked_fill: Color,
    pub checked_fill: Color,
    pub mark: TextStyle,
    pub label: TextStyle,
    pub disabled: bool,
}

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self {
            layout: DeclarationStyle::new().size(160.0, 36.0),
            indicator_size: 18.0,
            gap: 8.0,
            radius: 4.0,
            unchecked_fill: Color::rgba(0.16, 0.17, 0.20, 1.0),
            checked_fill: Color::rgba(0.18, 0.38, 0.72, 1.0),
            mark: TextStyle::default().color(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            label: TextStyle::default().color(Color::rgba(0.08, 0.08, 0.09, 1.0)),
            disabled: false,
        }
    }
}

impl CheckboxStyle {
    pub fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    pub const fn indicator_size(mut self, indicator_size: f32) -> Self {
        self.indicator_size = indicator_size;
        self
    }

    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub const fn fills(mut self, unchecked: Color, checked: Color) -> Self {
        self.unchecked_fill = unchecked;
        self.checked_fill = checked;
        self
    }

    pub fn mark(mut self, mark: TextStyle) -> Self {
        self.mark = mark;
        self
    }

    pub fn label(mut self, label: TextStyle) -> Self {
        self.label = label;
        self
    }

    pub const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// Stable scene identities reserved by a checkbox declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckboxIds {
    pub root: WidgetId,
    pub indicator: WidgetId,
    pub mark: WidgetId,
    pub label: WidgetId,
}

/// Visual and layout properties for a production split pane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitPaneStyle {
    pub layout: DeclarationStyle,
    pub divider_width: f32,
    pub panel_padding: f32,
    pub divider_color: Color,
}

impl Default for SplitPaneStyle {
    fn default() -> Self {
        Self {
            layout: DeclarationStyle::new(),
            divider_width: 5.0,
            panel_padding: 0.0,
            divider_color: Color::rgba(0.3, 0.3, 0.32, 1.0),
        }
    }
}

impl SplitPaneStyle {
    pub const fn layout(mut self, layout: DeclarationStyle) -> Self {
        self.layout = layout;
        self
    }

    pub const fn divider(mut self, width: f32, color: Color) -> Self {
        self.divider_width = width;
        self.divider_color = color;
        self
    }

    pub const fn panel_padding(mut self, padding: f32) -> Self {
        self.panel_padding = padding;
        self
    }
}

/// Stable scene identities reserved by a split-pane declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitPaneIds {
    pub root: WidgetId,
    pub first: WidgetId,
    pub divider: WidgetId,
    pub second: WidgetId,
}

/// One fixed-width column in a production [`DeclarationUi::table`].
///
/// Widths are intentionally fixed in this first slice. Intrinsic `Auto` tracks
/// cannot align a separately virtualized body with the header without
/// measuring offscreen rows, which this API never does.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    pub id: WidgetId,
    pub label: String,
    pub width: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub sortable: bool,
    pub resizable: bool,
}

impl TableColumn {
    pub fn new(id: WidgetId, label: impl Into<String>, width: f32) -> Self {
        Self {
            id,
            label: label.into(),
            width,
            min_width: 0.0,
            max_width: f32::INFINITY,
            sortable: false,
            resizable: false,
        }
    }

    pub const fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub const fn resizable(mut self, min_width: f32, max_width: f32) -> Self {
        self.resizable = true;
        self.min_width = min_width;
        self.max_width = max_width;
        self
    }
}

/// Current-generation inputs for a virtual production table.
#[derive(Clone, Debug, PartialEq)]
pub struct TableSpec {
    pub id: WidgetId,
    pub row_count: usize,
    pub row_height: f32,
    pub viewport_height: f32,
    pub columns: Vec<TableColumn>,
    pub explicit_offset: Option<f32>,
    pub scroll_to: Option<usize>,
    pub selectable: bool,
    pub selected_row_index: Option<usize>,
}

impl TableSpec {
    pub fn new(
        id: WidgetId,
        row_count: usize,
        row_height: f32,
        viewport_height: f32,
        columns: impl Into<Vec<TableColumn>>,
    ) -> Self {
        Self {
            id,
            row_count,
            row_height,
            viewport_height,
            columns: columns.into(),
            explicit_offset: None,
            scroll_to: None,
            selectable: false,
            selected_row_index: None,
        }
    }

    pub const fn with_offset(mut self, offset: f32) -> Self {
        self.explicit_offset = Some(offset);
        self
    }

    pub const fn scroll_to(mut self, row: usize) -> Self {
        self.scroll_to = Some(row);
        self
    }

    pub const fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    /// Supply the caller-owned logical selection used as the keyboard anchor.
    ///
    /// A stale index after a data-set shrink is treated as no selection.
    pub const fn selected_row(mut self, index: usize) -> Self {
        self.selected_row_index = Some(index);
        self
    }
}

/// Renderer-neutral visual properties for a production table declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct TableStyle {
    pub layout: DeclarationStyle,
    pub header_height: f32,
    pub header_background: Color,
    pub resize_handle_width: f32,
    pub resize_handle_color: Color,
    pub header_text: TextStyle,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            layout: DeclarationStyle::new(),
            header_height: 32.0,
            header_background: Color::rgba(0.12, 0.12, 0.14, 1.0),
            resize_handle_width: 5.0,
            resize_handle_color: Color::rgba(0.35, 0.35, 0.38, 1.0),
            header_text: TextStyle::default(),
        }
    }
}

/// Stable part identities reserved by a production table.
///
/// The table owns `root`, `header`, `body`, every [`Self::header_cell`],
/// [`Self::header_label`], [`Self::resize_handle`], and
/// [`Self::virtual_wrapper`] ID. Row callbacks must not reuse any of them.
/// Application row IDs and cell descendants occupy a separate namespace and
/// are checked atomically with the reserved parts during resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableIds {
    pub root: WidgetId,
    pub header: WidgetId,
    pub body: WidgetId,
}

impl TableIds {
    pub const fn new(root: WidgetId) -> Self {
        Self {
            root,
            header: derived_id(root, 0x78c3_943a_f0b3_2731),
            body: derived_id(root, 0xb529_d4a2_4013_aa7d),
        }
    }

    pub const fn header_cell(self, column: WidgetId) -> WidgetId {
        table_header_cell_id(self.root, column)
    }

    pub const fn header_label(self, column: WidgetId) -> WidgetId {
        table_header_label_id(self.root, column)
    }

    pub const fn resize_handle(self, column: WidgetId) -> WidgetId {
        table_resize_handle_id(self.root, column)
    }

    pub const fn virtual_wrapper(self, logical_row_index: usize) -> WidgetId {
        virtual_item_id(self.body, logical_row_index)
    }
}

/// Stable ID for a header cell derived from the logical column identity.
pub const fn table_header_cell_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table, 0xdea6_23af_77c1_a4b9 ^ column.0.rotate_left(11))
}

/// Stable ID helper for an application-declared logical cell.
pub const fn table_cell_id(row: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(row, 0xa173_b9ce_4f03_88d1 ^ column.0.rotate_left(29))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableResize {
    pub column: WidgetId,
    pub width: f32,
}

/// Intents consumed from the last committed table scene.
///
/// Sorting and selected application data remain caller-owned. A failed frame
/// replays these intents because response consumption is candidate state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableResponse {
    /// Last row whose press/release click completed in ledger order this frame.
    pub selected_row: Option<WidgetId>,
    /// Sort clicks in committed input-ledger order; data reordering is external.
    pub sort_requested: Vec<WidgetId>,
    pub resized: Vec<TableResize>,
}

struct TableColumnRuntime {
    width: f32,
    dragging: bool,
    pointer: u64,
    origin: f32,
    start_width: f32,
}

fn response_inside_target(input: &crate::frame_core::InputResponse) -> bool {
    input.position.x >= input.target_bounds.x
        && input.position.y >= input.target_bounds.y
        && input.position.x < input.target_bounds.x + input.target_bounds.width
        && input.position.y < input.target_bounds.y + input.target_bounds.height
}

impl SplitPaneIds {
    pub const fn new(root: WidgetId) -> Self {
        Self {
            root,
            first: derived_id(root, 0x5762_0fd5_c39c_35b1),
            divider: derived_id(root, 0xd1aa_d911_9387_3f23),
            second: derived_id(root, 0x7119_4c22_852d_41ab),
        }
    }
}

impl ButtonIds {
    pub const fn new(root: WidgetId) -> Self {
        Self {
            root,
            fill: derived_id(root, 0x8ab4_931c_46ad_83f9),
            label: derived_id(root, 0xe34a_5d71_928c_f607),
        }
    }
}

impl CheckboxIds {
    pub const fn new(root: WidgetId) -> Self {
        Self {
            root,
            indicator: derived_id(root, 0x3ad5_f946_7b24_c118),
            mark: derived_id(root, 0x91e8_32a7_d41f_b065),
            label: derived_id(root, 0xe6c2_7109_5ad8_3f4b),
        }
    }
}

const fn derived_id(parent: WidgetId, salt: u64) -> WidgetId {
    WidgetId(parent.0.rotate_left(23) ^ salt)
}

/// Stable FrameCore-owned identity for a progress indicator's fill node.
pub const fn progress_fill_id(progress: WidgetId) -> WidgetId {
    derived_id(progress, 0xa646_5928_7169_8105)
}

/// Stable FrameCore-owned identity for a progress indicator's unpainted remainder.
pub const fn progress_remainder_id(progress: WidgetId) -> WidgetId {
    derived_id(progress, 0x7658_021d_8d3d_91df)
}

fn interaction_state_id(id: WidgetId) -> WidgetId {
    derived_id(id, 0x41e8_7305_bfd9_2c6b)
}

fn split_ratio_state_id(id: WidgetId) -> WidgetId {
    derived_id(id, 0xc781_40c4_d551_c8f3)
}

fn split_drag_state_id(id: WidgetId) -> WidgetId {
    derived_id(id, 0x48bb_99e5_2879_b288)
}

fn split_pointer_state_id(id: WidgetId) -> WidgetId {
    derived_id(id, 0x4ff4_b7e3_6f88_3690)
}

fn table_width_state_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0xc187_b4fb_c84a_17d3)
}

fn table_drag_state_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0x9be2_9b3a_a191_4267)
}

fn table_pointer_state_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0x3f74_a6d8_28ea_f263)
}

fn table_drag_origin_state_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0xac79_4551_0ddd_a747)
}

fn table_drag_width_state_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0x6974_26db_4f89_5219)
}

fn table_base_width_state_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0xa27d_2ff8_931c_ba17)
}

fn table_active_press_state_id(table: WidgetId, pointer: u64) -> WidgetId {
    derived_id(table, 0xf2ab_86c4_915d_d31b ^ pointer.rotate_left(19))
}

fn table_active_press_target_state_id(table: WidgetId, pointer: u64) -> WidgetId {
    derived_id(table, 0x5f04_731c_e7af_d2d9 ^ pointer.rotate_left(31))
}

pub const fn table_header_label_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0x56e5_f39e_116a_9d89)
}

pub const fn table_resize_handle_id(table: WidgetId, column: WidgetId) -> WidgetId {
    derived_id(table_header_cell_id(table, column), 0xd4b4_6c8e_b7e9_313f)
}

/// Stable wrapper identity reserved for one logical item in a virtual viewport.
///
/// Application declarations inside that item must not reuse this ID. Any
/// collision is rejected atomically by FrameCore's duplicate-ID validation.
pub const fn virtual_item_id(viewport: WidgetId, item: usize) -> WidgetId {
    let item = item as u64;
    let mixed = item.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17);
    derived_id(viewport, 0x2349_762f_b42a_1d85 ^ mixed)
}

/// One once-only declaration context for the tree currently under construction.
enum DeclarationState<'a> {
    Widget(&'a mut WidgetStateStore),
    Attempt(&'a mut GenerationAttempt),
}

pub struct DeclarationUi<'a> {
    state: DeclarationState<'a>,
    child_stacks: Vec<Vec<Element>>,
    effective_hidden: bool,
    effective_disabled: bool,
    error: Option<FrameError>,
}

impl<'a> DeclarationUi<'a> {
    /// Construct a declaration context without candidate virtual-scroll access.
    pub fn new(state: &'a mut WidgetStateStore) -> Self {
        Self::with_state(DeclarationState::Widget(state))
    }

    fn from_attempt(attempt: &'a mut GenerationAttempt) -> Self {
        Self::with_state(DeclarationState::Attempt(attempt))
    }

    fn with_state(state: DeclarationState<'a>) -> Self {
        Self {
            state,
            child_stacks: vec![Vec::new()],
            effective_hidden: false,
            effective_disabled: false,
            error: None,
        }
    }

    fn state(&mut self) -> &mut WidgetStateStore {
        match &mut self.state {
            DeclarationState::Widget(state) => state,
            DeclarationState::Attempt(attempt) => attempt.widget_state(),
        }
    }

    /// Finish a declaration containing exactly one root element.
    pub fn finish(self) -> Element {
        self.try_finish()
            .expect("a directly constructed declaration must be valid")
    }

    fn try_finish(mut self) -> Result<Element, FrameError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        let roots = self
            .child_stacks
            .pop()
            .expect("the declaration root stack always exists");
        assert!(
            self.child_stacks.is_empty(),
            "unclosed declaration container"
        );
        let mut roots = roots.into_iter();
        let root = roots
            .next()
            .expect("a declaration requires one root element");
        assert!(
            roots.next().is_none(),
            "a declaration requires exactly one root element"
        );
        Ok(root)
    }

    /// Request keyboard focus after this declaration resolves successfully.
    pub fn request_keyboard_focus(&mut self, id: WidgetId) {
        self.state().request_keyboard_focus(id);
    }

    /// Request pointer capture after this declaration resolves successfully.
    pub fn request_pointer_capture(&mut self, pointer: u64, id: WidgetId) {
        self.state().request_pointer_capture(pointer, id);
    }

    /// Request release of an existing pointer capture on successful commit.
    pub fn request_pointer_release(&mut self, pointer: u64) {
        self.state().request_pointer_release(pointer);
    }

    /// Declare a vertical container and execute its body exactly once.
    pub fn column(&mut self, id: WidgetId, style: DeclarationStyle, body: impl FnOnce(&mut Self)) {
        self.container(id, Axis::Column, style, body);
    }

    /// Declare a horizontal container and execute its body exactly once.
    pub fn row(&mut self, id: WidgetId, style: DeclarationStyle, body: impl FnOnce(&mut Self)) {
        self.container(id, Axis::Row, style, body);
    }

    /// Declare only the current visible range of a uniform-height vertical list.
    ///
    /// The viewport's full logical content height participates in resolution
    /// even though the callback runs exactly once only for visible items.
    /// `viewport_height` is authoritative: vertical padding, flex sizing, a
    /// conflicting height/min/max, and style-owned scroll offsets are rejected
    /// before item callbacks.
    pub fn virtual_column(
        &mut self,
        spec: VirtualListSpec,
        mut style: DeclarationStyle,
        mut item: impl FnMut(&mut Self, usize),
    ) -> Result<VirtualWindow, FrameError> {
        let conflicts = style.padding != 0.0
            || style.flex_grow != 0.0
            || style.flex_basis.is_some()
            || style.scroll_offset.is_some()
            || style
                .height
                .is_some_and(|height| height != spec.viewport_height)
            || style
                .min_height
                .is_some_and(|height| height != spec.viewport_height)
            || style
                .max_height
                .is_some_and(|height| height != spec.viewport_height);
        if conflicts {
            let error = FrameError::InvalidVirtualContent {
                id: spec.id,
                error: crate::frame_core::VirtualDeclarationError::ConflictingViewportStyle,
            };
            self.error = Some(error.clone());
            return Err(error);
        }
        let planned = match &mut self.state {
            DeclarationState::Attempt(attempt) => attempt.plan_virtual_list(spec),
            DeclarationState::Widget(_) => Err(FrameError::InvalidVirtualContent {
                id: spec.id,
                error: crate::frame_core::VirtualDeclarationError::CandidateStateUnavailable,
            }),
        };
        let window = match planned {
            Ok(window) => window,
            Err(error) => {
                self.error = Some(error.clone());
                return Err(error);
            }
        };

        let previous_participation = self.enter_container_participation(style);
        let mut wrappers = Vec::with_capacity(window.visible_range.len());
        for index in window.visible_range.clone() {
            let children = self.capture_children(|ui| item(ui, index));
            wrappers.push(
                Element::flex(virtual_item_id(spec.id, index), Axis::Column, 0.0)
                    .without_paint()
                    .with_size(None, Some(spec.item_height))
                    .with_min_size(Some(0.0), Some(spec.item_height))
                    .with_absolute_position(
                        0.0,
                        (index as f64 * f64::from(spec.item_height)) as f32,
                    )
                    .with_children(children),
            );
        }
        self.restore_container_participation(previous_participation);

        style.height = Some(spec.viewport_height);
        style.min_height = Some(spec.viewport_height);
        style.max_height = Some(spec.viewport_height);
        style.clip_children = true;
        style.scrollable = true;
        style.scroll_offset = Some((0.0, window.requested_offset));
        let viewport = Element::flex(spec.id, Axis::Column, 0.0)
            .without_paint()
            .with_semantics(SemanticProperties::new(SemanticRole::ScrollView))
            .with_children(wrappers)
            .with_flex_shrink(0.0)
            .with_virtual_content_height(window.content_height);
        self.push(style.apply(viewport));
        Ok(window)
    }

    /// Declare a fixed-track header and one virtualized body exactly once.
    ///
    /// `row` runs once for each visible logical index, in ascending row order,
    /// and returns that row's stable application identity. It must declare
    /// exactly one direct child per column in column order. The callback is
    /// never retained and is never invoked for measurement or an offscreen
    /// row. Cell IDs should be derived from logical row and column identities
    /// with [`table_cell_id`], never from the visible slot. See [`TableIds`]
    /// for the complete reserved identity namespace.
    pub fn table(
        &mut self,
        spec: TableSpec,
        style: TableStyle,
        mut row: impl FnMut(&mut Self, usize) -> WidgetId,
    ) -> Result<(VirtualWindow, TableResponse), FrameError> {
        self.validate_table(&spec, &style)?;

        let ids = TableIds::new(spec.id);
        let suppressed = self.effective_hidden
            || self.effective_disabled
            || style.layout.hidden
            || style.layout.disabled;
        let mut response = TableResponse::default();
        let mut runtime = Vec::with_capacity(spec.columns.len());
        for column in &spec.columns {
            let width_state = table_width_state_id(spec.id, column.id);
            let base_state = table_base_width_state_id(spec.id, column.id);
            let previous_base = self
                .state()
                .get(base_state)
                .map(|bits| f32::from_bits(bits as u32));
            let width = if column.resizable {
                if previous_base == Some(column.width) {
                    self.state()
                        .get(width_state)
                        .map(|bits| f32::from_bits(bits as u32))
                        .filter(|width| width.is_finite())
                        .unwrap_or(column.width)
                } else {
                    column.width
                }
                .clamp(column.min_width, column.max_width)
            } else {
                column.width
            };
            self.state()
                .insert(base_state, u64::from(column.width.to_bits()));
            let mut dragging = column.resizable
                && self
                    .state()
                    .get(table_drag_state_id(spec.id, column.id))
                    .is_some_and(|value| value != 0);
            let pointer = self
                .state()
                .get(table_pointer_state_id(spec.id, column.id))
                .unwrap_or_default();
            if dragging
                && self.state().pointer_capture_owner(pointer)
                    != Some(table_resize_handle_id(spec.id, column.id))
            {
                dragging = false;
            }
            runtime.push(TableColumnRuntime {
                width,
                dragging,
                pointer,
                origin: self
                    .state()
                    .get(table_drag_origin_state_id(spec.id, column.id))
                    .map(|bits| f32::from_bits(bits as u32))
                    .unwrap_or_default(),
                start_width: self
                    .state()
                    .get(table_drag_width_state_id(spec.id, column.id))
                    .map(|bits| f32::from_bits(bits as u32))
                    .unwrap_or(width),
            });
        }

        let header_targets = spec
            .columns
            .iter()
            .flat_map(|column| {
                [
                    table_header_cell_id(spec.id, column.id),
                    table_resize_handle_id(spec.id, column.id),
                ]
            })
            .collect::<HashSet<_>>();
        let inputs = self.state().drain_responses(|input| {
            header_targets.contains(&input.target)
                || (input.virtual_owner == Some(ids.body)
                    && input.target == input.interaction_owner.unwrap_or(WidgetId(u64::MAX)))
        });
        let keyboard_inputs = self
            .state()
            .drain_keyboard_responses(|input| input.target == ids.body);
        let mut pressed_this_batch = HashSet::new();
        for input in inputs {
            let sortable_column = spec.columns.iter().find(|column| {
                column.sortable && table_header_cell_id(spec.id, column.id) == input.target
            });
            let selectable_row = spec.selectable && input.interaction_owner == Some(input.target);
            let active_state = table_active_press_state_id(spec.id, input.pointer);
            let active_target_state = table_active_press_target_state_id(spec.id, input.pointer);

            match input.kind {
                PointerEventKind::Press => {
                    // Every new table gesture supersedes stale click state for
                    // this table/pointer, including a press on a resize handle.
                    self.state().insert(active_state, 0);
                    if !suppressed && (sortable_column.is_some() || selectable_row) {
                        self.state().insert(active_state, 1);
                        self.state().insert(active_target_state, input.target.0);
                        self.state()
                            .request_pointer_capture(input.pointer, input.target);
                        if selectable_row {
                            self.state().request_keyboard_focus(ids.body);
                        }
                        pressed_this_batch.insert(input.pointer);
                    }
                }
                PointerEventKind::Release | PointerEventKind::Cancel => {
                    let active = self.state().get(active_state) == Some(1)
                        && self.state().get(active_target_state) == Some(input.target.0);
                    let capture_valid = pressed_this_batch.contains(&input.pointer)
                        || self.state().pointer_capture_owner(input.pointer) == Some(input.target);
                    if input.kind == PointerEventKind::Release
                        && !suppressed
                        && active
                        && capture_valid
                        && response_inside_target(&input)
                    {
                        if let Some(column) = sortable_column {
                            response.sort_requested.push(column.id);
                        } else if selectable_row {
                            // Multiple completed row clicks are processed in ledger order;
                            // the last completed click wins this scalar selection intent.
                            response.selected_row = Some(input.target);
                        }
                    }
                    // All terminal table events clear prior click state even
                    // when sorting/selection was disabled since the press.
                    self.state().insert(active_state, 0);
                    self.state().request_pointer_release(input.pointer);
                    pressed_this_batch.remove(&input.pointer);
                }
                PointerEventKind::Move => {}
            }

            if let Some(index) = spec
                .columns
                .iter()
                .position(|column| table_resize_handle_id(spec.id, column.id) == input.target)
            {
                if suppressed {
                    continue;
                }
                let column = &spec.columns[index];
                let state = &mut runtime[index];
                match input.kind {
                    PointerEventKind::Press if !state.dragging => {
                        state.dragging = true;
                        state.pointer = input.pointer;
                        state.origin = input.position.x;
                        state.start_width = state.width;
                        self.state()
                            .request_pointer_capture(input.pointer, input.target);
                    }
                    PointerEventKind::Move if state.dragging && input.pointer == state.pointer => {
                        let requested = state.start_width + input.position.x - state.origin;
                        if requested.is_finite() {
                            state.width = requested.clamp(column.min_width, column.max_width);
                            response.resized.push(TableResize {
                                column: column.id,
                                width: state.width,
                            });
                        }
                    }
                    PointerEventKind::Release | PointerEventKind::Cancel
                        if input.pointer == state.pointer =>
                    {
                        state.dragging = false;
                        self.state().request_pointer_release(input.pointer);
                    }
                    PointerEventKind::Press
                    | PointerEventKind::Move
                    | PointerEventKind::Release
                    | PointerEventKind::Cancel => {}
                }
                continue;
            }
        }

        let mut keyboard_selection = spec
            .selected_row_index
            .filter(|index| *index < spec.row_count);
        let mut keyboard_activated = false;
        if spec.selectable && !suppressed && spec.row_count > 0 {
            let last = spec.row_count - 1;
            let page = ((spec.viewport_height / spec.row_height).floor() as usize).max(1);
            for input in keyboard_inputs {
                if !input.event.pressed {
                    continue;
                }
                let requested = match input.event.key {
                    esox_input::Key::Named(esox_input::NamedKey::ArrowDown) => Some(
                        keyboard_selection.map_or(0, |index| index.saturating_add(1).min(last)),
                    ),
                    esox_input::Key::Named(esox_input::NamedKey::ArrowUp) => {
                        Some(keyboard_selection.map_or(last, |index| index.saturating_sub(1)))
                    }
                    esox_input::Key::Named(esox_input::NamedKey::Home) => Some(0),
                    esox_input::Key::Named(esox_input::NamedKey::End) => Some(last),
                    esox_input::Key::Named(esox_input::NamedKey::PageDown) => Some(
                        keyboard_selection.map_or(0, |index| index.saturating_add(page).min(last)),
                    ),
                    esox_input::Key::Named(esox_input::NamedKey::PageUp) => {
                        Some(keyboard_selection.map_or(last, |index| index.saturating_sub(page)))
                    }
                    esox_input::Key::Named(
                        esox_input::NamedKey::Enter | esox_input::NamedKey::Space,
                    ) => keyboard_selection,
                    _ => None,
                };
                if let Some(index) = requested {
                    keyboard_selection = Some(index);
                    keyboard_activated = true;
                }
            }
        }

        for (column, state) in spec.columns.iter().zip(&mut runtime) {
            if suppressed && state.dragging {
                self.state().request_pointer_release(state.pointer);
                state.dragging = false;
            }
            self.state().insert(
                table_width_state_id(spec.id, column.id),
                u64::from(state.width.to_bits()),
            );
            self.state().insert(
                table_drag_state_id(spec.id, column.id),
                u64::from(state.dragging),
            );
            self.state()
                .insert(table_pointer_state_id(spec.id, column.id), state.pointer);
            self.state().insert(
                table_drag_origin_state_id(spec.id, column.id),
                u64::from(state.origin.to_bits()),
            );
            self.state().insert(
                table_drag_width_state_id(spec.id, column.id),
                u64::from(state.start_width.to_bits()),
            );
        }

        let widths = runtime.iter().map(|state| state.width).collect::<Vec<_>>();
        let total_width_f64: f64 = widths.iter().map(|width| f64::from(*width)).sum();
        if !total_width_f64.is_finite() || total_width_f64 > f64::from(f32::MAX) {
            let error = FrameError::InvalidTable {
                id: spec.id,
                error: TableDeclarationError::UnrepresentableAggregateWidth,
            };
            self.error = Some(error.clone());
            return Err(error);
        }
        let total_width = total_width_f64 as f32;

        let tracks = widths
            .iter()
            .copied()
            .map(GridTrack::Fixed)
            .collect::<Vec<_>>();
        let header_children = spec
            .columns
            .iter()
            .zip(widths.iter().copied())
            .map(|(column, width)| {
                let label = Element::text_with_properties(
                    table_header_label_id(spec.id, column.id),
                    column.label.clone(),
                    style.header_text.properties.clone(),
                )
                .with_paint(PaintPrimitive::Text {
                    content: column.label.clone(),
                    properties: style.header_text.properties.clone(),
                    color: style.header_text.color,
                });
                let label = style.header_text.layout.flex_grow(1.0).apply(label);
                let mut children = vec![label];
                if column.resizable {
                    // A shrunk track owns its whole handle, but the handle never
                    // escapes the resolved header-cell bounds.
                    let handle_width = style.resize_handle_width.min(width);
                    children.push(
                        Element::flex(
                            table_resize_handle_id(spec.id, column.id),
                            Axis::Column,
                            0.0,
                        )
                        .with_size(Some(handle_width), None)
                        .with_min_size(Some(handle_width), Some(0.0))
                        .with_paint(PaintPrimitive::SolidRect {
                            color: style.resize_handle_color,
                        })
                        .with_cursor_icon(CursorIcon::ColResize)
                        .ordered_pointer_target()
                        .interactive(),
                    );
                }
                let mut cell =
                    Element::flex(table_header_cell_id(spec.id, column.id), Axis::Row, 0.0)
                        .with_size(Some(width), Some(style.header_height))
                        .with_children(children);
                if column.sortable {
                    cell = cell.ordered_pointer_target().interactive();
                }
                cell
            })
            .collect();
        let header = Element::grid(ids.header, tracks.clone(), 0.0)
            .with_size(Some(total_width), Some(style.header_height))
            .with_min_size(Some(total_width), Some(style.header_height))
            .with_paint(PaintPrimitive::SolidRect {
                color: style.header_background,
            })
            .with_children(header_children);

        let mut virtual_spec = VirtualListSpec::new(
            ids.body,
            spec.row_count,
            spec.row_height,
            spec.viewport_height,
        );
        virtual_spec.explicit_offset = spec.explicit_offset;
        virtual_spec.scroll_to = spec.scroll_to;
        if keyboard_activated {
            virtual_spec.explicit_offset = None;
            virtual_spec.scroll_to = keyboard_selection;
        }

        let previous_participation = self.enter_container_participation(style.layout);
        let body_result = self.virtual_column(
            virtual_spec,
            DeclarationStyle::new().width(total_width),
            |ui, index| {
                if ui.error.is_some() {
                    return;
                }
                let (row_id, children) = ui.capture_children_result(|ui| row(ui, index));
                if keyboard_activated && keyboard_selection == Some(index) {
                    response.selected_row = Some(row_id);
                }
                if children.len() != spec.columns.len() {
                    ui.error = Some(FrameError::InvalidTable {
                        id: spec.id,
                        error: TableDeclarationError::InvalidRowCellCount {
                            row: index,
                            expected: spec.columns.len(),
                            actual: children.len(),
                        },
                    });
                    return;
                }
                let mut row_element = Element::grid(row_id, tracks.clone(), 0.0)
                    .with_size(Some(total_width), Some(spec.row_height))
                    .with_min_size(Some(total_width), Some(spec.row_height))
                    .with_children(children);
                if spec.selectable {
                    row_element = row_element
                        .with_interaction_owner(row_id)
                        .ordered_pointer_target()
                        .interactive();
                }
                ui.push(row_element);
            },
        );
        self.restore_container_participation(previous_participation);
        let window = body_result?;

        let roots = self
            .child_stacks
            .last_mut()
            .expect("a declaration child stack always exists");
        let body = roots
            .pop()
            .expect("virtual_column pushed the table body viewport")
            .interactive();
        let root = Element::flex(ids.root, Axis::Column, 0.0)
            .without_paint()
            .with_children(vec![header, body]);
        self.push(style.layout.apply(root));
        Ok((window, response))
    }

    fn validate_table(&mut self, spec: &TableSpec, style: &TableStyle) -> Result<(), FrameError> {
        let invalid = if spec.columns.is_empty() {
            Some(TableDeclarationError::EmptyColumns)
        } else if !style.header_height.is_finite() || style.header_height <= 0.0 {
            Some(TableDeclarationError::InvalidHeaderHeight(
                style.header_height,
            ))
        } else if !style.resize_handle_width.is_finite() || style.resize_handle_width <= 0.0 {
            Some(TableDeclarationError::InvalidResizeHandleWidth(
                style.resize_handle_width,
            ))
        } else {
            let mut ids = HashSet::with_capacity(spec.columns.len());
            spec.columns.iter().find_map(|column| {
                if !ids.insert(column.id) {
                    Some(TableDeclarationError::DuplicateColumnId(column.id))
                } else if !column.width.is_finite() || column.width <= 0.0 {
                    Some(TableDeclarationError::InvalidColumnWidth {
                        column: column.id,
                        value: column.width,
                    })
                } else if !column.min_width.is_finite() || column.min_width < 0.0 {
                    Some(TableDeclarationError::InvalidColumnMinimum {
                        column: column.id,
                        value: column.min_width,
                    })
                } else if column.max_width.is_nan() || column.max_width <= 0.0 {
                    Some(TableDeclarationError::InvalidColumnMaximum {
                        column: column.id,
                        value: column.max_width,
                    })
                } else if column.min_width > column.max_width {
                    Some(TableDeclarationError::InvalidColumnRange {
                        column: column.id,
                        min: column.min_width,
                        max: column.max_width,
                    })
                } else {
                    None
                }
            })
        };
        if let Some(error) = invalid {
            let error = FrameError::InvalidTable { id: spec.id, error };
            self.error = Some(error.clone());
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Declare a grid container and execute its body exactly once.
    ///
    /// Children are placed in deterministic row-major order. Additional rows
    /// are created by FrameCore through its shared Taffy layout path.
    pub fn grid(&mut self, id: WidgetId, style: GridStyle, body: impl FnOnce(&mut Self)) {
        self.child_stacks.push(Vec::new());
        let previous_participation = self.enter_container_participation(style.layout);
        body(self);
        self.restore_container_participation(previous_participation);
        let children = self
            .child_stacks
            .pop()
            .expect("the grid container stack was just pushed");
        let element = Element::grid_with_gaps(id, style.columns, style.column_gap, style.row_gap)
            .with_children(children);
        self.push(
            style
                .layout
                .apply(Self::paint_container(element, style.layout.background)),
        );
    }

    fn container(
        &mut self,
        id: WidgetId,
        axis: Axis,
        style: DeclarationStyle,
        body: impl FnOnce(&mut Self),
    ) {
        self.child_stacks.push(Vec::new());
        let previous_participation = self.enter_container_participation(style);
        body(self);
        self.restore_container_participation(previous_participation);
        let children = self
            .child_stacks
            .pop()
            .expect("the container stack was just pushed");
        let element = Element::flex(id, axis, style.gap).with_children(children);
        self.push(style.apply(Self::paint_container(element, style.background)));
    }

    fn paint_container(element: Element, background: Option<Color>) -> Element {
        match background {
            Some(color) => element.with_paint(PaintPrimitive::SolidRect { color }),
            None => element.without_paint(),
        }
    }

    fn enter_container_participation(&mut self, style: DeclarationStyle) -> (bool, bool) {
        let previous = (self.effective_hidden, self.effective_disabled);
        self.effective_hidden |= style.hidden;
        self.effective_disabled |= style.disabled;
        previous
    }

    fn restore_container_participation(&mut self, participation: (bool, bool)) {
        (self.effective_hidden, self.effective_disabled) = participation;
    }

    /// Declare styled text measured by the backend-independent text boundary.
    pub fn text(&mut self, id: WidgetId, content: impl Into<String>, style: TextStyle) {
        let content = content.into();
        let element = Element::text_with_properties(id, content.clone(), style.properties.clone())
            .with_paint(PaintPrimitive::Text {
                content,
                properties: style.properties,
                color: style.color,
            });
        self.push(style.layout.apply(element));
    }

    /// Declare an image by stable renderer-neutral resource key.
    ///
    /// Intrinsic dimensions are supplied later through [`IntrinsicMeasurer`];
    /// declaration never reads a GPU atlas or executes a measurement callback.
    pub fn image(&mut self, id: WidgetId, resource: u64, style: ImageStyle) -> Response {
        let effective_hidden = self.effective_hidden || style.layout.hidden;
        let effective_disabled = self.effective_disabled || style.layout.disabled || style.disabled;
        let response = self.pointer_response(id, effective_hidden || effective_disabled);

        let mut semantics = SemanticProperties::new(SemanticRole::Image);
        if let Some(label) = style.label {
            semantics = semantics.with_label(label);
        }
        if style.disabled {
            semantics = semantics.disabled();
        }

        let mut element = Element::image(id, resource).with_semantics(semantics);
        if !style.disabled {
            element = element.interactive();
        }
        self.push(
            style
                .layout
                .apply(element)
                .with_disabled(style.layout.disabled || style.disabled),
        );

        Response {
            disabled: effective_disabled,
            ..response
        }
    }

    /// Declare a solid rectangle with no renderer or GPU dependency.
    pub fn solid_rect(&mut self, id: WidgetId, style: DeclarationStyle, color: Color) {
        let element =
            Element::flex(id, Axis::Column, 0.0).with_paint(PaintPrimitive::SolidRect { color });
        self.push(style.apply(element));
    }

    /// Declare a rectangular border with no renderer or GPU dependency.
    pub fn border(&mut self, id: WidgetId, style: DeclarationStyle, color: Color, width: f32) {
        let element = Element::flex(id, Axis::Column, 0.0)
            .with_paint(PaintPrimitive::Border { color, width });
        self.push(style.apply(element));
    }

    /// Declare a semantic separator with exact renderer-neutral solid paint.
    ///
    /// Horizontal separators stretch through a column and own their height;
    /// vertical separators stretch through a row and own their width. The
    /// application-supplied stable ID is used unchanged in layout, paint,
    /// damage, and semantic products.
    pub fn separator(&mut self, id: WidgetId, mut style: SeparatorStyle) -> Result<(), FrameError> {
        if !style.thickness.is_finite() || style.thickness <= 0.0 {
            let error = FrameError::InvalidSeparator {
                id,
                error: SeparatorDeclarationError::InvalidThickness(style.thickness),
            };
            self.error = Some(error.clone());
            return Err(error);
        }

        match style.orientation {
            SeparatorOrientation::Horizontal => {
                style.layout.height = Some(style.thickness);
                style.layout.min_height = Some(style.thickness);
                style.layout.max_height = Some(style.thickness);
            }
            SeparatorOrientation::Vertical => {
                style.layout.width = Some(style.thickness);
                style.layout.min_width = Some(style.thickness);
                style.layout.max_width = Some(style.thickness);
            }
        }
        let element = Element::flex(id, Axis::Column, 0.0)
            .with_paint(PaintPrimitive::SolidRect { color: style.color })
            .with_semantics(SemanticProperties::new(SemanticRole::Separator));
        self.push(style.layout.apply(element));
        Ok(())
    }

    /// Declare a determinate progress indicator using exact rounded paint and range semantics.
    ///
    /// Values are never clamped and rounded paint is never approximated. The application ID
    /// remains the track and semantic identity. The fill and unpainted layout remainder use
    /// [`progress_fill_id`] and [`progress_remainder_id`], which applications must not reuse.
    pub fn progress(&mut self, id: WidgetId, style: ProgressStyle) -> Result<(), FrameError> {
        let valid_range =
            style.minimum.is_finite() && style.maximum.is_finite() && style.minimum < style.maximum;
        if !valid_range {
            let error = FrameError::InvalidProgress {
                id,
                error: ProgressDeclarationError::InvalidRange {
                    minimum: style.minimum,
                    maximum: style.maximum,
                },
            };
            self.error = Some(error.clone());
            return Err(error);
        }
        if !style.value.is_finite() || style.value < style.minimum || style.value > style.maximum {
            let error = FrameError::InvalidProgress {
                id,
                error: ProgressDeclarationError::ValueOutOfRange {
                    minimum: style.minimum,
                    maximum: style.maximum,
                    value: style.value,
                },
            };
            self.error = Some(error.clone());
            return Err(error);
        }
        if !style.radius.is_finite() || style.radius < 0.0 {
            let error = FrameError::InvalidProgress {
                id,
                error: ProgressDeclarationError::InvalidRadius(style.radius),
            };
            self.error = Some(error.clone());
            return Err(error);
        }

        let fraction = ((f64::from(style.value) - f64::from(style.minimum))
            / (f64::from(style.maximum) - f64::from(style.minimum))) as f32;
        let mut children = Vec::with_capacity(2);
        if fraction > 0.0 {
            children.push(
                Element::flex(progress_fill_id(id), Axis::Column, 0.0)
                    .with_flex_basis(0.0)
                    .with_flex_grow(fraction)
                    .with_paint(PaintPrimitive::RoundedRect {
                        color: style.fill_color,
                        radius: style.radius,
                    }),
            );
        }
        if fraction < 1.0 {
            children.push(
                Element::flex(progress_remainder_id(id), Axis::Column, 0.0)
                    .with_flex_basis(0.0)
                    .with_flex_grow(1.0 - fraction)
                    .without_paint(),
            );
        }

        let semantics = SemanticProperties::new(SemanticRole::ProgressBar).with_value_range(
            SemanticValueRange {
                minimum: style.minimum,
                maximum: style.maximum,
                value: style.value,
            },
        );
        let element = Element::flex(id, Axis::Row, 0.0)
            .with_children(children)
            .with_paint(PaintPrimitive::RoundedRect {
                color: style.track_color,
                radius: style.radius,
            })
            .with_semantics(semantics);
        self.push(style.layout.apply(element));
        Ok(())
    }

    /// Declare a left/right split pane with a retained, draggable divider ratio.
    pub fn split_pane_h(
        &mut self,
        id: WidgetId,
        initial_ratio: f32,
        style: SplitPaneStyle,
        first: impl FnOnce(&mut Self),
        second: impl FnOnce(&mut Self),
    ) {
        self.split_pane(id, Axis::Row, initial_ratio, style, first, second);
    }

    /// Declare a top/bottom split pane with a retained, draggable divider ratio.
    pub fn split_pane_v(
        &mut self,
        id: WidgetId,
        initial_ratio: f32,
        style: SplitPaneStyle,
        first: impl FnOnce(&mut Self),
        second: impl FnOnce(&mut Self),
    ) {
        self.split_pane(id, Axis::Column, initial_ratio, style, first, second);
    }

    fn split_pane(
        &mut self,
        id: WidgetId,
        axis: Axis,
        initial_ratio: f32,
        style: SplitPaneStyle,
        first: impl FnOnce(&mut Self),
        second: impl FnOnce(&mut Self),
    ) {
        const MIN_RATIO: f32 = 0.05;
        const MAX_RATIO: f32 = 0.95;

        let ids = SplitPaneIds::new(id);
        let ratio_state = split_ratio_state_id(id);
        let drag_state = split_drag_state_id(id);
        let pointer_state = split_pointer_state_id(id);
        let default_ratio = if initial_ratio.is_finite() {
            initial_ratio.clamp(MIN_RATIO, MAX_RATIO)
        } else {
            0.5
        };
        let mut ratio = self
            .state()
            .get(ratio_state)
            .map(|bits| f32::from_bits(bits as u32))
            .filter(|ratio| ratio.is_finite())
            .unwrap_or(default_ratio)
            .clamp(MIN_RATIO, MAX_RATIO);
        let mut dragging = self
            .state()
            .get(drag_state)
            .is_some_and(|active| active != 0);
        let mut drag_pointer = self.state().get(pointer_state).unwrap_or_default();
        if dragging && self.state().pointer_capture_owner(drag_pointer) != Some(ids.divider) {
            dragging = false;
        }
        let suppressed = self.effective_hidden
            || self.effective_disabled
            || style.layout.hidden
            || style.layout.disabled;

        while let Some(response) = self.state().take_response(ids.divider) {
            if suppressed {
                continue;
            }
            match response.kind {
                PointerEventKind::Press if !dragging => {
                    dragging = true;
                    drag_pointer = response.pointer;
                    self.state()
                        .request_pointer_capture(response.pointer, ids.divider);
                }
                PointerEventKind::Move if dragging && response.pointer == drag_pointer => {
                    if let Some(parent) = response.target_parent_bounds {
                        let (position, start, parent_extent, divider_extent) = match axis {
                            Axis::Row => (
                                response.position.x,
                                parent.x,
                                parent.width,
                                response.target_bounds.width,
                            ),
                            Axis::Column => (
                                response.position.y,
                                parent.y,
                                parent.height,
                                response.target_bounds.height,
                            ),
                        };
                        let available = parent_extent - divider_extent;
                        if available.is_finite() && available > 0.0 {
                            let requested = (position - start - divider_extent * 0.5) / available;
                            if requested.is_finite() {
                                ratio = requested.clamp(MIN_RATIO, MAX_RATIO);
                            }
                        }
                    }
                }
                PointerEventKind::Release | PointerEventKind::Cancel
                    if response.pointer == drag_pointer =>
                {
                    dragging = false;
                    self.state().request_pointer_release(response.pointer);
                }
                PointerEventKind::Press
                | PointerEventKind::Move
                | PointerEventKind::Release
                | PointerEventKind::Cancel => {}
            }
        }
        if suppressed && dragging {
            self.state().request_pointer_release(drag_pointer);
            dragging = false;
        }
        self.state().insert(ratio_state, u64::from(ratio.to_bits()));
        self.state().insert(drag_state, u64::from(dragging));
        self.state().insert(pointer_state, drag_pointer);

        let previous_participation = self.enter_container_participation(style.layout);
        let first_children = self.capture_children(first);
        let second_children = self.capture_children(second);
        self.restore_container_participation(previous_participation);

        let padding = if style.panel_padding.is_finite() {
            style.panel_padding.max(0.0)
        } else {
            0.0
        };
        let divider_width = if style.divider_width.is_finite() {
            style.divider_width.max(0.0)
        } else {
            0.0
        };
        let panel = |panel_id, grow, children| {
            Element::flex(panel_id, Axis::Column, 0.0)
                .without_paint()
                .with_padding(padding)
                .with_flex_basis(0.0)
                .with_flex_grow(grow)
                .with_min_size(Some(0.0), Some(0.0))
                .with_children(children)
        };
        let divider = match axis {
            Axis::Row => Element::flex(ids.divider, Axis::Column, 0.0)
                .with_size(Some(divider_width), None)
                .with_min_size(Some(divider_width), Some(0.0))
                .with_cursor_icon(CursorIcon::ColResize),
            Axis::Column => Element::flex(ids.divider, Axis::Column, 0.0)
                .with_size(None, Some(divider_width))
                .with_min_size(Some(0.0), Some(divider_width))
                .with_cursor_icon(CursorIcon::RowResize),
        }
        .with_paint(PaintPrimitive::SolidRect {
            color: style.divider_color,
        })
        .ordered_pointer_target()
        .interactive();
        let root = Element::flex(ids.root, axis, 0.0).with_children(vec![
            panel(ids.first, ratio, first_children),
            divider,
            panel(ids.second, 1.0 - ratio, second_children),
        ]);
        self.push(
            style
                .layout
                .apply(Self::paint_container(root, style.layout.background)),
        );
    }

    fn capture_children(&mut self, body: impl FnOnce(&mut Self)) -> Vec<Element> {
        self.child_stacks.push(Vec::new());
        body(self);
        self.child_stacks
            .pop()
            .expect("the compound child stack was just pushed")
    }

    fn capture_children_result<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> T,
    ) -> (T, Vec<Element>) {
        self.child_stacks.push(Vec::new());
        let result = body(self);
        let children = self
            .child_stacks
            .pop()
            .expect("the compound child stack was just pushed");
        (result, children)
    }

    /// Declare a basic button and consume responses dispatched from the last
    /// committed scene before this tree began construction.
    pub fn button(
        &mut self,
        id: WidgetId,
        label: impl Into<String>,
        style: ButtonStyle,
    ) -> Response {
        let label = label.into();
        let effective_hidden = self.effective_hidden || style.layout.hidden;
        let effective_disabled = self.effective_disabled || style.layout.disabled || style.disabled;
        let suppress_responses = effective_hidden || effective_disabled;
        let response = self.pointer_response(id, suppress_responses);

        let ids = ButtonIds::new(id);
        let text =
            Element::text_with_properties(ids.label, label.clone(), style.text.properties.clone())
                .with_paint(PaintPrimitive::Text {
                    content: label.clone(),
                    properties: style.text.properties,
                    color: style.text.color,
                });
        let text = style.text.layout.apply(text);
        let fill = Element::flex(ids.fill, Axis::Column, 0.0)
            .with_padding(8.0)
            .with_flex_grow(1.0)
            .with_paint(PaintPrimitive::SolidRect { color: style.fill })
            .with_children(vec![text]);
        let semantics = if style.disabled {
            SemanticProperties::new(SemanticRole::Button)
                .with_label(label)
                .disabled()
        } else {
            SemanticProperties::new(SemanticRole::Button).with_label(label)
        };
        let mut button = Element::flex(ids.root, Axis::Column, 0.0)
            .with_paint(PaintPrimitive::Border {
                color: style.border,
                width: style.border_width,
            })
            .with_semantics(semantics)
            .with_children(vec![fill]);
        if !style.disabled {
            button = button.interactive();
        }
        self.push(
            style
                .layout
                .apply(button)
                .with_disabled(style.layout.disabled || style.disabled),
        );

        Response {
            disabled: effective_disabled,
            ..response
        }
    }

    /// Declare a controlled checkbox and consume pointer input from the last
    /// committed scene.
    ///
    /// The declaration never mutates application state. A click returns
    /// `changed = true`; the caller supplies the next checked value. Indicator
    /// geometry and paint stay renderer-neutral and are rejected rather than
    /// clamped when they cannot be represented exactly.
    pub fn checkbox(
        &mut self,
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        style: CheckboxStyle,
    ) -> Result<Response, FrameError> {
        let reject = |error| FrameError::InvalidCheckbox { id, error };
        if !style.indicator_size.is_finite() || style.indicator_size <= 0.0 {
            let error = reject(CheckboxDeclarationError::InvalidIndicatorSize(
                style.indicator_size,
            ));
            self.error = Some(error.clone());
            return Err(error);
        }
        if !style.gap.is_finite() || style.gap < 0.0 {
            let error = reject(CheckboxDeclarationError::InvalidGap(style.gap));
            self.error = Some(error.clone());
            return Err(error);
        }
        if !style.radius.is_finite() || style.radius < 0.0 {
            let error = reject(CheckboxDeclarationError::InvalidRadius(style.radius));
            self.error = Some(error.clone());
            return Err(error);
        }
        if style.radius > style.indicator_size / 2.0 {
            let error = reject(CheckboxDeclarationError::RadiusExceedsIndicator {
                radius: style.radius,
                indicator_size: style.indicator_size,
            });
            self.error = Some(error.clone());
            return Err(error);
        }

        let label = label.into();
        let effective_hidden = self.effective_hidden || style.layout.hidden;
        let effective_disabled = self.effective_disabled || style.layout.disabled || style.disabled;
        let suppress_responses = effective_hidden || effective_disabled;
        let mut response = self.pointer_response(id, suppress_responses);
        response.changed = response.clicked;
        response.disabled = effective_disabled;

        let ids = CheckboxIds::new(id);
        let label_element =
            Element::text_with_properties(ids.label, label.clone(), style.label.properties.clone())
                .with_paint(PaintPrimitive::Text {
                    content: label.clone(),
                    properties: style.label.properties,
                    color: style.label.color,
                });
        let label_element = style.label.layout.apply(label_element);

        let mut indicator_children = Vec::with_capacity(1);
        if checked {
            let mark =
                Element::text_with_properties(ids.mark, "\u{2713}", style.mark.properties.clone())
                    .with_paint(PaintPrimitive::Text {
                        content: "\u{2713}".to_owned(),
                        properties: style.mark.properties,
                        color: style.mark.color,
                    });
            indicator_children.push(style.mark.layout.apply(mark));
        }
        let indicator = Element::flex(ids.indicator, Axis::Column, 0.0)
            .with_size(Some(style.indicator_size), Some(style.indicator_size))
            .with_min_size(Some(style.indicator_size), Some(style.indicator_size))
            .with_max_size(Some(style.indicator_size), Some(style.indicator_size))
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_paint(PaintPrimitive::RoundedRect {
                color: if checked {
                    style.checked_fill
                } else {
                    style.unchecked_fill
                },
                radius: style.radius,
            })
            .with_children(indicator_children);

        let mut semantics = SemanticProperties::new(SemanticRole::Checkbox)
            .with_label(label)
            .with_checked(checked);
        if style.disabled || style.layout.disabled {
            semantics = semantics.disabled();
        }
        let mut checkbox = style
            .layout
            .apply(Element::flex(ids.root, Axis::Row, style.gap))
            .with_main_axis_alignment(MainAxisAlignment::Start)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .without_paint()
            .with_semantics(semantics)
            .with_children(vec![indicator, label_element]);
        if !style.disabled {
            checkbox = checkbox.interactive();
        }
        self.push(checkbox.with_disabled(style.layout.disabled || style.disabled));

        Ok(response)
    }

    fn pointer_response(&mut self, id: WidgetId, suppress: bool) -> Response {
        let state_id = interaction_state_id(id);
        let mut pressed = self.state().get(state_id).is_some_and(|value| value != 0);
        let mut clicked = false;
        let mut hovered = false;
        while let Some(response) = self.state().take_response(id) {
            if suppress {
                continue;
            }
            match response.kind {
                PointerEventKind::Press => {
                    pressed = true;
                    hovered = true;
                }
                PointerEventKind::Move => hovered = true,
                PointerEventKind::Release => {
                    clicked = pressed;
                    pressed = false;
                    hovered = true;
                }
                PointerEventKind::Cancel => pressed = false,
            }
        }
        if suppress {
            clicked = false;
            pressed = false;
            hovered = false;
        }
        self.state().insert(state_id, u64::from(pressed));
        Response {
            clicked,
            hovered,
            pressed,
            ..Response::default()
        }
    }

    fn push(&mut self, element: Element) {
        self.child_stacks
            .last_mut()
            .expect("a declaration child stack always exists")
            .push(element);
    }
}

/// Dispatch committed-scene input, execute the application declaration once,
/// resolve current-frame geometry, and atomically commit its products.
pub fn run_declaration_frame<'a, M, C, F>(
    core: &'a mut FrameCore,
    measurer: &M,
    consumer: &mut C,
    declare: F,
) -> Result<&'a CommittedScene, FrameError>
where
    M: IntrinsicMeasurer,
    C: SceneConsumer,
    F: FnOnce(&mut DeclarationUi<'_>),
{
    let mut attempt = core.begin_generation();
    let mut ui = DeclarationUi::from_attempt(&mut attempt);
    declare(&mut ui);
    let root = ui.try_finish()?;
    core.finish_generation(attempt, root, measurer, consumer)
}
