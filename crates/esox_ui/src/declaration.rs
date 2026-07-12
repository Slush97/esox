//! Current-frame production declarations backed by [`crate::frame_core::FrameCore`].
//!
//! This is intentionally a small vertical slice. Declarations only build an
//! owned element tree; measurement, layout, paint record creation, hit testing,
//! semantics, and damage are resolved after the application closure returns.

use crate::frame_core::{
    Axis, Color, CommittedScene, Element, FrameCore, FrameError, GenerationAttempt,
    IntrinsicMeasurer, PaintPrimitive, PointerEventKind, SceneConsumer, SemanticProperties,
    SemanticRole, TextProperties, VirtualListSpec, VirtualWindow, WidgetId, WidgetStateStore,
};
use crate::response::Response;
use esox_input::CursorIcon;

pub use crate::frame_core::{CrossAxisAlignment, GridTrack, LogicalTransform, MainAxisAlignment};

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

const fn derived_id(parent: WidgetId, salt: u64) -> WidgetId {
    WidgetId(parent.0.rotate_left(23) ^ salt)
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

    /// Declare a basic button and consume responses dispatched from the last
    /// committed scene before this tree began construction.
    pub fn button(
        &mut self,
        id: WidgetId,
        label: impl Into<String>,
        style: ButtonStyle,
    ) -> Response {
        let label = label.into();
        let state_id = interaction_state_id(id);
        let mut pressed = self.state().get(state_id).is_some_and(|value| value != 0);
        let mut clicked = false;
        let mut hovered = false;
        let effective_hidden = self.effective_hidden || style.layout.hidden;
        let effective_disabled = self.effective_disabled || style.layout.disabled || style.disabled;
        let suppress_responses = effective_hidden || effective_disabled;
        while let Some(response) = self.state().take_response(id) {
            if suppress_responses {
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
        if suppress_responses {
            clicked = false;
            pressed = false;
            hovered = false;
        }
        self.state().insert(state_id, u64::from(pressed));

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
            clicked,
            hovered,
            pressed,
            disabled: effective_disabled,
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
