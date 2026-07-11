//! GPU- and window-independent current-frame scene construction.
//!
//! This is the narrow frame core used to establish the ADR-001 lifecycle
//! contracts while the production widget API is migrated incrementally.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use taffy::prelude::{
    AlignContent, AlignItems, AvailableSpace, Display, FlexDirection, JustifyContent, NodeId,
    Position, Rect, Size, Style, TaffyMaxContent, TaffyTree,
};
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
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
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
}

/// Serializable semantic properties declared without resolved geometry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticProperties {
    pub role: SemanticRole,
    pub label: Option<String>,
    pub disabled: bool,
}

impl SemanticProperties {
    pub fn new(role: SemanticRole) -> Self {
        Self {
            role,
            label: None,
            disabled: false,
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
}

/// One node in the lightweight tree owned only for the current frame.
#[derive(Clone, Debug)]
pub struct Element {
    id: WidgetId,
    kind: ElementKind,
    children: Vec<Self>,
    flex_grow: f32,
    padding: f32,
    size: Size<Option<f32>>,
    min_size: Size<Option<f32>>,
    max_size: Size<Option<f32>>,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    paint: Option<PaintPrimitive>,
    interactive: bool,
    semantics: Option<SemanticProperties>,
    clips_children: bool,
    scroll_offset: LogicalPoint,
    absolute_position: Option<LogicalPoint>,
    blocking_overlay: bool,
    hidden: bool,
    disabled: bool,
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
            padding: 0.0,
            size: Size::NONE,
            min_size: Size::NONE,
            max_size: Size::NONE,
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            paint: Some(paint),
            interactive: false,
            semantics: None,
            clips_children: false,
            scroll_offset: LogicalPoint::default(),
            absolute_position: None,
            blocking_overlay: false,
            hidden: false,
            disabled: false,
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

    /// Translate descendants by the current frame's scroll offset.
    pub fn with_scroll_offset(mut self, x: f32, y: f32) -> Self {
        self.scroll_offset = LogicalPoint::new(x, y);
        self
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
}

/// One pointer response routed through an immutable committed scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputResponse {
    pub kind: PointerEventKind,
    pub pointer: u64,
    pub target: WidgetId,
    pub committed_generation: u64,
    pub position: LogicalPoint,
    pub target_bounds: LogicalRect,
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
#[derive(Debug, Default)]
pub struct WidgetStateStore {
    values: HashMap<WidgetId, u64>,
    responses: HashMap<WidgetId, VecDeque<InputResponse>>,
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

/// All scene products for one resolved node, sourced by one traversal.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedNode {
    pub id: WidgetId,
    pub bounds: LogicalRect,
    pub paint_bounds: Option<LogicalRect>,
    pub hit_bounds: Option<LogicalRect>,
    pub semantic_bounds: Option<LogicalRect>,
    pub effective_clip: Option<LogicalRect>,
    pub current_damage_bounds: Option<LogicalRect>,
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

/// Failure before a scene reaches the atomic commit point.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameError {
    InvalidGrid {
        id: WidgetId,
        error: GridDeclarationError,
    },
    DuplicateWidgetId(WidgetId),
    Layout(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FrameError {}

/// Per-window owner of persistent state and the last committed scene.
#[derive(Debug)]
pub struct FrameCore {
    viewport: LogicalSize,
    generation: u64,
    widget_state: WidgetStateStore,
    committed: Option<CommittedScene>,
    pending_pointer_events: VecDeque<QueuedPointerEvent>,
    pointer_captures: HashMap<u64, PointerCapture>,
    keyboard_focus: Option<WidgetId>,
    active_focus_scopes: Vec<WidgetId>,
    focus_restoration: HashMap<WidgetId, Option<WidgetId>>,
    cancellations: VecDeque<InputResponse>,
}

#[derive(Clone, Copy, Debug)]
struct QueuedPointerEvent {
    kind: PointerEventKind,
    pointer: u64,
    position: LogicalPoint,
}

#[derive(Clone, Copy, Debug)]
struct PointerCapture {
    owner: WidgetId,
    focus_scope: Option<WidgetId>,
}

impl FrameCore {
    /// Create an independent frame context for one logical viewport.
    pub fn new(viewport: LogicalSize) -> Self {
        Self {
            viewport,
            generation: 0,
            widget_state: WidgetStateStore::default(),
            committed: None,
            pending_pointer_events: VecDeque::new(),
            pointer_captures: HashMap::new(),
            keyboard_focus: None,
            active_focus_scopes: Vec::new(),
            focus_restoration: HashMap::new(),
            cancellations: VecDeque::new(),
        }
    }

    /// Change the viewport used by the very next frame.
    pub fn resize(&mut self, viewport: LogicalSize) {
        self.viewport = viewport;
    }

    /// The immutable scene used for committed-scene input and damage comparison.
    pub fn committed_scene(&self) -> Option<&CommittedScene> {
        self.committed.as_ref()
    }

    /// Queue a synthetic pointer press for dispatch before the next declaration.
    pub fn queue_pointer_press(&mut self, position: LogicalPoint) {
        self.queue_pointer_event(PointerEventKind::Press, 0, position);
    }

    /// Queue a pointer event with an explicit pointer identity.
    pub fn queue_pointer_event(
        &mut self,
        kind: PointerEventKind,
        pointer: u64,
        position: LogicalPoint,
    ) {
        self.pending_pointer_events.push_back(QueuedPointerEvent {
            kind,
            pointer,
            position,
        });
    }

    /// The current capture owner for a pointer in this window context.
    pub fn pointer_capture(&self, pointer: u64) -> Option<WidgetId> {
        self.pointer_captures
            .get(&pointer)
            .map(|capture| capture.owner)
    }

    /// The current keyboard focus in this window context.
    pub fn keyboard_focus(&self) -> Option<WidgetId> {
        self.keyboard_focus
    }

    /// Consume a framework-synthesized pointer cancellation at most once.
    pub fn take_cancellation(&mut self) -> Option<InputResponse> {
        self.cancellations.pop_front()
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
        self.dispatch_pending_input();
        let root = declare(&mut self.widget_state);
        let resolved = resolve(&root, self.viewport, measurer)?;
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

        self.reconcile_interaction(&scene);

        self.generation = scene.generation;
        self.committed = Some(scene);
        let committed = self.committed.as_ref().expect("scene was just committed");
        consumer.consume(committed);
        Ok(committed)
    }

    fn dispatch_pending_input(&mut self) {
        while let Some(event) = self.pending_pointer_events.pop_front() {
            let Some(scene) = self.committed.as_ref() else {
                continue;
            };
            let target = self
                .pointer_captures
                .get(&event.pointer)
                .and_then(|capture| scene.node(capture.owner))
                .or_else(|| scene.hit_test(event.position));
            let Some(target) = target else {
                continue;
            };
            let response = InputResponse {
                kind: event.kind,
                pointer: event.pointer,
                target: target.id,
                committed_generation: scene.generation,
                position: event.position,
                target_bounds: target
                    .hit_bounds
                    .expect("a hit-tested node always has hit bounds"),
            };
            self.widget_state
                .responses
                .entry(target.id)
                .or_default()
                .push_back(response);
            if matches!(
                event.kind,
                PointerEventKind::Release | PointerEventKind::Cancel
            ) {
                self.pointer_captures.remove(&event.pointer);
            }
        }
    }

    fn reconcile_interaction(&mut self, scene: &CommittedScene) {
        let focusable = |id| scene.node(id).is_some_and(|node| node.hit_bounds.is_some());

        let removed_captures: Vec<_> = self
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
            self.pointer_captures.remove(&pointer);
            if let Some(old_node) = self.committed.as_ref().and_then(|old| old.node(owner)) {
                self.cancellations.push_back(InputResponse {
                    kind: PointerEventKind::Cancel,
                    pointer,
                    target: owner,
                    committed_generation: self.generation,
                    position: LogicalPoint::new(old_node.bounds.x, old_node.bounds.y),
                    target_bounds: old_node.bounds,
                });
            }
        }

        for pointer in self.widget_state.requested_pointer_releases.drain() {
            self.pointer_captures.remove(&pointer);
        }
        for (pointer, owner) in self.widget_state.requested_pointer_captures.drain() {
            if let Some(node) = scene.node(owner).filter(|node| node.hit_bounds.is_some()) {
                self.pointer_captures.insert(
                    pointer,
                    PointerCapture {
                        owner,
                        focus_scope: node.focus_scope,
                    },
                );
            }
        }

        if let Some(requested) = self.widget_state.requested_keyboard_focus.take() {
            if focusable(requested) {
                self.keyboard_focus = Some(requested);
            }
        }

        let current_scopes: Vec<_> = scene.focus_scopes.iter().map(|scope| scope.owner).collect();
        let closed_scopes: Vec<_> = self
            .active_focus_scopes
            .iter()
            .rev()
            .filter(|owner| !current_scopes.contains(owner))
            .copied()
            .collect();
        let mut restoration = None;
        for owner in closed_scopes {
            if let Some(target) = self.focus_restoration.remove(&owner).flatten() {
                restoration = Some(target);
            }
        }

        for scope in &scene.focus_scopes {
            if !self.active_focus_scopes.contains(&scope.owner) {
                self.focus_restoration
                    .insert(scope.owner, self.keyboard_focus);
            }
        }

        if let Some(scope) = scene.focus_scopes.last() {
            if !self
                .keyboard_focus
                .is_some_and(|focused| scope.members.contains(&focused))
            {
                self.keyboard_focus = scope.members.first().copied();
            }
        } else if let Some(target) = restoration.filter(|target| focusable(*target)) {
            self.keyboard_focus = Some(target);
        } else if !self.keyboard_focus.is_some_and(focusable) {
            self.keyboard_focus = scene.focus_order.first().copied();
        }
        self.active_focus_scopes = current_scopes;
        self.widget_state.responses.retain(|id, _| {
            scene.node(*id).is_some_and(|node| {
                !node.effective_hidden && !node.effective_disabled && node.hit_bounds.is_some()
            })
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
    ) {
        let layout = tree.unrounded_layout(ids[&element.id]);
        let effective_hidden = context.hidden || element.hidden;
        let effective_disabled = context.disabled || element.disabled;
        let bounds = LogicalRect {
            x: context.origin.0 + layout.location.x,
            y: context.origin.1 + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };
        let participates_in_interaction = !effective_hidden && !effective_disabled;
        let effective_clip = (!effective_hidden).then_some(context.clip).flatten();
        let focus_scope = (element.blocking_overlay && participates_in_interaction)
            .then_some(element.id)
            .or(context.focus_scope);
        let paint_bounds = (!effective_hidden && element.paint.is_some()).then_some(bounds);
        let hit_bounds = (element.interactive && participates_in_interaction).then_some(bounds);
        let semantic_bounds = (!effective_hidden)
            .then_some(())
            .and(element.semantics.as_ref())
            .map(|_| bounds);
        let current_damage_bounds =
            (!effective_hidden && element.paint.is_some()).then_some(bounds);
        output.nodes.push(ResolvedNode {
            id: element.id,
            bounds,
            paint_bounds,
            hit_bounds,
            semantic_bounds,
            effective_clip,
            current_damage_bounds,
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
                    bounds,
                    effective_clip: context.clip,
                });
                output.damage.push(DamageRecord {
                    id: element.id,
                    current_bounds: bounds,
                    effective_clip: context.clip,
                });
            }
        }
        if element.interactive && participates_in_interaction {
            output.hit_index.push(HitRecord {
                id: element.id,
                bounds,
                effective_clip: context.clip,
                focus_scope,
                blocks_input: element.blocking_overlay,
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
                    bounds,
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
            context.clip.and_then(|clip| clip.intersection(bounds))
        } else {
            context.clip
        };
        let child_origin = (
            bounds.x - element.scroll_offset.x,
            bounds.y - element.scroll_offset.y,
        );
        for child in &element.children {
            collect(
                child,
                tree,
                ids,
                TraversalContext {
                    origin: child_origin,
                    clip: child_clip,
                    focus_scope,
                    semantic_parent,
                    hidden: effective_hidden,
                    disabled: effective_disabled,
                },
                output,
            );
        }
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
            focus_scope: None,
            semantic_parent: None,
            hidden: false,
            disabled: false,
        },
        &mut output,
    );
    Ok(output)
}
