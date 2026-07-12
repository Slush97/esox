//! GPU- and window-independent current-frame scene construction.
//!
//! This is the narrow frame core used to establish the ADR-001 lifecycle
//! contracts while the production widget API is migrated incrementally.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use esox_input::CursorIcon;
use serde::{Deserialize, Serialize};
use taffy::prelude::{
    AlignContent, AlignItems, AvailableSpace, Display, FlexDirection, JustifyContent, NodeId,
    Position, Rect, Size, Style, TaffyMaxContent, TaffyTree,
};
use taffy::style::Overflow;
use taffy::style_helpers::{auto, fr, length};

/// Stable widget identity within one [`FrameCore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(pub u64);

/// An unrounded rectangle in logical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LogicalRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LogicalRect {
    fn contains(self, point: LogicalPoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        // `None` means that no clip is active throughout the scene model. Keep
        // an explicit zero-area rectangle when two active clips are disjoint,
        // otherwise a fully clipped descendant would become unclipped.
        Some(Self {
            x,
            y,
            width: (right - x).max(0.0),
            height: (bottom - y).max(0.0),
        })
    }
}

/// An unrounded point in logical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LogicalPoint {
    pub x: f32,
    pub y: f32,
}

impl LogicalPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A renderer-neutral 2D transform applied after layout resolution.
///
/// Scaling is resolved around the transformed element's current-generation
/// layout center. Layout constraints and Taffy's unrounded geometry are never
/// changed by this value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogicalTransform {
    translate_x: f32,
    translate_y: f32,
    scale_x: f32,
    scale_y: f32,
}

impl LogicalTransform {
    /// Create a translation and independent X/Y scale.
    pub const fn new(translate_x: f32, translate_y: f32, scale_x: f32, scale_y: f32) -> Self {
        Self {
            translate_x,
            translate_y,
            scale_x,
            scale_y,
        }
    }

    /// Create a translation-only transform.
    pub const fn translate(x: f32, y: f32) -> Self {
        Self::new(x, y, 1.0, 1.0)
    }

    /// Create an independent X/Y scale with no translation.
    pub const fn scale(x: f32, y: f32) -> Self {
        Self::new(0.0, 0.0, x, y)
    }
}

impl Default for LogicalTransform {
    fn default() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedTransform {
    scale_x: f32,
    scale_y: f32,
    translate_x: f32,
    translate_y: f32,
}

impl ResolvedTransform {
    const IDENTITY: Self = Self {
        scale_x: 1.0,
        scale_y: 1.0,
        translate_x: 0.0,
        translate_y: 0.0,
    };

    fn local(transform: LogicalTransform, bounds: LogicalRect) -> Option<Self> {
        let center_x = bounds.x + bounds.width * 0.5;
        let center_y = bounds.y + bounds.height * 0.5;
        let resolved = Self {
            scale_x: transform.scale_x,
            scale_y: transform.scale_y,
            translate_x: transform.translate_x + center_x * (1.0 - transform.scale_x),
            translate_y: transform.translate_y + center_y * (1.0 - transform.scale_y),
        };
        resolved.is_valid().then_some(resolved)
    }

    /// Compose a child-local transform after this parent transform.
    fn compose(self, child: Self) -> Option<Self> {
        let resolved = Self {
            scale_x: self.scale_x * child.scale_x,
            scale_y: self.scale_y * child.scale_y,
            translate_x: self.scale_x * child.translate_x + self.translate_x,
            translate_y: self.scale_y * child.translate_y + self.translate_y,
        };
        resolved.is_valid().then_some(resolved)
    }

    fn rect(self, rect: LogicalRect) -> Option<LogicalRect> {
        let x1 = rect.x * self.scale_x + self.translate_x;
        let y1 = rect.y * self.scale_y + self.translate_y;
        let x2 = (rect.x + rect.width) * self.scale_x + self.translate_x;
        let y2 = (rect.y + rect.height) * self.scale_y + self.translate_y;
        let transformed = LogicalRect {
            x: x1.min(x2),
            y: y1.min(y2),
            width: (x2 - x1).abs(),
            height: (y2 - y1).abs(),
        };
        (transformed.x.is_finite()
            && transformed.y.is_finite()
            && transformed.width.is_finite()
            && transformed.height.is_finite())
        .then_some(transformed)
    }

    fn is_valid(self) -> bool {
        self.scale_x.is_finite()
            && self.scale_y.is_finite()
            && self.translate_x.is_finite()
            && self.translate_y.is_finite()
            && self.scale_x != 0.0
            && self.scale_y != 0.0
    }
}

/// An unrounded size in logical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LogicalSize {
    pub width: f32,
    pub height: f32,
}

/// A backend-neutral linear RGBA color.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

impl LogicalSize {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn is_valid(self) -> bool {
        self.width.is_finite() && self.height.is_finite() && self.width > 0.0 && self.height > 0.0
    }
}

/// Main axis for a flex container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Row,
    Column,
}

/// A grid track understood by the minimal Taffy adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridTrack {
    Fixed(f32),
    Fraction(f32),
    Auto,
}

/// Alignment of children along a container's main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Alignment of children along a container's cross axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
enum ElementKind {
    Flex {
        axis: Axis,
        gap: f32,
    },
    Grid {
        columns: Vec<GridTrack>,
        column_gap: f32,
        row_gap: f32,
    },
    Fixed(LogicalSize),
    Text {
        content: String,
        properties: TextProperties,
    },
    Image(u64),
}

/// Base direction hint carried to the selected text backend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Auto,
    LeftToRight,
    RightToLeft,
}

/// Esox-owned shaping and measurement properties for one text leaf.
#[derive(Clone, Debug, PartialEq)]
pub struct TextProperties {
    pub font_family: Option<String>,
    pub font_size: f32,
    pub font_weight: u16,
    pub locale: Option<String>,
    pub direction: TextDirection,
}

impl Default for TextProperties {
    fn default() -> Self {
        Self {
            font_family: None,
            font_size: 16.0,
            font_weight: 400,
            locale: None,
            direction: TextDirection::Auto,
        }
    }
}

/// Backend-neutral paint data attached to one declared element.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    Box,
    SolidRect {
        color: Color,
    },
    RoundedRect {
        color: Color,
        radius: f32,
    },
    Border {
        color: Color,
        width: f32,
    },
    Text {
        content: String,
        properties: TextProperties,
        color: Color,
    },
    Image {
        resource: u64,
    },
}

/// Esox-owned semantic role, independent of any accessibility backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticRole {
    Generic,
    Text,
    Image,
    Button,
    Checkbox,
    Separator,
    ProgressBar,
    ScrollView,
}

/// A determinate numeric range advertised without an accessibility-backend type.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticValueRange {
    pub minimum: f32,
    pub maximum: f32,
    pub value: f32,
}

/// Serializable semantic properties declared without resolved geometry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticProperties {
    pub role: SemanticRole,
    pub label: Option<String>,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub value_range: Option<SemanticValueRange>,
}

impl SemanticProperties {
    pub fn new(role: SemanticRole) -> Self {
        Self {
            role,
            label: None,
            disabled: false,
            checked: None,
            value_range: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn with_value_range(mut self, value_range: SemanticValueRange) -> Self {
        self.value_range = Some(value_range);
        self
    }
}

/// One node in the lightweight tree owned only for the current frame.
#[derive(Clone, Debug)]
pub struct Element {
    id: WidgetId,
    kind: ElementKind,
    children: Vec<Self>,
    flex_grow: f32,
    flex_shrink: Option<f32>,
    flex_basis: Option<f32>,
    padding: f32,
    size: Size<Option<f32>>,
    min_size: Size<Option<f32>>,
    max_size: Size<Option<f32>>,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    paint: Option<PaintPrimitive>,
    interactive: bool,
    ordered_pointer_target: bool,
    interaction_owner: Option<WidgetId>,
    cursor_icon: Option<CursorIcon>,
    semantics: Option<SemanticProperties>,
    clips_children: bool,
    scrollable: bool,
    requested_scroll_offset: Option<LogicalPoint>,
    virtual_content_height: Option<f32>,
    absolute_position: Option<LogicalPoint>,
    blocking_overlay: bool,
    hidden: bool,
    disabled: bool,
    transform: LogicalTransform,
}

impl Element {
    fn new(id: WidgetId, kind: ElementKind) -> Self {
        let paint = match &kind {
            ElementKind::Text {
                content,
                properties,
            } => PaintPrimitive::Text {
                content: content.clone(),
                properties: properties.clone(),
                color: Color::BLACK,
            },
            ElementKind::Image(resource) => PaintPrimitive::Image {
                resource: *resource,
            },
            ElementKind::Flex { .. } | ElementKind::Grid { .. } | ElementKind::Fixed(_) => {
                PaintPrimitive::Box
            }
        };
        Self {
            id,
            kind,
            children: Vec::new(),
            flex_grow: 0.0,
            flex_shrink: None,
            flex_basis: None,
            padding: 0.0,
            size: Size::NONE,
            min_size: Size::NONE,
            max_size: Size::NONE,
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            paint: Some(paint),
            interactive: false,
            ordered_pointer_target: false,
            interaction_owner: None,
            cursor_icon: None,
            semantics: None,
            clips_children: false,
            scrollable: false,
            requested_scroll_offset: None,
            virtual_content_height: None,
            absolute_position: None,
            blocking_overlay: false,
            hidden: false,
            disabled: false,
            transform: LogicalTransform::default(),
        }
    }

    /// Create a flex container.
    pub fn flex(id: WidgetId, axis: Axis, gap: f32) -> Self {
        Self::new(id, ElementKind::Flex { axis, gap })
    }

    /// Create a grid container with explicit columns and a uniform gap.
    pub fn grid(id: WidgetId, columns: Vec<GridTrack>, gap: f32) -> Self {
        Self::grid_with_gaps(id, columns, gap, gap)
    }

    /// Create a grid with independent column and row gaps and deterministic implicit rows.
    pub fn grid_with_gaps(
        id: WidgetId,
        columns: Vec<GridTrack>,
        column_gap: f32,
        row_gap: f32,
    ) -> Self {
        Self::new(
            id,
            ElementKind::Grid {
                columns,
                column_gap,
                row_gap,
            },
        )
    }

    /// Create a fixed-size leaf.
    pub fn fixed(id: WidgetId, width: f32, height: f32) -> Self {
        Self::new(id, ElementKind::Fixed(LogicalSize::new(width, height)))
    }

    /// Create a deterministically measured text leaf.
    pub fn text(id: WidgetId, text: impl Into<String>) -> Self {
        Self::text_with_properties(id, text, TextProperties::default())
    }

    /// Create a text leaf with backend-neutral shaping and locale hints.
    pub fn text_with_properties(
        id: WidgetId,
        text: impl Into<String>,
        properties: TextProperties,
    ) -> Self {
        let text = text.into();
        let mut element = Self::new(
            id,
            ElementKind::Text {
                content: text.clone(),
                properties,
            },
        );
        element.semantics = Some(SemanticProperties::new(SemanticRole::Text).with_label(text));
        element
    }

    /// Create an image leaf measured from decoded image metadata.
    pub fn image(id: WidgetId, image: u64) -> Self {
        Self::new(id, ElementKind::Image(image))
    }

    /// Set the owned children of this node.
    pub fn with_children(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }

    /// Let this node consume remaining space on its parent's main axis.
    pub fn with_flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Set the flex shrink factor used when the parent is constrained.
    pub fn with_flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = Some(shrink);
        self
    }

    /// Set a logical flex basis independently from intrinsic child content.
    pub fn with_flex_basis(mut self, basis: f32) -> Self {
        self.flex_basis = Some(basis);
        self
    }

    /// Apply uniform logical padding inside this element.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Constrain either dimension to an exact logical size.
    pub fn with_size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        self.size = Size { width, height };
        self
    }

    /// Set optional minimum logical dimensions.
    pub fn with_min_size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        self.min_size = Size { width, height };
        self
    }

    /// Set optional maximum logical dimensions.
    pub fn with_max_size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        self.max_size = Size { width, height };
        self
    }

    /// Align children or grid tracks along this container's main axis.
    pub fn with_main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = alignment;
        self
    }

    /// Align children or grid tracks along this container's cross axis.
    pub fn with_cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    /// Exclude this container from paint while retaining its descendants.
    pub fn without_paint(mut self) -> Self {
        self.paint = None;
        self
    }

    /// Replace the renderer-neutral paint primitive for this node.
    pub fn with_paint(mut self, paint: PaintPrimitive) -> Self {
        self.paint = Some(paint);
        self
    }

    /// Include this node in hit testing and semantics.
    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        if self.semantics.is_none() {
            self.semantics = Some(SemanticProperties::new(SemanticRole::Generic));
        }
        self
    }

    /// Set the renderer-neutral cursor shown while this interactive node is hit.
    pub fn with_cursor_icon(mut self, cursor_icon: CursorIcon) -> Self {
        self.cursor_icon = Some(cursor_icon);
        self
    }

    /// Route later events in the same queued batch to this pressed node.
    pub fn ordered_pointer_target(mut self) -> Self {
        self.ordered_pointer_target = true;
        self
    }

    /// Associate this node and its descendants with a logical compound-widget part.
    pub fn with_interaction_owner(mut self, owner: WidgetId) -> Self {
        self.interaction_owner = Some(owner);
        self
    }

    /// Attach an Esox-owned semantic declaration to this node.
    pub fn with_semantics(mut self, semantics: SemanticProperties) -> Self {
        self.semantics = Some(semantics);
        self
    }

    /// Clip descendants to this node's resolved bounds.
    pub fn clip_children(mut self) -> Self {
        self.clips_children = true;
        self
    }

    /// Retain this viewport's last successfully applied FrameCore scroll offset.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Explicitly request this scroll viewport's current-frame offset.
    ///
    /// An explicit request takes precedence over retained FrameCore state for
    /// this generation. Use [`Self::scrollable`] on later declarations to let
    /// wheel input and the last successfully applied offset drive the viewport.
    pub fn with_scroll_offset(mut self, x: f32, y: f32) -> Self {
        self.scrollable = true;
        self.requested_scroll_offset = Some(LogicalPoint::new(x, y));
        self
    }

    /// Declare a full logical vertical content extent independently of children.
    ///
    /// This lets a virtual viewport clamp scrolling against all logical items
    /// while its current-generation tree contains only visible item wrappers.
    pub fn with_virtual_content_height(mut self, height: f32) -> Self {
        self.virtual_content_height = Some(height);
        self
    }

    fn apply_retained_scroll_offsets(&mut self, retained: &HashMap<WidgetId, ScrollOffsetState>) {
        if self.scrollable && self.requested_scroll_offset.is_none() {
            self.requested_scroll_offset = Some(
                retained
                    .get(&self.id)
                    .map(|state| state.requested)
                    .unwrap_or_default(),
            );
        }
        for child in &mut self.children {
            child.apply_retained_scroll_offsets(retained);
        }
    }

    /// Remove this node from normal flow and place it at a logical position.
    pub fn with_absolute_position(mut self, x: f32, y: f32) -> Self {
        self.absolute_position = Some(LogicalPoint::new(x, y));
        self
    }

    /// Make this node a semantic, focus-scoped blocker in the overlay layer.
    pub fn blocking_overlay(mut self) -> Self {
        self.blocking_overlay = true;
        self.interactive = true;
        if self.semantics.is_none() {
            self.semantics = Some(SemanticProperties::new(SemanticRole::Generic));
        }
        self
    }

    /// Collapse this element and its descendants out of layout and every scene product.
    ///
    /// Hidden is intentionally structural: it is distinct from disabled state and from
    /// paint suppression through [`Self::without_paint`]. The declaration remains in the
    /// current tree, but Taffy resolves it with `display: none`.
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Set whether this element and its descendants are structurally hidden.
    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Disable interaction for this element and all descendants without changing layout or paint.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Set whether this element and its descendants are disabled.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Transform this element and its descendants after current-generation layout.
    pub fn with_transform(mut self, transform: LogicalTransform) -> Self {
        self.transform = transform;
        self
    }
}

/// One pointer response routed through an immutable committed scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputResponse {
    pub kind: PointerEventKind,
    pub pointer: u64,
    pub target: WidgetId,
    pub committed_generation: u64,
    /// Original order in the dispatched pointer ledger for this generation.
    pub dispatch_ordinal: u64,
    /// Committed virtual viewport containing the target, when any.
    pub virtual_owner: Option<WidgetId>,
    /// Nearest committed compound-widget interaction owner, when any.
    pub interaction_owner: Option<WidgetId>,
    pub position: LogicalPoint,
    pub target_bounds: LogicalRect,
    /// Committed transformed bounds of the target's structural parent.
    pub target_parent_bounds: Option<LogicalRect>,
}

/// One keyboard response routed to the committed keyboard focus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardInputResponse {
    pub event: esox_input::KeyEvent,
    pub modifiers: esox_input::Modifiers,
    pub target: WidgetId,
    pub committed_generation: u64,
    /// Original order in the dispatched keyboard ledger for this generation.
    pub dispatch_ordinal: u64,
}

/// Pointer event kinds retained in committed-generation response order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerEventKind {
    Press,
    Move,
    Release,
    Cancel,
}

/// Persistent per-widget values, deliberately separate from [`Element`].
#[derive(Clone, Debug, Default)]
pub struct WidgetStateStore {
    values: HashMap<WidgetId, u64>,
    responses: HashMap<WidgetId, VecDeque<InputResponse>>,
    keyboard_responses: HashMap<WidgetId, VecDeque<KeyboardInputResponse>>,
    active_pointer_captures: HashMap<u64, WidgetId>,
    requested_keyboard_focus: Option<WidgetId>,
    requested_pointer_captures: HashMap<u64, WidgetId>,
    requested_pointer_releases: HashSet<u64>,
}

impl WidgetStateStore {
    /// Read a persistent value.
    pub fn get(&self, id: WidgetId) -> Option<u64> {
        self.values.get(&id).copied()
    }

    /// Replace a persistent value.
    pub fn insert(&mut self, id: WidgetId, value: u64) -> Option<u64> {
        self.values.insert(id, value)
    }

    /// Consume the oldest pending response for this widget at most once.
    pub fn take_response(&mut self, id: WidgetId) -> Option<InputResponse> {
        let queue = self.responses.get_mut(&id)?;
        let response = queue.pop_front();
        if queue.is_empty() {
            self.responses.remove(&id);
        }
        response
    }

    /// Drain matching responses across targets in their original ledger order.
    pub fn drain_responses(
        &mut self,
        mut matches: impl FnMut(&InputResponse) -> bool,
    ) -> Vec<InputResponse> {
        let mut drained = Vec::new();
        self.responses.retain(|_, queue| {
            let mut retained = VecDeque::with_capacity(queue.len());
            while let Some(response) = queue.pop_front() {
                if matches(&response) {
                    drained.push(response);
                } else {
                    retained.push_back(response);
                }
            }
            *queue = retained;
            !queue.is_empty()
        });
        drained.sort_by_key(|response| (response.committed_generation, response.dispatch_ordinal));
        drained
    }

    /// Consume the oldest pending keyboard response for this widget at most once.
    pub fn take_keyboard_response(&mut self, id: WidgetId) -> Option<KeyboardInputResponse> {
        let queue = self.keyboard_responses.get_mut(&id)?;
        let response = queue.pop_front();
        if queue.is_empty() {
            self.keyboard_responses.remove(&id);
        }
        response
    }

    /// Drain matching keyboard responses across targets in ledger order.
    pub fn drain_keyboard_responses(
        &mut self,
        mut matches: impl FnMut(&KeyboardInputResponse) -> bool,
    ) -> Vec<KeyboardInputResponse> {
        let mut drained = Vec::new();
        self.keyboard_responses.retain(|_, queue| {
            let mut retained = VecDeque::with_capacity(queue.len());
            while let Some(response) = queue.pop_front() {
                if matches(&response) {
                    drained.push(response);
                } else {
                    retained.push_back(response);
                }
            }
            *queue = retained;
            !queue.is_empty()
        });
        drained.sort_by_key(|response| (response.committed_generation, response.dispatch_ordinal));
        drained
    }

    /// Read the capture owner visible at the start of this generation.
    pub fn pointer_capture_owner(&self, pointer: u64) -> Option<WidgetId> {
        self.active_pointer_captures.get(&pointer).copied()
    }

    /// Request keyboard focus if this widget is focusable in the scene being built.
    pub fn request_keyboard_focus(&mut self, id: WidgetId) {
        self.requested_keyboard_focus = Some(id);
    }

    /// Request capture after the current declaration resolves successfully.
    pub fn request_pointer_capture(&mut self, pointer: u64, id: WidgetId) {
        self.requested_pointer_captures.insert(pointer, id);
        self.requested_pointer_releases.remove(&pointer);
    }

    /// Request explicit release of a pointer capture before the next commit.
    pub fn request_pointer_release(&mut self, pointer: u64) {
        self.requested_pointer_captures.remove(&pointer);
        self.requested_pointer_releases.insert(pointer);
    }
}

/// One dimension of constraint space supplied to an intrinsic measurer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AvailableLength {
    Definite(f32),
    MinContent,
    MaxContent,
}

/// Known dimensions supplied by Taffy for a leaf measurement query.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KnownDimensions {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Available width and height supplied by Taffy for a leaf measurement query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvailableSize {
    pub width: AvailableLength,
    pub height: AvailableLength,
}

/// Complete GPU-independent text measurement request.
#[derive(Clone, Copy, Debug)]
pub struct TextMeasureRequest<'a> {
    pub content: &'a str,
    pub properties: &'a TextProperties,
    pub known_dimensions: KnownDimensions,
    pub available_space: AvailableSize,
}

/// Complete GPU-independent image measurement request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageMeasureRequest {
    pub resource: u64,
    pub known_dimensions: KnownDimensions,
    pub available_space: AvailableSize,
}

/// GPU-independent intrinsic measurement used during Taffy layout.
pub trait IntrinsicMeasurer {
    fn measure_text(&self, request: TextMeasureRequest<'_>) -> LogicalSize;
    fn measure_image(&self, request: ImageMeasureRequest) -> LogicalSize;
}

/// Fixed-metric measurement fixture for headless contract tests.
#[derive(Clone, Debug)]
pub struct DeterministicMeasurer {
    glyph_width: f32,
    line_height: f32,
    images: HashMap<u64, LogicalSize>,
}

impl DeterministicMeasurer {
    /// Create a text measurer with fixed glyph and line metrics.
    pub fn new(glyph_width: f32, line_height: f32) -> Self {
        Self {
            glyph_width,
            line_height,
            images: HashMap::new(),
        }
    }

    /// Add deterministic decoded metadata for an image key.
    pub fn with_image(mut self, image: u64, size: LogicalSize) -> Self {
        self.images.insert(image, size);
        self
    }
}

impl IntrinsicMeasurer for DeterministicMeasurer {
    fn measure_text(&self, request: TextMeasureRequest<'_>) -> LogicalSize {
        let natural_width = request.content.chars().count() as f32 * self.glyph_width;
        let available_width = match request.available_space.width {
            AvailableLength::Definite(width) => Some(width),
            AvailableLength::MinContent | AvailableLength::MaxContent => None,
        };
        let width = request.known_dimensions.width.unwrap_or_else(|| {
            available_width
                .map(|available| natural_width.min(available.max(0.0)))
                .unwrap_or(natural_width)
        });
        let lines = if natural_width == 0.0 {
            1.0
        } else {
            (natural_width / width.max(1.0)).ceil()
        };
        LogicalSize::new(
            width,
            request
                .known_dimensions
                .height
                .unwrap_or(lines * self.line_height),
        )
    }

    fn measure_image(&self, request: ImageMeasureRequest) -> LogicalSize {
        let intrinsic = self
            .images
            .get(&request.resource)
            .copied()
            .unwrap_or_default();
        LogicalSize::new(
            request.known_dimensions.width.unwrap_or(intrinsic.width),
            request.known_dimensions.height.unwrap_or(intrinsic.height),
        )
    }
}

#[derive(Clone, Debug)]
enum MeasureContext {
    Text {
        content: String,
        properties: TextProperties,
    },
    Image(u64),
}

/// Current-generation scroll geometry for one explicitly declared scroll viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMetrics {
    /// Resolved viewport extent used to clip descendants.
    pub viewport_extent: LogicalSize,
    /// Resolved scrollable extent, never smaller than the viewport.
    pub content_extent: LogicalSize,
    /// Raw caller-owned offset declared for this generation.
    pub requested_offset: LogicalPoint,
    /// Offset sanitized and clamped against this generation's extents.
    pub applied_offset: LogicalPoint,
    /// Largest valid offset on each axis for this generation.
    pub maximum_offset: LogicalPoint,
}

/// Per-window scroll offset retained across successful FrameCore generations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollOffsetState {
    /// Offset requested by input or the declaration that last committed.
    pub requested: LogicalPoint,
    /// Current-generation sanitized and clamped offset used by scene products.
    pub applied: LogicalPoint,
}

/// Inputs for one uniform-height vertical virtual viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListSpec {
    pub id: WidgetId,
    pub item_count: usize,
    pub item_height: f32,
    pub viewport_height: f32,
    pub explicit_offset: Option<f32>,
    pub scroll_to: Option<usize>,
}

impl VirtualListSpec {
    pub const fn new(
        id: WidgetId,
        item_count: usize,
        item_height: f32,
        viewport_height: f32,
    ) -> Self {
        Self {
            id,
            item_count,
            item_height,
            viewport_height,
            explicit_offset: None,
            scroll_to: None,
        }
    }

    /// Override retained and wheel-driven state for this generation.
    pub const fn with_offset(mut self, offset: f32) -> Self {
        self.explicit_offset = Some(offset);
        self
    }

    /// Minimally reveal one logical item unless an explicit offset is present.
    pub const fn scroll_to(mut self, item: usize) -> Self {
        self.scroll_to = Some(item);
        self
    }
}

/// Candidate virtual range and scroll geometry visible during declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualWindow {
    pub visible_range: Range<usize>,
    pub requested_offset: f32,
    pub applied_offset: f32,
    pub maximum_offset: f32,
    pub content_height: f32,
}

/// Invalid virtual content rejected before any item callback executes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VirtualDeclarationError {
    CandidateStateUnavailable,
    ConflictingViewportStyle,
    InvalidItemHeight(f32),
    InvalidViewportHeight(f32),
    InvalidContentHeight(f32),
    UnrepresentableContentExtent {
        item_count: usize,
        item_height: f32,
    },
    UnrepresentableScrollGeometry {
        item_count: usize,
        item_height: f32,
        viewport_height: f32,
    },
    UnrepresentableItemPositions {
        item_count: usize,
        item_height: f32,
    },
}

/// All scene products for one resolved node, sourced by one traversal.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedNode {
    pub id: WidgetId,
    /// Stable parent relationship in this immutable committed generation.
    pub parent: Option<WidgetId>,
    /// Still-declared virtual viewport owning this visible descendant.
    pub virtual_owner: Option<WidgetId>,
    /// Nearest logical compound-widget owner inherited by this node.
    pub interaction_owner: Option<WidgetId>,
    pub bounds: LogicalRect,
    /// This node's layout rectangle after the composed logical transform.
    pub transformed_bounds: LogicalRect,
    pub paint_bounds: Option<LogicalRect>,
    pub hit_bounds: Option<LogicalRect>,
    pub cursor_icon: Option<CursorIcon>,
    pub ordered_pointer_target: bool,
    pub semantic_bounds: Option<LogicalRect>,
    pub effective_clip: Option<LogicalRect>,
    pub current_damage_bounds: Option<LogicalRect>,
    /// Scroll geometry when this node was explicitly declared as a scroll viewport.
    pub scroll_metrics: Option<ScrollMetrics>,
    pub focus_scope: Option<WidgetId>,
    pub blocks_input: bool,
    /// True when this node is under a current-generation hidden declaration.
    pub effective_hidden: bool,
    /// True when this node is under a current-generation disabled declaration.
    pub effective_disabled: bool,
}

/// One backend-neutral paint operation in final paint order.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintRecord {
    pub id: WidgetId,
    pub primitive: PaintPrimitive,
    pub bounds: LogicalRect,
    pub effective_clip: Option<LogicalRect>,
}

/// One current-generation hit-test entry in final paint order.
#[derive(Clone, Debug, PartialEq)]
pub struct HitRecord {
    pub id: WidgetId,
    pub bounds: LogicalRect,
    pub effective_clip: Option<LogicalRect>,
    pub focus_scope: Option<WidgetId>,
    pub blocks_input: bool,
    pub cursor_icon: Option<CursorIcon>,
    pub ordered_pointer_target: bool,
}

/// One node in the serializable Esox-owned semantic tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: WidgetId,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub properties: SemanticProperties,
    pub bounds: LogicalRect,
    pub effective_clip: Option<LogicalRect>,
    pub focus_scope: Option<WidgetId>,
}

/// Backend-neutral semantic products committed atomically with paint and hits.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticSnapshot {
    pub roots: Vec<WidgetId>,
    pub nodes: Vec<SemanticNode>,
}

impl SemanticSnapshot {
    /// Find one semantic node by its window-scoped stable identity.
    pub fn node(&self, id: WidgetId) -> Option<&SemanticNode> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

/// Current bounds contributing damage for one painted node.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageRecord {
    pub id: WidgetId,
    pub current_bounds: LogicalRect,
    pub effective_clip: Option<LogicalRect>,
}

/// Focusable members owned by one blocking overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusScope {
    pub owner: WidgetId,
    pub members: Vec<WidgetId>,
}

/// Immutable products committed together for one frame generation.
#[derive(Clone, Debug, PartialEq)]
pub struct CommittedScene {
    pub generation: u64,
    pub viewport: LogicalSize,
    pub nodes: Vec<ResolvedNode>,
    pub display_list: Vec<PaintRecord>,
    pub hit_index: Vec<HitRecord>,
    pub semantics: SemanticSnapshot,
    pub damage: Vec<DamageRecord>,
    pub focus_order: Vec<WidgetId>,
    pub focus_scopes: Vec<FocusScope>,
}

impl CommittedScene {
    /// Find a resolved node by its window-scoped stable identity.
    pub fn node(&self, id: WidgetId) -> Option<&ResolvedNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// Hit test in reverse final paint order using current effective clips.
    pub fn hit_test(&self, point: LogicalPoint) -> Option<&ResolvedNode> {
        let hit = self.hit_index.iter().rev().find(|hit| {
            hit.bounds.contains(point) && hit.effective_clip.is_none_or(|clip| clip.contains(point))
        })?;
        self.node(hit.id)
    }

    /// Resolve the topmost clipped hit cursor, defaulting when no hint applies.
    pub fn cursor_icon_at(&self, point: LogicalPoint) -> CursorIcon {
        self.hit_index
            .iter()
            .rev()
            .find(|hit| {
                hit.bounds.contains(point)
                    && hit.effective_clip.is_none_or(|clip| clip.contains(point))
            })
            .and_then(|hit| hit.cursor_icon)
            .unwrap_or(CursorIcon::Default)
    }
}

/// Destination for an already committed, renderer-independent scene.
pub trait SceneConsumer {
    fn consume(&mut self, scene: &CommittedScene);
}

/// Headless consumer retaining structured submissions for assertions.
#[derive(Debug, Default)]
pub struct NullSceneConsumer {
    scenes: Vec<CommittedScene>,
}

impl NullSceneConsumer {
    /// Scenes observed after atomic commits.
    pub fn scenes(&self) -> &[CommittedScene] {
        &self.scenes
    }
}

impl SceneConsumer for NullSceneConsumer {
    fn consume(&mut self, scene: &CommittedScene) {
        self.scenes.push(scene.clone());
    }
}

/// A rejected grid declaration, reported before layout or commit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridDeclarationError {
    EmptyColumns,
    InvalidFixedTrack { index: usize, value: f32 },
    InvalidFractionTrack { index: usize, value: f32 },
    InvalidColumnGap(f32),
    InvalidRowGap(f32),
}

/// Invalid production table declarations rejected before any row is declared.
#[derive(Clone, Debug, PartialEq)]
pub enum TableDeclarationError {
    EmptyColumns,
    DuplicateColumnId(WidgetId),
    InvalidColumnWidth {
        column: WidgetId,
        value: f32,
    },
    InvalidColumnMinimum {
        column: WidgetId,
        value: f32,
    },
    InvalidColumnMaximum {
        column: WidgetId,
        value: f32,
    },
    InvalidColumnRange {
        column: WidgetId,
        min: f32,
        max: f32,
    },
    InvalidHeaderHeight(f32),
    InvalidResizeHandleWidth(f32),
    UnrepresentableAggregateWidth,
    InvalidRowCellCount {
        row: usize,
        expected: usize,
        actual: usize,
    },
}

/// Invalid renderer-neutral separator declaration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeparatorDeclarationError {
    InvalidThickness(f32),
}

/// Invalid determinate progress declarations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProgressDeclarationError {
    InvalidRange {
        minimum: f32,
        maximum: f32,
    },
    ValueOutOfRange {
        minimum: f32,
        maximum: f32,
        value: f32,
    },
    InvalidRadius(f32),
}

/// Invalid production checkbox declarations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CheckboxDeclarationError {
    InvalidIndicatorSize(f32),
    InvalidGap(f32),
    InvalidRadius(f32),
    RadiusExceedsIndicator { radius: f32, indicator_size: f32 },
}

/// Failure before a scene reaches the atomic commit point.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameError {
    InvalidVirtualContent {
        id: WidgetId,
        error: VirtualDeclarationError,
    },
    InvalidGrid {
        id: WidgetId,
        error: GridDeclarationError,
    },
    InvalidTable {
        id: WidgetId,
        error: TableDeclarationError,
    },
    InvalidSeparator {
        id: WidgetId,
        error: SeparatorDeclarationError,
    },
    InvalidProgress {
        id: WidgetId,
        error: ProgressDeclarationError,
    },
    InvalidCheckbox {
        id: WidgetId,
        error: CheckboxDeclarationError,
    },
    DuplicateWidgetId(WidgetId),
    InvalidTransform(WidgetId),
    InvalidFlexBasis {
        id: WidgetId,
        value: f32,
    },
    GenerationOwnerMismatch {
        attempt_owner: u64,
        current_owner: u64,
    },
    StaleGenerationAttempt {
        base_generation: u64,
        current_generation: u64,
    },
    GenerationViewportChanged {
        attempted: LogicalSize,
        current: LogicalSize,
    },
    GenerationStateChanged {
        attempted_revision: u64,
        current_revision: u64,
    },
    Layout(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FrameError {}

static NEXT_FRAME_CORE_OWNER_TOKEN: AtomicU64 = AtomicU64::new(1);

fn next_frame_core_owner_token() -> u64 {
    NEXT_FRAME_CORE_OWNER_TOKEN
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .expect("FrameCore owner token space exhausted")
}

/// Per-window owner of persistent state and the last committed scene.
#[derive(Debug)]
pub struct FrameCore {
    owner_token: u64,
    mutation_revision: u64,
    viewport: LogicalSize,
    generation: u64,
    widget_state: WidgetStateStore,
    committed: Option<CommittedScene>,
    scroll_offsets: HashMap<WidgetId, ScrollOffsetState>,
    pending_wheel_events: VecDeque<QueuedWheelEvent>,
    wheel_scroll_speed: f32,
    pending_pointer_events: VecDeque<QueuedPointerEvent>,
    pending_keyboard_events: VecDeque<QueuedKeyboardEvent>,
    pointer_captures: HashMap<u64, PointerCapture>,
    keyboard_focus: Option<WidgetId>,
    active_focus_scopes: Vec<WidgetId>,
    focus_restoration: HashMap<WidgetId, Option<WidgetId>>,
    cancellations: VecDeque<InputResponse>,
}

#[derive(Clone, Copy, Debug)]
struct QueuedWheelEvent {
    position: LogicalPoint,
    delta: LogicalPoint,
}

#[derive(Clone, Copy, Debug)]
struct QueuedPointerEvent {
    kind: PointerEventKind,
    pointer: u64,
    position: LogicalPoint,
}

#[derive(Clone, Debug)]
struct QueuedKeyboardEvent {
    event: esox_input::KeyEvent,
    modifiers: esox_input::Modifiers,
}

#[derive(Clone, Copy, Debug)]
struct PointerCapture {
    owner: WidgetId,
    focus_scope: Option<WidgetId>,
}

/// FrameCore-owned state that may change while one generation is attempted.
///
/// A candidate is cloned from the persistent owner, used for committed-scene
/// input dispatch, declaration, and interaction reconciliation, then installed
/// only when every fallible frame phase has succeeded.
#[derive(Clone, Debug)]
struct GenerationCandidate {
    widget_state: WidgetStateStore,
    scroll_offsets: HashMap<WidgetId, ScrollOffsetState>,
    pointer_captures: HashMap<u64, PointerCapture>,
    keyboard_focus: Option<WidgetId>,
    active_focus_scopes: Vec<WidgetId>,
    focus_restoration: HashMap<WidgetId, Option<WidgetId>>,
    cancellations: VecDeque<InputResponse>,
}

/// One detached in-progress FrameCore generation attempt.
///
/// Input has already been dispatched into the candidate widget state exposed
/// by [`Self::widget_state`], but no persistent FrameCore state changes until
/// [`FrameCore::finish_generation`] resolves and commits successfully. The
/// attempt owns no borrow of FrameCore, so application declaration can retain
/// access to the per-window owner. Dropping or explicitly aborting it discards
/// the candidate and leaves queued input available for deterministic retry.
pub struct GenerationAttempt {
    candidate: GenerationCandidate,
    owner_token: u64,
    base_generation: u64,
    mutation_revision: u64,
    viewport: LogicalSize,
    pointer_event_prefix: usize,
    keyboard_event_prefix: usize,
    wheel_event_prefix: usize,
}

impl GenerationAttempt {
    /// Candidate widget state used by the once-only declaration phase.
    pub fn widget_state(&mut self) -> &mut WidgetStateStore {
        &mut self.candidate.widget_state
    }

    /// Compute and retain a current-generation uniform virtual window.
    ///
    /// Queued wheel input has already been routed into this attempt's candidate
    /// scroll map. All mutations remain transactional until the attempt commits.
    pub fn plan_virtual_list(
        &mut self,
        spec: VirtualListSpec,
    ) -> Result<VirtualWindow, FrameError> {
        if !spec.item_height.is_finite() || spec.item_height <= 0.0 {
            return Err(FrameError::InvalidVirtualContent {
                id: spec.id,
                error: VirtualDeclarationError::InvalidItemHeight(spec.item_height),
            });
        }
        if !spec.viewport_height.is_finite() || spec.viewport_height <= 0.0 {
            return Err(FrameError::InvalidVirtualContent {
                id: spec.id,
                error: VirtualDeclarationError::InvalidViewportHeight(spec.viewport_height),
            });
        }
        let content_height64 = spec.item_count as f64 * f64::from(spec.item_height);
        if !content_height64.is_finite() || content_height64 > f64::from(f32::MAX) {
            return Err(FrameError::InvalidVirtualContent {
                id: spec.id,
                error: VirtualDeclarationError::UnrepresentableContentExtent {
                    item_count: spec.item_count,
                    item_height: spec.item_height,
                },
            });
        }
        if spec.item_count > 1 {
            let previous = ((spec.item_count - 2) as f64 * f64::from(spec.item_height)) as f32;
            let last = ((spec.item_count - 1) as f64 * f64::from(spec.item_height)) as f32;
            let bits = last.to_bits();
            let spacing_down = last - f32::from_bits(bits.saturating_sub(1));
            let spacing_up = f32::from_bits(bits.saturating_add(1)) - last;
            if previous == last || spacing_down.max(spacing_up) > spec.item_height {
                return Err(FrameError::InvalidVirtualContent {
                    id: spec.id,
                    error: VirtualDeclarationError::UnrepresentableItemPositions {
                        item_count: spec.item_count,
                        item_height: spec.item_height,
                    },
                });
            }
        }

        let content_height = content_height64 as f32;
        let maximum_offset64 = (content_height64 - f64::from(spec.viewport_height)).max(0.0);
        let maximum_offset = maximum_offset64 as f32;
        let last_bottom_is_finite = if spec.item_count == 0 {
            true
        } else {
            let last_top = ((spec.item_count - 1) as f64 * f64::from(spec.item_height)) as f32;
            (last_top + spec.item_height).is_finite()
        };
        let viewport_movement_is_distinct = maximum_offset64 == 0.0
            || (maximum_offset.is_finite()
                && f64::from(maximum_offset) < content_height64
                && maximum_offset < content_height
                && (maximum_offset + spec.viewport_height).is_finite());
        if !last_bottom_is_finite || !viewport_movement_is_distinct {
            return Err(FrameError::InvalidVirtualContent {
                id: spec.id,
                error: VirtualDeclarationError::UnrepresentableScrollGeometry {
                    item_count: spec.item_count,
                    item_height: spec.item_height,
                    viewport_height: spec.viewport_height,
                },
            });
        }
        let sanitize = |offset: f32| {
            if offset.is_nan() || offset == f32::NEG_INFINITY {
                0.0
            } else if offset == f32::INFINITY {
                maximum_offset
            } else {
                offset.clamp(0.0, maximum_offset)
            }
        };
        let retained = self
            .candidate
            .scroll_offsets
            .get(&spec.id)
            .copied()
            .unwrap_or_default();
        let mut requested_offset = spec.explicit_offset.unwrap_or(retained.requested.y);
        let mut applied_offset = sanitize(requested_offset);

        if spec.explicit_offset.is_none() {
            if let Some(target) = spec.scroll_to.filter(|_| spec.item_count > 0) {
                let target = target.min(spec.item_count - 1);
                let target_top = (target as f64 * f64::from(spec.item_height)) as f32;
                let target_bottom = ((target + 1) as f64 * f64::from(spec.item_height)) as f32;
                if target_top < applied_offset {
                    requested_offset = target_top;
                } else if target_bottom > applied_offset + spec.viewport_height {
                    requested_offset = target_bottom - spec.viewport_height;
                } else {
                    requested_offset = applied_offset;
                }
                applied_offset = sanitize(requested_offset);
            }
        }
        if spec.item_count == 0 {
            requested_offset = 0.0;
            applied_offset = 0.0;
        }

        self.candidate.scroll_offsets.insert(
            spec.id,
            ScrollOffsetState {
                requested: LogicalPoint::new(0.0, requested_offset),
                applied: LogicalPoint::new(0.0, applied_offset),
            },
        );

        let first = (f64::from(applied_offset) / f64::from(spec.item_height)).floor() as usize;
        let end = ((f64::from(applied_offset) + f64::from(spec.viewport_height))
            / f64::from(spec.item_height))
        .ceil()
        .min(spec.item_count as f64) as usize;
        Ok(VirtualWindow {
            visible_range: first.min(spec.item_count)..end.max(first.min(spec.item_count)),
            requested_offset,
            applied_offset,
            maximum_offset,
            content_height,
        })
    }

    /// Explicitly discard this generation without changing persistent state.
    pub fn abort(self) {}
}

impl FrameCore {
    /// Create an independent frame context for one logical viewport.
    pub fn new(viewport: LogicalSize) -> Self {
        Self::try_new(viewport).expect("FrameCore requires a finite, positive logical viewport")
    }

    /// Create an independent frame context after validating its viewport.
    pub fn try_new(viewport: LogicalSize) -> Option<Self> {
        if !viewport.is_valid() {
            return None;
        }
        Some(Self {
            owner_token: next_frame_core_owner_token(),
            mutation_revision: 0,
            viewport,
            generation: 0,
            widget_state: WidgetStateStore::default(),
            committed: None,
            scroll_offsets: HashMap::new(),
            pending_wheel_events: VecDeque::new(),
            wheel_scroll_speed: 40.0,
            pending_pointer_events: VecDeque::new(),
            pending_keyboard_events: VecDeque::new(),
            pointer_captures: HashMap::new(),
            keyboard_focus: None,
            active_focus_scopes: Vec::new(),
            focus_restoration: HashMap::new(),
            cancellations: VecDeque::new(),
        })
    }

    /// Change the viewport used by the very next frame when it is valid.
    pub fn resize(&mut self, viewport: LogicalSize) -> bool {
        if !viewport.is_valid() {
            return false;
        }
        self.viewport = viewport;
        self.mutation_revision = self
            .mutation_revision
            .checked_add(1)
            .expect("FrameCore mutation revision exhausted");
        true
    }

    /// Apply a renderer-neutral viewport produced by the platform boundary.
    pub fn resize_logical_viewport(&mut self, viewport: esox_input::LogicalViewport) -> bool {
        self.resize(LogicalSize::new(viewport.width, viewport.height))
    }

    /// The immutable scene used for committed-scene input and damage comparison.
    pub fn committed_scene(&self) -> Option<&CommittedScene> {
        self.committed.as_ref()
    }

    /// The last successfully reconciled scroll state for a stable widget ID.
    pub fn scroll_offset(&self, id: WidgetId) -> Option<ScrollOffsetState> {
        self.scroll_offsets.get(&id).copied()
    }

    /// Set the logical movement applied to each normalized wheel-delta unit.
    ///
    /// The default is `40.0`, preserving the legacy production scroll speed.
    pub fn set_wheel_scroll_speed(&mut self, speed: f32) {
        if speed.is_finite() && speed >= 0.0 {
            self.wheel_scroll_speed = speed;
            self.mutation_revision = self
                .mutation_revision
                .checked_add(1)
                .expect("FrameCore mutation revision exhausted");
        }
    }

    /// Queue a normalized two-axis wheel event at a committed-scene position.
    ///
    /// Positive components increase the corresponding stored offset. Wheel
    /// routing is independent of pointer capture and happens against the last
    /// immutable committed scene before the next declaration.
    pub fn queue_wheel(&mut self, position: LogicalPoint, delta: LogicalPoint) -> bool {
        if !position.is_finite() || !delta.is_finite() || (delta.x == 0.0 && delta.y == 0.0) {
            return false;
        }
        self.pending_wheel_events
            .push_back(QueuedWheelEvent { position, delta });
        true
    }

    /// Queue a renderer-neutral wheel event produced by the platform adapter.
    pub fn queue_wheel_event(&mut self, event: esox_input::WheelEvent) -> bool {
        self.queue_wheel(
            LogicalPoint::new(event.position.x, event.position.y),
            LogicalPoint::new(event.delta.x, event.delta.y),
        )
    }

    /// Queue a synthetic pointer press for dispatch before the next declaration.
    pub fn queue_pointer_press(&mut self, position: LogicalPoint) {
        let _ = self.queue_pointer_event(PointerEventKind::Press, 0, position);
    }

    /// Queue a pointer event with an explicit pointer identity.
    pub fn queue_pointer_event(
        &mut self,
        kind: PointerEventKind,
        pointer: u64,
        position: LogicalPoint,
    ) -> bool {
        if !position.is_finite() {
            return false;
        }
        self.pending_pointer_events.push_back(QueuedPointerEvent {
            kind,
            pointer,
            position,
        });
        true
    }

    /// Queue a renderer-neutral logical pointer event from the platform boundary.
    pub fn queue_pointer_input(&mut self, pointer: u64, event: esox_input::PointerEvent) -> bool {
        let kind = match event.phase {
            esox_input::PointerPhase::Move => PointerEventKind::Move,
            esox_input::PointerPhase::Press { .. } => PointerEventKind::Press,
            esox_input::PointerPhase::Release { .. } => PointerEventKind::Release,
        };
        self.queue_pointer_event(
            kind,
            pointer,
            LogicalPoint::new(event.position.x, event.position.y),
        )
    }

    /// Queue a platform-independent keyboard event for the committed keyboard focus.
    pub fn queue_keyboard_input(
        &mut self,
        event: esox_input::KeyEvent,
        modifiers: esox_input::Modifiers,
    ) {
        self.pending_keyboard_events
            .push_back(QueuedKeyboardEvent { event, modifiers });
    }

    /// The current capture owner for a pointer in this window context.
    pub fn pointer_capture(&self, pointer: u64) -> Option<WidgetId> {
        self.pointer_captures
            .get(&pointer)
            .map(|capture| capture.owner)
    }

    /// Cursor for a pointer, honoring explicit capture before position lookup.
    pub fn cursor_icon_at(&self, pointer: u64, position: LogicalPoint) -> CursorIcon {
        let captured = self
            .pointer_captures
            .get(&pointer)
            .map(|capture| capture.owner);
        captured
            .and_then(|owner| self.committed.as_ref()?.node(owner)?.cursor_icon)
            .or_else(|| {
                self.committed
                    .as_ref()
                    .map(|scene| scene.cursor_icon_at(position))
            })
            .unwrap_or(CursorIcon::Default)
    }

    /// The current keyboard focus in this window context.
    pub fn keyboard_focus(&self) -> Option<WidgetId> {
        self.keyboard_focus
    }

    /// Consume a framework-synthesized pointer cancellation at most once.
    pub fn take_cancellation(&mut self) -> Option<InputResponse> {
        let cancellation = self.cancellations.pop_front();
        if cancellation.is_some() {
            self.mutation_revision = self
                .mutation_revision
                .checked_add(1)
                .expect("FrameCore mutation revision exhausted");
        }
        cancellation
    }

    /// Begin a candidate generation and dispatch queued committed-scene input.
    ///
    /// The returned attempt is detached from this per-window owner. Declaration
    /// may mutate only its cloned candidate widget state while retaining access
    /// to other window state. A later successful [`Self::finish_generation`]
    /// installs all candidate state atomically; abort, drop, stale finish,
    /// viewport-changed finish, and resolution failure install nothing.
    pub fn begin_generation(&self) -> GenerationAttempt {
        let mut candidate = self.generation_candidate();
        candidate.widget_state.active_pointer_captures = candidate
            .pointer_captures
            .iter()
            .map(|(pointer, capture)| (*pointer, capture.owner))
            .collect();
        self.dispatch_pending_input(&mut candidate);
        self.dispatch_pending_keyboard(&mut candidate);
        self.dispatch_pending_wheels(&mut candidate.scroll_offsets);
        GenerationAttempt {
            candidate,
            owner_token: self.owner_token,
            base_generation: self.generation,
            mutation_revision: self.mutation_revision,
            viewport: self.viewport,
            pointer_event_prefix: self.pending_pointer_events.len(),
            keyboard_event_prefix: self.pending_keyboard_events.len(),
            wheel_event_prefix: self.pending_wheel_events.len(),
        }
    }

    /// Declare exactly once, resolve from current inputs, then commit atomically.
    pub fn run_frame<M, C, F>(
        &mut self,
        measurer: &M,
        consumer: &mut C,
        declare: F,
    ) -> Result<&CommittedScene, FrameError>
    where
        M: IntrinsicMeasurer,
        C: SceneConsumer,
        F: FnOnce(&mut WidgetStateStore) -> Element,
    {
        let mut attempt = self.begin_generation();
        let root = declare(attempt.widget_state());
        self.finish_generation(attempt, root, measurer, consumer)
    }

    /// Resolve and atomically commit a detached generation attempt.
    pub fn finish_generation<'a, M, C>(
        &'a mut self,
        attempt: GenerationAttempt,
        mut root: Element,
        measurer: &M,
        consumer: &mut C,
    ) -> Result<&'a CommittedScene, FrameError>
    where
        M: IntrinsicMeasurer,
        C: SceneConsumer,
    {
        if attempt.owner_token != self.owner_token {
            return Err(FrameError::GenerationOwnerMismatch {
                attempt_owner: attempt.owner_token,
                current_owner: self.owner_token,
            });
        }
        if attempt.base_generation != self.generation {
            return Err(FrameError::StaleGenerationAttempt {
                base_generation: attempt.base_generation,
                current_generation: self.generation,
            });
        }
        if attempt.viewport != self.viewport {
            return Err(FrameError::GenerationViewportChanged {
                attempted: attempt.viewport,
                current: self.viewport,
            });
        }
        if attempt.mutation_revision != self.mutation_revision {
            return Err(FrameError::GenerationStateChanged {
                attempted_revision: attempt.mutation_revision,
                current_revision: self.mutation_revision,
            });
        }
        let GenerationAttempt {
            mut candidate,
            pointer_event_prefix,
            keyboard_event_prefix,
            wheel_event_prefix,
            ..
        } = attempt;
        root.apply_retained_scroll_offsets(&candidate.scroll_offsets);
        let mut resolved = resolve(&root, self.viewport, measurer)?;
        Self::expand_damage(&mut resolved.damage, self.committed.as_ref());
        let all_focus_order: Vec<_> = resolved.hit_index.iter().map(|hit| hit.id).collect();
        let mut focus_scopes: Vec<FocusScope> = Vec::new();
        for node in &resolved.nodes {
            if node.blocks_input {
                focus_scopes.push(FocusScope {
                    owner: node.id,
                    members: Vec::new(),
                });
            }
            if let Some(scope) = node.focus_scope {
                if node.hit_bounds.is_some() {
                    if let Some(record) =
                        focus_scopes.iter_mut().find(|record| record.owner == scope)
                    {
                        record.members.push(node.id);
                    }
                }
            }
        }
        let focus_order = focus_scopes
            .last()
            .map_or(all_focus_order, |scope| scope.members.clone());
        let scene = CommittedScene {
            generation: self.generation + 1,
            viewport: self.viewport,
            nodes: resolved.nodes,
            display_list: resolved.display_list,
            hit_index: resolved.hit_index,
            semantics: resolved.semantics,
            damage: resolved.damage,
            focus_order,
            focus_scopes,
        };

        candidate.scroll_offsets =
            Self::reconcile_scroll_offsets(&scene, &candidate.scroll_offsets);
        Self::reconcile_interaction(
            &mut candidate,
            &scene,
            self.committed.as_ref(),
            self.generation,
        );

        self.generation = scene.generation;
        self.widget_state = candidate.widget_state;
        self.scroll_offsets = candidate.scroll_offsets;
        self.pointer_captures = candidate.pointer_captures;
        self.keyboard_focus = candidate.keyboard_focus;
        self.active_focus_scopes = candidate.active_focus_scopes;
        self.focus_restoration = candidate.focus_restoration;
        self.cancellations = candidate.cancellations;
        self.pending_wheel_events
            .drain(..wheel_event_prefix.min(self.pending_wheel_events.len()));
        self.pending_pointer_events
            .drain(..pointer_event_prefix.min(self.pending_pointer_events.len()));
        self.pending_keyboard_events
            .drain(..keyboard_event_prefix.min(self.pending_keyboard_events.len()));
        self.committed = Some(scene);
        let committed = self.committed.as_ref().expect("scene was just committed");
        consumer.consume(committed);
        Ok(committed)
    }

    fn generation_candidate(&self) -> GenerationCandidate {
        GenerationCandidate {
            widget_state: self.widget_state.clone(),
            scroll_offsets: self.scroll_offsets.clone(),
            pointer_captures: self.pointer_captures.clone(),
            keyboard_focus: self.keyboard_focus,
            active_focus_scopes: self.active_focus_scopes.clone(),
            focus_restoration: self.focus_restoration.clone(),
            cancellations: self.cancellations.clone(),
        }
    }

    fn dispatch_pending_wheels(
        &self,
        candidate_scroll_offsets: &mut HashMap<WidgetId, ScrollOffsetState>,
    ) {
        let Some(scene) = self.committed.as_ref() else {
            return;
        };
        for event in &self.pending_wheel_events {
            if !event.position.x.is_finite()
                || !event.position.y.is_finite()
                || !event.delta.x.is_finite()
                || !event.delta.y.is_finite()
            {
                continue;
            }
            let scaled_delta = LogicalPoint::new(
                event.delta.x * self.wheel_scroll_speed,
                event.delta.y * self.wheel_scroll_speed,
            );
            if !scaled_delta.x.is_finite() || !scaled_delta.y.is_finite() {
                continue;
            }
            Self::route_wheel(
                scene,
                event.position,
                scaled_delta,
                candidate_scroll_offsets,
            );
        }
    }

    fn route_wheel(
        scene: &CommittedScene,
        position: LogicalPoint,
        delta: LogicalPoint,
        offsets: &mut HashMap<WidgetId, ScrollOffsetState>,
    ) {
        // Resolved nodes are stored in structural paint order. Start at the
        // topmost eligible structural node, then bubble only through its
        // committed ancestry. In particular, an overlapping later sibling
        // (including a blocking overlay) prevents routing into obscured
        // scrollable siblings behind it.
        let mut route = scene
            .nodes
            .iter()
            .rev()
            .find(|node| {
                !node.effective_hidden
                    && !node.effective_disabled
                    && node.transformed_bounds.contains(position)
                    && node
                        .effective_clip
                        .is_none_or(|clip| clip.contains(position))
            })
            .map(|node| node.id);
        let mut route_x = delta.x != 0.0;
        let mut route_y = delta.y != 0.0;
        let mut deferred_x = None;
        let mut deferred_y = None;
        while let Some(id) = route.filter(|_| route_x || route_y) {
            let node = scene
                .node(id)
                .expect("wheel route contains only committed nodes");
            if let Some(metrics) = node.scroll_metrics {
                let state = offsets.entry(id).or_insert(ScrollOffsetState {
                    requested: metrics.requested_offset,
                    applied: metrics.applied_offset,
                });
                let previous = state.applied;
                let accumulate = |applied: f32, requested: f32, delta: f32| {
                    let same_direction_overshoot = (delta > 0.0 && requested > applied)
                        || (delta < 0.0 && requested < applied);
                    let base = if same_direction_overshoot {
                        requested
                    } else {
                        applied
                    };
                    base + delta
                };
                let raw_requested = LogicalPoint::new(
                    if route_x {
                        accumulate(previous.x, state.requested.x, delta.x)
                    } else {
                        state.requested.x
                    },
                    if route_y {
                        accumulate(previous.y, state.requested.y, delta.y)
                    } else {
                        state.requested.y
                    },
                );
                let next = LogicalPoint::new(
                    if route_x {
                        (previous.x + delta.x).clamp(0.0, metrics.maximum_offset.x)
                    } else {
                        previous.x
                    },
                    if route_y {
                        (previous.y + delta.y).clamp(0.0, metrics.maximum_offset.y)
                    } else {
                        previous.y
                    },
                );
                if route_x && next.x != previous.x {
                    state.requested.x = raw_requested.x;
                    route_x = false;
                } else if route_x && deferred_x.is_none() {
                    deferred_x = Some((id, raw_requested.x));
                }
                if route_y && next.y != previous.y {
                    state.requested.y = raw_requested.y;
                    route_y = false;
                } else if route_y && deferred_y.is_none() {
                    deferred_y = Some((id, raw_requested.y));
                }
                // Each axis is consumed by the deepest viewport that changes
                // on that axis, even when clamping applies only part of the
                // delta. Residual delta is intentionally not propagated.
                state.applied = next;
            }
            route = node.parent;
        }
        // If no viewport on the committed ancestry could move, preserve the
        // raw intent on the deepest eligible viewport. A same-generation
        // content growth can then apply it without changing ancestor routing.
        if route_x {
            if let Some((id, requested)) = deferred_x {
                if let Some(state) = offsets.get_mut(&id) {
                    state.requested.x = requested;
                }
            }
        }
        if route_y {
            if let Some((id, requested)) = deferred_y {
                if let Some(state) = offsets.get_mut(&id) {
                    state.requested.y = requested;
                }
            }
        }
    }

    fn reconcile_scroll_offsets(
        scene: &CommittedScene,
        previous: &HashMap<WidgetId, ScrollOffsetState>,
    ) -> HashMap<WidgetId, ScrollOffsetState> {
        scene
            .nodes
            .iter()
            .filter_map(|node| {
                node.scroll_metrics.map(|metrics| {
                    // A still-declared hidden viewport has no useful resolved
                    // extent. Preserve its stable-ID state until it
                    // participates again; removal still drops the entry.
                    let state = if node.effective_hidden {
                        previous.get(&node.id).copied().unwrap_or_default()
                    } else {
                        ScrollOffsetState {
                            requested: metrics.requested_offset,
                            applied: metrics.applied_offset,
                        }
                    };
                    (node.id, state)
                })
            })
            .collect()
    }

    fn expand_damage(damage: &mut Vec<DamageRecord>, committed: Option<&CommittedScene>) {
        let Some(committed) = committed else {
            return;
        };
        for previous in &committed.display_list {
            let unchanged = damage.iter().any(|current| {
                current.id == previous.id
                    && current.current_bounds == previous.bounds
                    && current.effective_clip == previous.effective_clip
            });
            if !unchanged {
                damage.push(DamageRecord {
                    id: previous.id,
                    current_bounds: previous.bounds,
                    effective_clip: previous.effective_clip,
                });
            }
        }
    }

    fn dispatch_pending_input(&self, candidate: &mut GenerationCandidate) {
        let mut ordered_targets = HashMap::<u64, WidgetId>::new();
        for (dispatch_ordinal, event) in self.pending_pointer_events.iter().enumerate() {
            let Some(scene) = self.committed.as_ref() else {
                continue;
            };
            let target = candidate
                .pointer_captures
                .get(&event.pointer)
                .and_then(|capture| scene.node(capture.owner))
                .or_else(|| {
                    ordered_targets
                        .get(&event.pointer)
                        .and_then(|owner| scene.node(*owner))
                })
                .or_else(|| scene.hit_test(event.position));
            let Some(target) = target else {
                continue;
            };
            if event.kind == PointerEventKind::Press && target.ordered_pointer_target {
                ordered_targets.insert(event.pointer, target.id);
            }
            let response = InputResponse {
                kind: event.kind,
                pointer: event.pointer,
                target: target.id,
                committed_generation: scene.generation,
                dispatch_ordinal: dispatch_ordinal as u64,
                virtual_owner: target.virtual_owner,
                interaction_owner: target.interaction_owner,
                position: event.position,
                target_bounds: target
                    .hit_bounds
                    .expect("a hit-tested node always has hit bounds"),
                target_parent_bounds: target
                    .parent
                    .and_then(|parent| scene.node(parent))
                    .map(|parent| parent.transformed_bounds),
            };
            candidate
                .widget_state
                .responses
                .entry(target.id)
                .or_default()
                .push_back(response);
            if matches!(
                event.kind,
                PointerEventKind::Release | PointerEventKind::Cancel
            ) {
                candidate.pointer_captures.remove(&event.pointer);
                ordered_targets.remove(&event.pointer);
            }
        }
    }

    fn dispatch_pending_keyboard(&self, candidate: &mut GenerationCandidate) {
        let Some(scene) = self.committed.as_ref() else {
            return;
        };
        let Some(target) = self
            .keyboard_focus
            .filter(|focused| scene.focus_order.contains(focused))
        else {
            return;
        };
        let responses = candidate
            .widget_state
            .keyboard_responses
            .entry(target)
            .or_default();
        for (dispatch_ordinal, queued) in self.pending_keyboard_events.iter().enumerate() {
            responses.push_back(KeyboardInputResponse {
                event: queued.event.clone(),
                modifiers: queued.modifiers,
                target,
                committed_generation: scene.generation,
                dispatch_ordinal: dispatch_ordinal as u64,
            });
        }
    }

    fn reconcile_interaction(
        candidate: &mut GenerationCandidate,
        scene: &CommittedScene,
        committed: Option<&CommittedScene>,
        committed_generation: u64,
    ) {
        let focusable = |id| scene.node(id).is_some_and(|node| node.hit_bounds.is_some());

        let removed_captures: Vec<_> = candidate
            .pointer_captures
            .iter()
            .filter(|(_, capture)| {
                scene.node(capture.owner).is_none_or(|node| {
                    node.hit_bounds.is_none() || node.focus_scope != capture.focus_scope
                })
            })
            .map(|(pointer, capture)| (*pointer, capture.owner))
            .collect();
        for (pointer, owner) in removed_captures {
            candidate.pointer_captures.remove(&pointer);
            if let Some(old_node) = committed.and_then(|old| old.node(owner)) {
                candidate.cancellations.push_back(InputResponse {
                    kind: PointerEventKind::Cancel,
                    pointer,
                    target: owner,
                    committed_generation,
                    dispatch_ordinal: u64::MAX,
                    virtual_owner: old_node.virtual_owner,
                    interaction_owner: old_node.interaction_owner,
                    position: LogicalPoint::new(
                        old_node.hit_bounds.unwrap_or(old_node.transformed_bounds).x,
                        old_node.hit_bounds.unwrap_or(old_node.transformed_bounds).y,
                    ),
                    target_bounds: old_node.hit_bounds.unwrap_or(old_node.transformed_bounds),
                    target_parent_bounds: old_node
                        .parent
                        .and_then(|parent| committed.and_then(|scene| scene.node(parent)))
                        .map(|parent| parent.transformed_bounds),
                });
            }
        }
        for pointer in candidate.widget_state.requested_pointer_releases.drain() {
            candidate.pointer_captures.remove(&pointer);
        }
        for (pointer, owner) in candidate.widget_state.requested_pointer_captures.drain() {
            if let Some(node) = scene.node(owner).filter(|node| node.hit_bounds.is_some()) {
                candidate.pointer_captures.insert(
                    pointer,
                    PointerCapture {
                        owner,
                        focus_scope: node.focus_scope,
                    },
                );
            }
        }

        if let Some(requested) = candidate.widget_state.requested_keyboard_focus.take() {
            if focusable(requested) {
                candidate.keyboard_focus = Some(requested);
            }
        }

        let current_scopes: Vec<_> = scene.focus_scopes.iter().map(|scope| scope.owner).collect();
        let closed_scopes: Vec<_> = candidate
            .active_focus_scopes
            .iter()
            .rev()
            .filter(|owner| !current_scopes.contains(owner))
            .copied()
            .collect();
        let mut restoration = None;
        for owner in closed_scopes {
            if let Some(target) = candidate.focus_restoration.remove(&owner).flatten() {
                restoration = Some(target);
            }
        }

        for scope in &scene.focus_scopes {
            if !candidate.active_focus_scopes.contains(&scope.owner) {
                candidate
                    .focus_restoration
                    .insert(scope.owner, candidate.keyboard_focus);
            }
        }

        if let Some(scope) = scene.focus_scopes.last() {
            if !candidate
                .keyboard_focus
                .is_some_and(|focused| scope.members.contains(&focused))
            {
                candidate.keyboard_focus = scope.members.first().copied();
            }
        } else if let Some(target) = restoration.filter(|target| focusable(*target)) {
            candidate.keyboard_focus = Some(target);
        } else if !candidate.keyboard_focus.is_some_and(focusable) {
            candidate.keyboard_focus = scene.focus_order.first().copied();
        }
        candidate.active_focus_scopes = current_scopes;
        candidate
            .widget_state
            .keyboard_responses
            .retain(|id, _| scene.focus_order.contains(id));
        candidate.widget_state.responses.retain(|id, responses| {
            let current_target = scene.node(*id).is_some_and(|node| {
                !node.effective_hidden && !node.effective_disabled && node.hit_bounds.is_some()
            });
            if current_target {
                return true;
            }
            // A same-batch wheel may move a committed virtual descendant out
            // of the declaration range after pointer dispatch. Retain only
            // responses from that immediately preceding generation, and only
            // while its virtual owner remains an active scroll viewport. A
            // later generation drops an unconsumed response rather than
            // creating immortal stale input.
            let retained_virtual_response = committed
                .and_then(|old| old.node(*id))
                .and_then(|old_node| old_node.virtual_owner)
                .and_then(|owner| scene.node(owner))
                .is_some_and(|owner| {
                    !owner.effective_hidden
                        && !owner.effective_disabled
                        && owner.scroll_metrics.is_some()
                        && responses.iter().all(|response| {
                            response.committed_generation.checked_add(1) == Some(scene.generation)
                        })
                });
            retained_virtual_response
        });
    }
}

#[derive(Debug, Default)]
struct ResolvedProducts {
    nodes: Vec<ResolvedNode>,
    display_list: Vec<PaintRecord>,
    hit_index: Vec<HitRecord>,
    semantics: SemanticSnapshot,
    damage: Vec<DamageRecord>,
}

#[derive(Clone, Copy, Debug)]
struct TraversalContext {
    origin: (f32, f32),
    clip: Option<LogicalRect>,
    transform: ResolvedTransform,
    parent: Option<WidgetId>,
    virtual_owner: Option<WidgetId>,
    interaction_owner: Option<WidgetId>,
    focus_scope: Option<WidgetId>,
    semantic_parent: Option<WidgetId>,
    hidden: bool,
    disabled: bool,
}

fn resolve(
    root: &Element,
    viewport: LogicalSize,
    measurer: &impl IntrinsicMeasurer,
) -> Result<ResolvedProducts, FrameError> {
    fn build(
        element: &Element,
        root: bool,
        viewport: LogicalSize,
        tree: &mut TaffyTree<MeasureContext>,
        ids: &mut HashMap<WidgetId, NodeId>,
        live_ids: &mut HashSet<WidgetId>,
    ) -> Result<NodeId, FrameError> {
        if !live_ids.insert(element.id) {
            return Err(FrameError::DuplicateWidgetId(element.id));
        }
        if element
            .flex_basis
            .is_some_and(|basis| !basis.is_finite() || basis < 0.0)
        {
            return Err(FrameError::InvalidFlexBasis {
                id: element.id,
                value: element.flex_basis.expect("invalid basis was present"),
            });
        }
        if element
            .virtual_content_height
            .is_some_and(|height| !height.is_finite() || height < 0.0)
        {
            return Err(FrameError::InvalidVirtualContent {
                id: element.id,
                error: VirtualDeclarationError::InvalidContentHeight(
                    element
                        .virtual_content_height
                        .expect("invalid virtual content height was present"),
                ),
            });
        }

        let children = element
            .children
            .iter()
            .map(|child| build(child, false, viewport, tree, ids, live_ids))
            .collect::<Result<Vec<_>, _>>()?;

        let mut style = match &element.kind {
            ElementKind::Flex { axis, gap } => Style {
                display: Display::Flex,
                flex_direction: match axis {
                    Axis::Row => FlexDirection::Row,
                    Axis::Column => FlexDirection::Column,
                },
                gap: taffy::geometry::Size {
                    width: length(*gap),
                    height: length(*gap),
                },
                ..Default::default()
            },
            ElementKind::Grid {
                columns,
                column_gap,
                row_gap,
            } => {
                if columns.is_empty() {
                    return Err(FrameError::InvalidGrid {
                        id: element.id,
                        error: GridDeclarationError::EmptyColumns,
                    });
                }
                for (index, track) in columns.iter().enumerate() {
                    let error = match track {
                        GridTrack::Fixed(value) if !value.is_finite() || *value < 0.0 => {
                            Some(GridDeclarationError::InvalidFixedTrack {
                                index,
                                value: *value,
                            })
                        }
                        GridTrack::Fraction(value) if !value.is_finite() || *value <= 0.0 => {
                            Some(GridDeclarationError::InvalidFractionTrack {
                                index,
                                value: *value,
                            })
                        }
                        _ => None,
                    };
                    if let Some(error) = error {
                        return Err(FrameError::InvalidGrid {
                            id: element.id,
                            error,
                        });
                    }
                }
                if !column_gap.is_finite() || *column_gap < 0.0 {
                    return Err(FrameError::InvalidGrid {
                        id: element.id,
                        error: GridDeclarationError::InvalidColumnGap(*column_gap),
                    });
                }
                if !row_gap.is_finite() || *row_gap < 0.0 {
                    return Err(FrameError::InvalidGrid {
                        id: element.id,
                        error: GridDeclarationError::InvalidRowGap(*row_gap),
                    });
                }
                Style {
                    display: Display::Grid,
                    grid_template_columns: columns
                        .iter()
                        .map(|track| match track {
                            GridTrack::Fixed(value) => length(*value),
                            GridTrack::Fraction(value) => fr(*value),
                            GridTrack::Auto => auto(),
                        })
                        .collect(),
                    grid_template_rows: vec![auto()],
                    gap: taffy::geometry::Size {
                        width: length(*column_gap),
                        height: length(*row_gap),
                    },
                    ..Default::default()
                }
            }
            ElementKind::Fixed(size) => Style {
                size: Size {
                    width: length(size.width),
                    height: length(size.height),
                },
                flex_shrink: 0.0,
                ..Default::default()
            },
            ElementKind::Text { .. } | ElementKind::Image(_) => Style {
                min_size: Size {
                    width: length(0.0),
                    height: auto(),
                },
                ..Default::default()
            },
        };
        style.flex_grow = element.flex_grow;
        if let Some(shrink) = element.flex_shrink {
            style.flex_shrink = shrink;
        }
        if let Some(basis) = element.flex_basis {
            style.flex_basis = length(basis);
        }
        style.justify_content = Some(match element.main_axis_alignment {
            MainAxisAlignment::Start => JustifyContent::START,
            MainAxisAlignment::Center => JustifyContent::CENTER,
            MainAxisAlignment::End => JustifyContent::END,
            MainAxisAlignment::SpaceBetween => JustifyContent::SPACE_BETWEEN,
            MainAxisAlignment::SpaceAround => JustifyContent::SPACE_AROUND,
            MainAxisAlignment::SpaceEvenly => JustifyContent::SPACE_EVENLY,
        });
        match &element.kind {
            ElementKind::Grid { .. } => {
                style.align_content = Some(match element.cross_axis_alignment {
                    CrossAxisAlignment::Stretch => AlignContent::STRETCH,
                    CrossAxisAlignment::Start => AlignContent::START,
                    CrossAxisAlignment::Center => AlignContent::CENTER,
                    CrossAxisAlignment::End => AlignContent::END,
                });
            }
            _ => {
                style.align_items = Some(match element.cross_axis_alignment {
                    CrossAxisAlignment::Stretch => AlignItems::STRETCH,
                    CrossAxisAlignment::Start => AlignItems::START,
                    CrossAxisAlignment::Center => AlignItems::CENTER,
                    CrossAxisAlignment::End => AlignItems::END,
                });
            }
        }
        let dimension = |value: Option<f32>| value.map_or_else(auto, length);
        style.padding = Rect {
            left: length(element.padding),
            right: length(element.padding),
            top: length(element.padding),
            bottom: length(element.padding),
        };
        if element.size.width.is_some() {
            style.size.width = dimension(element.size.width);
        }
        if element.size.height.is_some() {
            style.size.height = dimension(element.size.height);
        }
        if element.min_size.width.is_some() {
            style.min_size.width = dimension(element.min_size.width);
        }
        if element.min_size.height.is_some() {
            style.min_size.height = dimension(element.min_size.height);
        }
        if element.max_size.width.is_some() {
            style.max_size.width = dimension(element.max_size.width);
        }
        if element.max_size.height.is_some() {
            style.max_size.height = dimension(element.max_size.height);
        }
        if let Some(position) = element.absolute_position {
            style.position = Position::Absolute;
            style.inset = Rect {
                left: length(position.x),
                right: auto(),
                top: length(position.y),
                bottom: auto(),
            };
        }
        if element.flex_grow > 0.0 {
            style.min_size.width = length(0.0);
            style.min_size.height = length(0.0);
        }
        if root {
            style.size = Size {
                width: length(viewport.width),
                height: length(viewport.height),
            };
        }
        if element.hidden {
            style.display = Display::None;
        }
        if element.clips_children {
            style.overflow.x = Overflow::Clip;
            style.overflow.y = Overflow::Clip;
        }

        let node = if children.is_empty() {
            match &element.kind {
                ElementKind::Text {
                    content,
                    properties,
                } => tree
                    .new_leaf_with_context(
                        style,
                        MeasureContext::Text {
                            content: content.clone(),
                            properties: properties.clone(),
                        },
                    )
                    .map_err(|error| FrameError::Layout(error.to_string()))?,
                ElementKind::Image(image) => tree
                    .new_leaf_with_context(style, MeasureContext::Image(*image))
                    .map_err(|error| FrameError::Layout(error.to_string()))?,
                _ => tree
                    .new_leaf(style)
                    .map_err(|error| FrameError::Layout(error.to_string()))?,
            }
        } else {
            tree.new_with_children(style, &children)
                .map_err(|error| FrameError::Layout(error.to_string()))?
        };
        ids.insert(element.id, node);
        Ok(node)
    }

    let mut tree = TaffyTree::new();
    let mut ids = HashMap::new();
    let root_node = build(
        root,
        true,
        viewport,
        &mut tree,
        &mut ids,
        &mut HashSet::new(),
    )?;
    tree.compute_layout_with_measure(
        root_node,
        Size::MAX_CONTENT,
        |known, available, _, context, _| {
            let known_dimensions = KnownDimensions {
                width: known.width,
                height: known.height,
            };
            let map_available = |length| match length {
                AvailableSpace::Definite(value) => AvailableLength::Definite(value),
                AvailableSpace::MinContent => AvailableLength::MinContent,
                AvailableSpace::MaxContent => AvailableLength::MaxContent,
            };
            let available_space = AvailableSize {
                width: map_available(available.width),
                height: map_available(available.height),
            };
            let measured = match context {
                Some(MeasureContext::Text {
                    content,
                    properties,
                }) => measurer.measure_text(TextMeasureRequest {
                    content,
                    properties,
                    known_dimensions,
                    available_space,
                }),
                Some(MeasureContext::Image(resource)) => {
                    measurer.measure_image(ImageMeasureRequest {
                        resource: *resource,
                        known_dimensions,
                        available_space,
                    })
                }
                None => LogicalSize::default(),
            };
            Size {
                width: known.width.unwrap_or(measured.width),
                height: known.height.unwrap_or(measured.height),
            }
        },
    )
    .map_err(|error| FrameError::Layout(error.to_string()))?;

    fn collect(
        element: &Element,
        tree: &TaffyTree<MeasureContext>,
        ids: &HashMap<WidgetId, NodeId>,
        context: TraversalContext,
        output: &mut ResolvedProducts,
    ) -> Result<(), FrameError> {
        let layout = tree.unrounded_layout(ids[&element.id]);
        let effective_hidden = context.hidden || element.hidden;
        let effective_disabled = context.disabled || element.disabled;
        let bounds = LogicalRect {
            x: context.origin.0 + layout.location.x,
            y: context.origin.1 + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };
        let local_transform = ResolvedTransform::local(element.transform, bounds)
            .ok_or(FrameError::InvalidTransform(element.id))?;
        let transform = context
            .transform
            .compose(local_transform)
            .ok_or(FrameError::InvalidTransform(element.id))?;
        let transformed_bounds = transform
            .rect(bounds)
            .ok_or(FrameError::InvalidTransform(element.id))?;
        let scroll_metrics = element.requested_scroll_offset.map(|requested_offset| {
            let viewport_extent = LogicalSize::new(layout.size.width, layout.size.height);
            let content_extent = LogicalSize::new(
                layout.content_size.width.max(viewport_extent.width),
                layout
                    .content_size
                    .height
                    .max(element.virtual_content_height.unwrap_or(0.0))
                    .max(viewport_extent.height),
            );
            let maximum_offset = LogicalPoint::new(
                (content_extent.width - viewport_extent.width).max(0.0),
                (content_extent.height - viewport_extent.height).max(0.0),
            );
            let clamp_offset = |requested: f32, maximum: f32| {
                if requested.is_nan() || requested == f32::NEG_INFINITY {
                    0.0
                } else if requested == f32::INFINITY {
                    maximum
                } else {
                    requested.clamp(0.0, maximum)
                }
            };
            ScrollMetrics {
                viewport_extent,
                content_extent,
                requested_offset,
                applied_offset: LogicalPoint::new(
                    clamp_offset(requested_offset.x, maximum_offset.x),
                    clamp_offset(requested_offset.y, maximum_offset.y),
                ),
                maximum_offset,
            }
        });
        let participates_in_interaction = !effective_hidden && !effective_disabled;
        let effective_clip = (!effective_hidden).then_some(context.clip).flatten();
        let focus_scope = (element.blocking_overlay && participates_in_interaction)
            .then_some(element.id)
            .or(context.focus_scope);
        let paint_bounds =
            (!effective_hidden && element.paint.is_some()).then_some(transformed_bounds);
        let hit_bounds =
            (element.interactive && participates_in_interaction).then_some(transformed_bounds);
        let cursor_icon = hit_bounds.and(element.cursor_icon);
        let semantic_bounds = (!effective_hidden)
            .then_some(())
            .and(element.semantics.as_ref())
            .map(|_| transformed_bounds);
        let current_damage_bounds =
            (!effective_hidden && element.paint.is_some()).then_some(transformed_bounds);
        output.nodes.push(ResolvedNode {
            id: element.id,
            parent: context.parent,
            virtual_owner: context.virtual_owner,
            interaction_owner: element.interaction_owner.or(context.interaction_owner),
            bounds,
            transformed_bounds,
            paint_bounds,
            hit_bounds,
            cursor_icon,
            ordered_pointer_target: element.ordered_pointer_target,
            semantic_bounds,
            effective_clip,
            current_damage_bounds,
            scroll_metrics,
            focus_scope,
            blocks_input: element.blocking_overlay && participates_in_interaction,
            effective_hidden,
            effective_disabled,
        });

        if !effective_hidden {
            if let Some(primitive) = &element.paint {
                output.display_list.push(PaintRecord {
                    id: element.id,
                    primitive: primitive.clone(),
                    bounds: transformed_bounds,
                    effective_clip: context.clip,
                });
                output.damage.push(DamageRecord {
                    id: element.id,
                    current_bounds: transformed_bounds,
                    effective_clip: context.clip,
                });
            }
        }
        if element.interactive && participates_in_interaction {
            output.hit_index.push(HitRecord {
                id: element.id,
                bounds: transformed_bounds,
                effective_clip: context.clip,
                focus_scope,
                blocks_input: element.blocking_overlay,
                cursor_icon: element.cursor_icon,
                ordered_pointer_target: element.ordered_pointer_target,
            });
        }

        let semantic_parent = if !effective_hidden {
            if let Some(properties) = &element.semantics {
                if let Some(parent) = context.semantic_parent {
                    output
                        .semantics
                        .nodes
                        .iter_mut()
                        .find(|node| node.id == parent)
                        .expect("semantic parent was resolved before its child")
                        .children
                        .push(element.id);
                } else {
                    output.semantics.roots.push(element.id);
                }
                let mut properties = properties.clone();
                properties.disabled |= effective_disabled;
                output.semantics.nodes.push(SemanticNode {
                    id: element.id,
                    parent: context.semantic_parent,
                    children: Vec::new(),
                    properties,
                    bounds: transformed_bounds,
                    effective_clip: context.clip,
                    focus_scope,
                });
                Some(element.id)
            } else {
                context.semantic_parent
            }
        } else {
            context.semantic_parent
        };

        let child_clip = if element.clips_children {
            context
                .clip
                .and_then(|clip| clip.intersection(transformed_bounds))
        } else {
            context.clip
        };
        let applied_scroll_offset = scroll_metrics
            .map(|metrics| metrics.applied_offset)
            .unwrap_or_default();
        let child_origin = (
            bounds.x - applied_scroll_offset.x,
            bounds.y - applied_scroll_offset.y,
        );
        let virtual_owner = element
            .virtual_content_height
            .map(|_| element.id)
            .or(context.virtual_owner);
        let interaction_owner = element.interaction_owner.or(context.interaction_owner);
        for child in &element.children {
            collect(
                child,
                tree,
                ids,
                TraversalContext {
                    origin: child_origin,
                    clip: child_clip,
                    transform,
                    parent: Some(element.id),
                    virtual_owner,
                    interaction_owner,
                    focus_scope,
                    semantic_parent,
                    hidden: effective_hidden,
                    disabled: effective_disabled,
                },
                output,
            )?;
        }
        Ok(())
    }

    let viewport_rect = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: viewport.width,
        height: viewport.height,
    };
    let mut output = ResolvedProducts::default();
    collect(
        root,
        &tree,
        &ids,
        TraversalContext {
            origin: (0.0, 0.0),
            clip: Some(viewport_rect),
            transform: ResolvedTransform::IDENTITY,
            parent: None,
            virtual_owner: None,
            interaction_owner: None,
            focus_scope: None,
            semantic_parent: None,
            hidden: false,
            disabled: false,
        },
        &mut output,
    )?;
    Ok(output)
}
