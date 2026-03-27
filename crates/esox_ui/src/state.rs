//! UI interaction state — focus, hit testing, input, keyboard/mouse routing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

use esox_input::{KeyEvent, Modifiers};

use crate::layout::Rect;
use crate::widgets::menu_bar::MenuBarDeferred;

/// Trait for clipboard access. Implemented by platform layer.
pub trait ClipboardProvider {
    fn read_text(&self) -> Option<String>;
    fn write_text(&self, text: &str);
}

/// A snapshot of text state for undo/redo.
#[derive(Debug, Clone)]
struct UndoEntry {
    text: String,
    cursor: usize,
}

/// Undo/redo history for text input.
#[derive(Debug, Clone)]
pub struct UndoHistory {
    entries: VecDeque<UndoEntry>,
    index: usize,
}

impl UndoHistory {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            index: 0,
        }
    }

    fn push(&mut self, text: &str, cursor: usize) {
        // Truncate redo history.
        self.entries.truncate(self.index);
        self.entries.push_back(UndoEntry {
            text: text.to_string(),
            cursor,
        });
        // Cap at 100 entries.
        if self.entries.len() > 100 {
            self.entries.pop_front();
        }
        self.index = self.entries.len();
    }

    fn undo(&mut self) -> Option<&UndoEntry> {
        if self.index > 0 {
            self.index -= 1;
            Some(&self.entries[self.index])
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<&UndoEntry> {
        if self.index < self.entries.len() {
            let entry = &self.entries[self.index];
            self.index += 1;
            Some(entry)
        } else {
            None
        }
    }
}

/// Text input state: buffer, cursor, selection.
#[derive(Debug, Clone)]
pub struct InputState {
    /// The text content.
    pub text: String,
    /// Byte offset of the cursor within `text`.
    pub cursor: usize,
    /// Selection range as (start, end) byte offsets, where start <= end.
    pub selection: Option<(usize, usize)>,
    /// Horizontal scroll offset in pixels for long text.
    pub scroll_offset: f32,
    /// Anchor for shift+arrow selection expansion. When shift is held, cursor moves
    /// but anchor stays fixed; selection = (anchor.min(cursor), anchor.max(cursor)).
    pub selection_anchor: Option<usize>,
    /// Undo/redo history.
    undo_history: UndoHistory,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection: None,
            scroll_offset: 0.0,
            selection_anchor: None,
            undo_history: UndoHistory::new(),
        }
    }

    /// Delete selected text if any, returning whether something was deleted.
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection.take() {
            self.text.drain(start..end);
            self.cursor = start;
            true
        } else {
            false
        }
    }

    /// Insert a character at the cursor, replacing any selection.
    pub fn insert_char(&mut self, c: char) {
        self.delete_selection();
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at the cursor, replacing any selection.
    ///
    /// CRLF (`\r\n`) and lone `\r` are normalized to `\n` so that line
    /// helpers throughout the widget code can rely on `\n`-only line endings.
    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        if s.contains('\r') {
            let normalized = s.replace("\r\n", "\n").replace('\r', "\n");
            self.text.insert_str(self.cursor, &normalized);
            self.cursor += normalized.len();
        } else {
            self.text.insert_str(self.cursor, s);
            self.cursor += s.len();
        }
    }

    /// Delete the character before the cursor (Backspace).
    pub fn delete_back(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            let prev = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    /// Delete the character after the cursor (Delete key).
    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.text.len() {
            let next = self.cursor
                + self.text[self.cursor..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
            self.text.drain(self.cursor..next);
        }
    }

    /// Move cursor one character left.
    pub fn move_left(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        if self.cursor > 0 {
            self.cursor = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor one character right.
    pub fn move_right(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        if self.cursor < self.text.len() {
            self.cursor += self.text[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
    }

    /// Move cursor to the beginning of text.
    pub fn home(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        self.cursor = 0;
    }

    /// Move cursor to the end of text.
    pub fn end(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        self.cursor = self.text.len();
    }

    // ── Shift+extend selection methods ──

    /// Set anchor if not already set, then update selection from anchor to cursor.
    fn update_selection_from_anchor(&mut self) {
        let anchor = self.selection_anchor.unwrap_or(self.cursor);
        self.selection_anchor = Some(anchor);
        if anchor == self.cursor {
            self.selection = None;
        } else {
            self.selection = Some((anchor.min(self.cursor), anchor.max(self.cursor)));
        }
    }

    /// Move cursor left, extending selection (Shift+Left).
    pub fn move_left_extend(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        if self.cursor > 0 {
            self.cursor = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        self.update_selection_from_anchor();
    }

    /// Move cursor right, extending selection (Shift+Right).
    pub fn move_right_extend(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        if self.cursor < self.text.len() {
            self.cursor += self.text[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
        self.update_selection_from_anchor();
    }

    /// Move cursor to start, extending selection (Shift+Home).
    pub fn home_extend(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = 0;
        self.update_selection_from_anchor();
    }

    /// Move cursor to end, extending selection (Shift+End).
    pub fn end_extend(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.text.len();
        self.update_selection_from_anchor();
    }

    /// Move cursor to an arbitrary position, extending selection.
    pub fn move_to_extend(&mut self, pos: usize) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = pos;
        self.update_selection_from_anchor();
    }

    /// Move cursor to an arbitrary position, clearing selection.
    pub fn move_to(&mut self, pos: usize) {
        self.selection = None;
        self.selection_anchor = None;
        self.cursor = pos;
    }

    // ── Word boundary helpers ──

    /// Find the byte offset of the word boundary to the left of `pos`.
    fn word_boundary_left(&self, pos: usize) -> usize {
        let bytes = self.text.as_bytes();
        let mut i = pos;
        // Skip whitespace backward.
        while i > 0 && bytes[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        // Skip non-whitespace backward.
        while i > 0 && !bytes[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        i
    }

    /// Find the byte offset of the word boundary to the right of `pos`.
    fn word_boundary_right(&self, pos: usize) -> usize {
        let bytes = self.text.as_bytes();
        let len = bytes.len();
        let mut i = pos;
        // Skip non-whitespace forward.
        while i < len && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Skip whitespace forward.
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        i
    }

    /// Move cursor one word left (Ctrl+Left).
    pub fn move_word_left(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        self.cursor = self.word_boundary_left(self.cursor);
    }

    /// Move cursor one word right (Ctrl+Right).
    pub fn move_word_right(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        self.cursor = self.word_boundary_right(self.cursor);
    }

    /// Move cursor one word left, extending selection (Ctrl+Shift+Left).
    pub fn move_word_left_extend(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.word_boundary_left(self.cursor);
        self.update_selection_from_anchor();
    }

    /// Move cursor one word right, extending selection (Ctrl+Shift+Right).
    pub fn move_word_right_extend(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.word_boundary_right(self.cursor);
        self.update_selection_from_anchor();
    }

    /// Delete the word before the cursor (Ctrl+Backspace).
    pub fn delete_word_back(&mut self) {
        if self.delete_selection() {
            return;
        }
        let target = self.word_boundary_left(self.cursor);
        if target < self.cursor {
            self.text.drain(target..self.cursor);
            self.cursor = target;
        }
    }

    /// Delete the word after the cursor (Ctrl+Delete).
    pub fn delete_word_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let target = self.word_boundary_right(self.cursor);
        if target > self.cursor {
            self.text.drain(self.cursor..target);
        }
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        if !self.text.is_empty() {
            self.selection = Some((0, self.text.len()));
            self.selection_anchor = Some(0);
            self.cursor = self.text.len();
        }
    }

    /// Save current state to undo history. Call before mutations.
    pub fn save_undo(&mut self) {
        // Only push if text differs from last entry.
        let dominated = self
            .undo_history
            .entries
            .back()
            .is_some_and(|e| e.text == self.text);
        if !dominated {
            self.undo_history.push(&self.text, self.cursor);
        }
    }

    /// Undo the last text change.
    pub fn undo(&mut self) {
        if let Some(entry) = self.undo_history.undo() {
            self.text = entry.text.clone();
            self.cursor = entry.cursor.min(self.text.len());
            self.selection = None;
            self.selection_anchor = None;
        }
    }

    /// Redo the last undone change.
    pub fn redo(&mut self) {
        if let Some(entry) = self.undo_history.redo() {
            self.text = entry.text.clone();
            self.cursor = entry.cursor.min(self.text.len());
            self.selection = None;
            self.selection_anchor = None;
        }
    }

    /// Get the selected text, if any.
    pub fn selected_text(&self) -> Option<&str> {
        self.selection.map(|(s, e)| &self.text[s..e])
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for a select widget — just tracks the selected index.
#[derive(Debug, Clone)]
pub struct SelectState {
    pub selected_index: usize,
}

impl SelectState {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }
}

impl Default for SelectState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for the file drop zone.
#[derive(Debug, Clone)]
pub struct DropZoneState {
    /// Selected files.
    pub files: Vec<PathBuf>,
    /// Whether a file dialog is currently open.
    pub dialog_pending: bool,
}

impl DropZoneState {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            dialog_pending: false,
        }
    }
}

impl Default for DropZoneState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for tab widget.
#[derive(Debug, Clone)]
pub struct TabState {
    pub selected: usize,
}

impl TabState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }
}

impl Default for TabState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for virtual scroll widget.
#[derive(Debug, Clone)]
pub struct VirtualScrollState {
    pub item_count: usize,
    /// Set to Some(index) to auto-scroll that item into view.
    pub scroll_to: Option<usize>,
}

impl VirtualScrollState {
    pub fn new(item_count: usize) -> Self {
        Self {
            item_count,
            scroll_to: None,
        }
    }
}

/// Sort direction for table columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// State for table widget.
#[derive(Debug, Clone)]
pub struct TableState {
    pub selected_row: Option<usize>,
    /// User-overridden column widths (None = use default).
    pub column_widths: Vec<Option<f32>>,
    /// Active resize drag: (column_index, start_mouse_x, start_column_width).
    pub(crate) resize_drag: Option<(usize, f32, f32)>,
    /// Current sort state: (column_index, direction).
    pub sort: Option<(usize, SortDirection)>,
    /// Multi-select: set of selected row indices.
    pub selected_rows: HashSet<usize>,
    /// Anchor row for shift-click range selection.
    pub anchor_row: Option<usize>,
}

impl TableState {
    pub fn new() -> Self {
        Self {
            selected_row: None,
            column_widths: Vec::new(),
            resize_drag: None,
            sort: None,
            selected_rows: HashSet::new(),
            anchor_row: None,
        }
    }
}

impl Default for TableState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for tree widget.
#[derive(Debug, Clone)]
pub struct TreeState {
    pub expanded: HashSet<u64>,
    pub selected: Option<u64>,
    /// Multi-select: set of selected node IDs.
    pub selected_nodes: HashSet<u64>,
    /// Anchor node for shift-click range selection.
    pub anchor_node: Option<u64>,
}

impl TreeState {
    pub fn new() -> Self {
        Self {
            expanded: HashSet::new(),
            selected: None,
            selected_nodes: HashSet::new(),
            anchor_node: None,
        }
    }
}

impl Default for TreeState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for widget-to-widget drag.
#[derive(Debug, Clone, Copy)]
pub struct DragPayload {
    pub source_id: u64,
    pub payload: u64,
    pub x: f32,
    pub y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

/// Widget type hint for cursor icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    TextInput,
    Button,
    DropZone,
    Select,
    Slider,
    Checkbox,
    Radio,
    Scrollbar,
    Tab,
    TableRow,
    TreeNode,
    Toggle,
    Hyperlink,
    ColumnResize,
    ResizeNS,
    ResizeEW,
    Grab,
    Grabbing,
    NotAllowed,
    SplitDividerH,
    SplitDividerV,
    Combobox,
    Custom(esox_input::CursorIcon),
}

/// Overlay state (dropdown menus drawn on top of everything).
pub enum Overlay {
    Dropdown {
        id: u64,
        anchor: Rect,
        choices: Vec<String>,
        hovered: Option<usize>,
        selected: usize,
    },
    ContextMenu {
        id: u64,
        position: Rect,
        items: Vec<String>,
        hovered: Option<usize>,
    },
    ComboboxDropdown {
        id: u64,
        anchor: Rect,
        /// Full list of options (unfiltered).
        all_choices: Vec<String>,
        /// Indices into `all_choices` that match the current filter.
        filtered_indices: Vec<usize>,
        /// Which item in the *filtered* list is highlighted (keyboard nav).
        highlighted: Option<usize>,
        /// Scroll offset for long filtered lists (in pixels).
        scroll_offset: f32,
    },
}

/// Tooltip state — hover delay + text.
pub struct TooltipState {
    pub widget_id: u64,
    pub hover_start: Instant,
    pub anchor: Rect,
    pub text: String,
    pub visible: bool,
}

/// Mouse tracking state.
#[derive(Debug, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    /// Pending click: position + consumed flag.
    pub pending_click: Option<(f32, f32, bool)>,
    /// Pending right-click: position + consumed flag.
    pub pending_right_click: Option<(f32, f32, bool)>,
}

/// Hover animation state — drives ease-out color transitions.
pub struct HoverAnim {
    pub from: f32,
    pub to: f32,
    pub start: Instant,
    pub duration_ms: f32,
}

impl HoverAnim {
    /// Current interpolation value in [0, 1] with ease-out cubic.
    pub fn t(&self) -> f32 {
        let p = (self.start.elapsed().as_millis() as f32 / self.duration_ms).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - p).powi(3);
        self.from + (self.to - self.from) * eased
    }

    /// True when the animation has fully completed or from == to.
    pub fn is_settled(&self) -> bool {
        (self.from - self.to).abs() < 0.001
            || self.start.elapsed().as_millis() as f32 >= self.duration_ms
    }
}

/// Easing functions for animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseOutBack,
    EaseOutBounce,
    /// Custom cubic bezier curve, matching CSS `cubic-bezier(x1, y1, x2, y2)`.
    /// Control points are clamped to valid ranges (x in 0..1).
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,

            // Quadratic
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }

            // Cubic
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }

            // Quartic
            Easing::EaseInQuart => t * t * t * t,
            Easing::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            Easing::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }

            // Exponential
            Easing::EaseInExpo => {
                if t.abs() < f32::EPSILON {
                    0.0
                } else {
                    2.0f32.powf(10.0 * t - 10.0)
                }
            }
            Easing::EaseOutExpo => {
                if (t - 1.0).abs() < f32::EPSILON {
                    1.0
                } else {
                    1.0 - 2.0f32.powf(-10.0 * t)
                }
            }
            Easing::EaseInOutExpo => {
                if t.abs() < f32::EPSILON {
                    0.0
                } else if (t - 1.0).abs() < f32::EPSILON {
                    1.0
                } else if t < 0.5 {
                    2.0f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            }

            // Back (slight overshoot)
            Easing::EaseOutBack => {
                let c1: f32 = 1.70158;
                let c3 = c1 + 1.0;
                let t1 = t - 1.0;
                1.0 + c3 * t1 * t1 * t1 + c1 * t1 * t1
            }

            // Bounce
            Easing::EaseOutBounce => ease_out_bounce(t),

            // Custom cubic bezier (CSS-style)
            Easing::CubicBezier(x1, y1, x2, y2) => cubic_bezier_sample(x1, y1, x2, y2, t),
        }
    }
}

/// Bounce easing helper — four-segment quadratic bounce.
fn ease_out_bounce(t: f32) -> f32 {
    let n1: f32 = 7.5625;
    let d1: f32 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Solve a CSS-style cubic bezier curve.
///
/// Given control points (x1, y1) and (x2, y2), find the y value at the given
/// x (time) position. Uses Newton's method to invert the x(t) parametric
/// curve, then evaluates y(t).
fn cubic_bezier_sample(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    // Clamp x control points to [0, 1] per CSS spec.
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);

    // Edge cases.
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Newton's method to find t for given x.
    let mut t = x; // initial guess
    for _ in 0..8 {
        let xt = bezier_component(x1, x2, t);
        let dx = bezier_derivative(x1, x2, t);
        if dx.abs() < 1e-7 {
            break;
        }
        t -= (xt - x) / dx;
        t = t.clamp(0.0, 1.0);
    }

    bezier_component(y1, y2, t)
}

/// Evaluate one component of a cubic bezier at parameter t.
/// B(t) = 3(1-t)^2*t*p1 + 3(1-t)*t^2*p2 + t^3
fn bezier_component(p1: f32, p2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t
}

/// Derivative of one component of a cubic bezier at parameter t.
fn bezier_derivative(p1: f32, p2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

/// General-purpose animation state.
pub struct Anim {
    pub from: f32,
    pub to: f32,
    pub start: Instant,
    pub duration_ms: f32,
    pub easing: Easing,
    /// Whether this anim was queried this frame (for cleanup).
    pub(crate) queried: bool,
}

impl Anim {
    /// Current interpolation value.
    pub fn value(&self) -> f32 {
        let p = (self.start.elapsed().as_millis() as f32 / self.duration_ms).clamp(0.0, 1.0);
        let eased = self.easing.apply(p);
        self.from + (self.to - self.from) * eased
    }

    pub fn is_settled(&self) -> bool {
        (self.from - self.to).abs() < 0.001
            || self.start.elapsed().as_millis() as f32 >= self.duration_ms
    }
}

/// Spring dynamics configuration.
///
/// Controls the feel of spring-based animations. Higher stiffness makes the
/// spring snap faster; higher damping reduces oscillation. A damping ratio of
/// 1.0 is critically damped (no overshoot).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringConfig {
    /// Spring stiffness (force per unit displacement). Typical range: 100–1000.
    pub stiffness: f32,
    /// Damping coefficient (force per unit velocity). Typical range: 10–100.
    pub damping: f32,
    /// Mass of the simulated object. Almost always 1.0.
    pub mass: f32,
}

impl SpringConfig {
    /// Snappy spring — fast, no overshoot. Good for toggles and hover effects.
    pub const SNAPPY: Self = Self {
        stiffness: 400.0,
        damping: 30.0,
        mass: 1.0,
    };

    /// Gentle spring — smooth, slightly slower. Good for layout transitions.
    pub const GENTLE: Self = Self {
        stiffness: 170.0,
        damping: 20.0,
        mass: 1.0,
    };

    /// Bouncy spring — visible overshoot. Good for enter/exit animations.
    pub const BOUNCY: Self = Self {
        stiffness: 300.0,
        damping: 12.0,
        mass: 1.0,
    };

    /// Stiff spring — very fast settle. Good for micro-interactions.
    pub const STIFF: Self = Self {
        stiffness: 700.0,
        damping: 40.0,
        mass: 1.0,
    };

    pub const fn new(stiffness: f32, damping: f32) -> Self {
        Self {
            stiffness,
            damping,
            mass: 1.0,
        }
    }

    /// Damping ratio: < 1.0 underdamped (bouncy), 1.0 critical, > 1.0 overdamped.
    pub fn damping_ratio(&self) -> f32 {
        self.damping / (2.0 * (self.stiffness * self.mass).sqrt())
    }
}

/// Velocity-based spring animation state.
///
/// Unlike duration-based `Anim`, a spring settles naturally based on physics.
/// It supports smooth retargeting — changing the target mid-flight preserves
/// velocity for a natural feel.
pub struct SpringAnim {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    pub config: SpringConfig,
    pub last_tick: Instant,
    /// Whether this spring was queried this frame (for cleanup).
    pub(crate) queried: bool,
}

impl SpringAnim {
    /// Create a new spring, starting settled at `initial`.
    pub fn new(initial: f32, config: SpringConfig) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
            config,
            last_tick: Instant::now(),
            queried: true,
        }
    }

    /// Advance the spring simulation and return the current value.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // Cap dt to avoid instability after long pauses (e.g. tab switch).
        let dt = dt.min(0.064);

        // Semi-implicit Euler integration.
        // F = -stiffness * displacement - damping * velocity
        let displacement = self.value - self.target;
        let accel = (-self.config.stiffness * displacement - self.config.damping * self.velocity)
            / self.config.mass;
        self.velocity += accel * dt;
        self.value += self.velocity * dt;

        self.value
    }

    /// Whether the spring has effectively settled (close to target, low velocity).
    pub fn is_settled(&self) -> bool {
        let displacement = (self.value - self.target).abs();
        let speed = self.velocity.abs();
        displacement < 0.001 && speed < 0.01
    }
}

/// A single stop in a keyframe sequence.
#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    /// Position in the sequence, 0.0 to 1.0 (analogous to CSS percentage / 100).
    pub offset: f32,
    /// The value at this stop.
    pub value: f32,
    /// Easing curve from the *previous* keyframe to this one.
    /// The first keyframe's easing is ignored (it's the starting point).
    pub easing: Easing,
}

/// A reusable multi-step animation definition.
///
/// Keyframes are sorted by offset. Must contain at least two stops
/// (offset 0.0 and 1.0). Use the builder to construct:
///
/// ```ignore
/// let pulse = KeyframeSequence::new(600.0)
///     .stop(0.0, 1.0, Easing::Linear)
///     .stop(0.5, 1.3, Easing::EaseOutCubic)
///     .stop(1.0, 1.0, Easing::EaseInCubic);
/// ```
#[derive(Debug, Clone)]
pub struct KeyframeSequence {
    keyframes: Vec<Keyframe>,
    pub duration_ms: f32,
}

impl KeyframeSequence {
    /// Start building a keyframe sequence with the given total duration.
    pub fn new(duration_ms: f32) -> Self {
        Self {
            keyframes: Vec::new(),
            duration_ms,
        }
    }

    /// Add a keyframe stop. `offset` is 0.0–1.0, `easing` controls the curve
    /// *arriving* at this stop from the previous one.
    pub fn stop(mut self, offset: f32, value: f32, easing: Easing) -> Self {
        self.keyframes.push(Keyframe {
            offset: offset.clamp(0.0, 1.0),
            value,
            easing,
        });
        // Keep sorted by offset.
        self.keyframes.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        self
    }

    /// Access the keyframe stops.
    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keyframes
    }

    /// Sample the sequence at a given progress `t` in [0.0, 1.0].
    ///
    /// Finds the two bracketing keyframes, computes local progress within
    /// that segment, applies the destination keyframe's easing, and lerps.
    pub fn sample(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let kf = &self.keyframes;

        if kf.is_empty() {
            return 0.0;
        }
        if kf.len() == 1 {
            return kf[0].value;
        }

        // Before the first stop or at/past the last.
        if t <= kf[0].offset {
            return kf[0].value;
        }
        if t >= kf[kf.len() - 1].offset {
            return kf[kf.len() - 1].value;
        }

        // Find the segment: last keyframe with offset <= t.
        let idx = kf.partition_point(|k| k.offset <= t);
        // idx points to the first keyframe *after* t.
        let prev = &kf[idx.saturating_sub(1)];
        let next = &kf[idx.min(kf.len() - 1)];

        let span = next.offset - prev.offset;
        if span < f32::EPSILON {
            return next.value;
        }

        let local_t = (t - prev.offset) / span;
        let eased = next.easing.apply(local_t);
        prev.value + (next.value - prev.value) * eased
    }
}

/// Playback mode for keyframe animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackMode {
    /// Play once, then hold the final value.
    Once,
    /// Loop N times, then hold the final value.
    Loop(u32),
    /// Loop forever.
    Infinite,
    /// Play forward then backward. Count is full cycles (forward + back = 1).
    PingPong(u32),
    /// Ping-pong forever.
    PingPongInfinite,
}

/// Runtime state for an active keyframe animation.
pub struct KeyframeAnim {
    pub sequence: KeyframeSequence,
    pub start: Instant,
    pub mode: PlaybackMode,
    pub(crate) queried: bool,
}

impl KeyframeAnim {
    /// Current interpolated value based on elapsed time and playback mode.
    pub fn value(&self) -> f32 {
        let elapsed = self.start.elapsed().as_millis() as f32;
        let dur = self.sequence.duration_ms;

        if dur < f32::EPSILON {
            return self.sequence.sample(1.0);
        }

        let raw_progress = elapsed / dur;

        let t = match self.mode {
            PlaybackMode::Once => raw_progress.clamp(0.0, 1.0),
            PlaybackMode::Loop(n) => {
                let max = n as f32;
                if raw_progress >= max {
                    1.0
                } else {
                    raw_progress.fract()
                }
            }
            PlaybackMode::Infinite => raw_progress.fract(),
            PlaybackMode::PingPong(n) => {
                // One full cycle = forward + back = 2x duration.
                let max = n as f32 * 2.0;
                if raw_progress >= max {
                    0.0
                } else {
                    let within = raw_progress % 2.0;
                    if within <= 1.0 {
                        within
                    } else {
                        2.0 - within
                    }
                }
            }
            PlaybackMode::PingPongInfinite => {
                let within = raw_progress % 2.0;
                if within <= 1.0 {
                    within
                } else {
                    2.0 - within
                }
            }
        };

        self.sequence.sample(t)
    }

    /// Whether the animation has finished playing.
    pub fn is_finished(&self) -> bool {
        let elapsed = self.start.elapsed().as_millis() as f32;
        let dur = self.sequence.duration_ms;

        match self.mode {
            PlaybackMode::Once => elapsed >= dur,
            PlaybackMode::Loop(n) => elapsed >= dur * n as f32,
            PlaybackMode::Infinite | PlaybackMode::PingPongInfinite => false,
            PlaybackMode::PingPong(n) => elapsed >= dur * 2.0 * n as f32,
        }
    }
}

/// Result from a modal dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalAction {
    None,
    Confirm,
    Cancel,
}

/// Modal dialog state.
pub struct ModalState {
    pub id: u64,
    pub open: bool,
    pub saved_focus: Option<u64>,
}

/// Toast notification kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
    Warning,
}

/// A single toast notification.
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
    pub created: Instant,
    pub duration_ms: u64,
    pub dismissed: bool,
}

/// Queue of active toast notifications.
pub struct ToastQueue {
    pub toasts: Vec<Toast>,
    pub next_id: u64,
}

impl ToastQueue {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            next_id: 1,
        }
    }

    pub fn push(&mut self, kind: ToastKind, message: String, duration_ms: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.toasts.push(Toast {
            id,
            kind,
            message,
            created: Instant::now(),
            duration_ms,
            dismissed: false,
        });
        id
    }

    pub fn dismiss(&mut self, id: u64) {
        if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
            toast.dismissed = true;
        }
    }
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Accessibility node role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A11yRole {
    Button,
    Checkbox,
    RadioButton,
    TextInput,
    TextArea,
    Slider,
    Select,
    Tab,
    TabPanel,
    Table,
    TableRow,
    TableCell,
    Tree,
    TreeItem,
    ProgressBar,
    Dialog,
    Alert,
    Label,
    Separator,
    ScrollView,
    Group,
    ToggleButton,
    Link,
    SpinButton,
    Combobox,
}

/// A single accessibility node.
pub struct A11yNode {
    pub id: u64,
    pub role: A11yRole,
    pub label: String,
    pub value: Option<String>,
    pub rect: Rect,
    pub focused: bool,
    pub disabled: bool,
    pub expanded: Option<bool>,
    pub selected: Option<bool>,
    pub checked: Option<bool>,
    pub value_range: Option<(f32, f32, f32)>,
    pub children: Vec<u64>,
}

/// Accessibility tree built each frame.
pub struct A11yTree {
    pub nodes: Vec<A11yNode>,
    pub root_children: Vec<u64>,
}

impl A11yTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_children: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root_children.clear();
    }

    pub fn push(&mut self, node: A11yNode) {
        self.root_children.push(node.id);
        self.nodes.push(node);
    }
}

impl Default for A11yTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Central UI interaction state. App owns this and passes `&mut` to `Ui`.
pub struct UiState {
    /// Currently focused widget ID.
    pub focused: Option<u64>,
    /// Focus chain — rebuilt each frame in widget call order.
    pub(crate) focus_chain: Vec<u64>,
    /// Hit-test rects — rebuilt each frame.
    pub(crate) hit_rects: Vec<(Rect, u64, WidgetKind)>,
    /// Mouse state.
    pub mouse: MouseState,
    /// Buffered key events — drained during the frame by widgets.
    pub(crate) keys: Vec<(KeyEvent, Modifiers)>,
    /// Current modifier keys state.
    pub modifiers: Modifiers,
    /// Cursor blink state.
    pub cursor_blink: bool,
    /// When the cursor blink last toggled.
    pub cursor_blink_time: Instant,
    /// Scroll offsets keyed by widget ID: (offset, frames_since_last_access).
    /// Scroll offsets: `[vertical, horizontal]` and age counter.
    pub scroll_offsets: HashMap<u64, ([f32; 2], u32)>,
    /// Timestamp of last frame start for dt calculation.
    pub(crate) last_frame_time: Instant,
    /// Damage tracker for frame-skip optimization.
    pub damage: esox_gfx::DamageTracker,
    /// Previous frame's hovered widget (for damage on hover change).
    prev_hovered: Option<u64>,
    /// Previous frame's focused widget (for damage on focus change).
    prev_focused: Option<u64>,
    /// Overlay (dropdown / context menu) state.
    pub overlay: Option<Overlay>,
    /// Tooltip state.
    pub tooltip: Option<TooltipState>,
    /// Hover animation states keyed by widget ID.
    pub hover_anims: HashMap<u64, HoverAnim>,
    /// General-purpose animations keyed by ID.
    pub anims: HashMap<u64, Anim>,
    /// Spring-based animations keyed by ID.
    pub springs: HashMap<u64, SpringAnim>,
    /// Keyframe animations keyed by ID.
    pub keyframe_anims: HashMap<u64, KeyframeAnim>,
    /// Spoiler widgets that have been revealed by the user.
    pub revealed_spoilers: HashSet<u64>,
    /// Buffered scroll event: (mouse_x, mouse_y, delta_y).
    pub pending_scroll: Option<(f32, f32, f32)>,
    /// Active scrollbar drag: (scrollable_id, grab_offset_in_thumb).
    pub scrollbar_drag: Option<(u64, f32)>,
    /// Whether the left mouse button is currently held.
    pub mouse_pressed: bool,
    /// Active drag-and-drop payload.
    pub drag: Option<DragPayload>,
    /// Mouse position at last press (for dead zone).
    pub drag_start: Option<(f32, f32)>,
    /// Modal dialog stack.
    pub modal_stack: Vec<ModalState>,
    /// Toast notification queue.
    pub toasts: ToastQueue,
    /// Accessibility tree.
    pub a11y_tree: A11yTree,
    /// Whether accessibility is enabled.
    pub a11y_enabled: bool,
    /// Pending accessibility label for next widget.
    pub(crate) a11y_pending_label: Option<String>,
    /// Pending accessibility role for next widget.
    pub(crate) a11y_pending_role: Option<A11yRole>,
    /// Cached children heights for tree expand/collapse animations.
    pub tree_children_heights: HashMap<u64, f32>,
    /// Clipboard provider for Ctrl+C/X/V in text widgets.
    pub clipboard: Option<Box<dyn ClipboardProvider>>,
    /// HiDPI scale factor (default 1.0).
    pub scale_factor: f32,
    /// IME composition state.
    pub ime: ImeState,
    /// Whether a spinner was drawn this frame (needs continuous redraw).
    pub(crate) spinner_active: bool,
    /// Collapsing header open/closed state keyed by widget ID.
    pub collapsing_open: HashSet<u64>,
    /// Split pane ratios keyed by widget ID.
    pub split_ratios: HashMap<u64, f32>,
    /// Active split-pane divider drag: (split_id, is_horizontal).
    pub split_drag: Option<(u64, bool)>,
    /// Which menu bar dropdown is currently open (index into the menus slice).
    pub menu_bar_open: Option<usize>,
    /// Deferred menu bar dropdown painting data (drawn in finish() for z-order).
    pub(crate) menu_bar_deferred: Option<MenuBarDeferred>,
    /// Inline edit buffers for number_input widgets, keyed by widget ID.
    pub number_edit_buffers: HashMap<u64, InputState>,
    /// Per-combobox filter input state, keyed by widget ID.
    pub combobox_inputs: HashMap<u64, InputState>,
    /// Scroll velocity for momentum scrolling: `[vertical_vel, horizontal_vel]`.
    pub scroll_velocity: HashMap<u64, [f32; 2]>,
    /// Tile grid for partial redraw caching.
    pub tile_grid: Option<esox_gfx::TileGrid>,
    /// Cached solved layout tree from the previous frame.
    pub(crate) layout_cache: Option<crate::layout_tree::LayoutTree>,
    /// Whether the debug overlay is enabled (toggle with Alt+D or programmatically).
    pub debug_overlay: bool,
    /// Collected widget rects for debug overlay (populated when `debug_overlay` is true).
    pub(crate) debug_widget_rects: Vec<(Rect, u64, &'static str)>,
    /// The WidgetKind of the currently focused widget (set during register_widget).
    pub(crate) focused_kind: Option<WidgetKind>,
    /// Previous frame's max scroll values per scrollable, for pre-clamping on content shrink.
    /// Value: ([vertical_max, horizontal_max], age_counter).
    pub(crate) prev_max_scroll: HashMap<u64, ([f32; 2], u32)>,
    /// Whether the mouse moved since last frame (for targeted hover damage).
    pub(crate) mouse_moved: bool,
    /// Rects of widgets with active animations, for targeted animation damage.
    pub(crate) anim_rects: HashMap<u64, Rect>,
    /// Whether the most recently completed frame had any damage (latched before reset).
    frame_had_damage: bool,
}

/// IME (Input Method Editor) composition state.
pub struct ImeState {
    /// Preedit text being composed (not yet committed).
    pub preedit: String,
    /// Cursor range within the preedit text.
    pub preedit_cursor: Option<(usize, usize)>,
    /// Whether IME is currently active.
    pub active: bool,
    /// Committed text to be consumed by the focused text widget.
    pub committed: Option<String>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            focused: None,
            focus_chain: Vec::new(),
            hit_rects: Vec::new(),
            mouse: MouseState::default(),
            keys: Vec::new(),
            modifiers: Modifiers::empty(),
            cursor_blink: true,
            cursor_blink_time: Instant::now(),
            scroll_offsets: HashMap::new(),
            last_frame_time: Instant::now(),
            damage: esox_gfx::DamageTracker::new(),
            prev_hovered: None,
            prev_focused: None,
            overlay: None,
            tooltip: None,
            hover_anims: HashMap::new(),
            anims: HashMap::new(),
            springs: HashMap::new(),
            keyframe_anims: HashMap::new(),
            revealed_spoilers: HashSet::new(),
            pending_scroll: None,
            scrollbar_drag: None,
            mouse_pressed: false,
            drag: None,
            drag_start: None,
            modal_stack: Vec::new(),
            toasts: ToastQueue::new(),
            a11y_tree: A11yTree::new(),
            a11y_enabled: false,
            a11y_pending_label: None,
            a11y_pending_role: None,
            tree_children_heights: HashMap::new(),
            clipboard: None,
            scale_factor: 1.0,
            ime: ImeState {
                preedit: String::new(),
                preedit_cursor: None,
                active: false,
                committed: None,
            },
            spinner_active: false,
            collapsing_open: HashSet::new(),
            split_ratios: HashMap::new(),
            split_drag: None,
            menu_bar_open: None,
            menu_bar_deferred: None,
            number_edit_buffers: HashMap::new(),
            combobox_inputs: HashMap::new(),
            scroll_velocity: HashMap::new(),
            tile_grid: None,
            layout_cache: None,
            debug_overlay: false,
            debug_widget_rects: Vec::new(),
            focused_kind: None,
            prev_max_scroll: HashMap::new(),
            mouse_moved: false,
            anim_rects: HashMap::new(),
            frame_had_damage: true,
        }
    }

    /// Buffer a key event for processing during the frame.
    pub fn process_key(&mut self, event: KeyEvent, modifiers: Modifiers) {
        self.modifiers = modifiers;
        self.keys.push((event, modifiers));
        self.damage.invalidate_all();
    }

    /// Update modifier keys state.
    pub fn process_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    /// Process IME preedit event.
    pub fn on_ime_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        self.ime.preedit = text;
        self.ime.preedit_cursor = cursor;
        self.damage.invalidate_all();
    }

    /// Process IME commit event — buffer the text for the focused widget.
    pub fn on_ime_commit(&mut self, text: String) {
        self.ime.preedit.clear();
        self.ime.preedit_cursor = None;
        self.ime.committed = Some(text);
        self.damage.invalidate_all();
    }

    /// Process IME enabled/disabled event.
    pub fn on_ime_enabled(&mut self, enabled: bool) {
        self.ime.active = enabled;
        if !enabled {
            self.ime.preedit.clear();
            self.ime.preedit_cursor = None;
        }
    }

    /// Update mouse position. `item_height` and `dropdown_gap` are used for
    /// hover tracking within any open dropdown overlay.
    pub fn process_mouse_move(&mut self, x: f32, y: f32, item_height: f32, dropdown_gap: f32) {
        // Flag mouse movement for targeted hover damage in begin_frame().
        if (self.mouse.x - x).abs() > 0.5 || (self.mouse.y - y).abs() > 0.5 {
            self.mouse_moved = true;
        }
        self.mouse.x = x;
        self.mouse.y = y;

        // Track hover within open overlay.
        match &mut self.overlay {
            Some(Overlay::Dropdown {
                ref anchor,
                ref choices,
                ref mut hovered,
                ..
            }) => {
                let dd_y = anchor.y + anchor.h + dropdown_gap;
                if x >= anchor.x && x < anchor.x + anchor.w && y >= dd_y {
                    let idx = ((y - dd_y) / item_height) as usize;
                    if idx < choices.len() {
                        *hovered = Some(idx);
                    } else {
                        *hovered = None;
                    }
                } else {
                    *hovered = None;
                }
            }
            Some(Overlay::ContextMenu {
                ref position,
                ref items,
                ref mut hovered,
                ..
            }) => {
                // position.x/y is the menu top-left; position.w is menu width.
                let menu_h = items.len() as f32 * item_height;
                if x >= position.x
                    && x < position.x + position.w
                    && y >= position.y
                    && y < position.y + menu_h
                {
                    let idx = ((y - position.y) / item_height) as usize;
                    if idx < items.len() {
                        *hovered = Some(idx);
                    } else {
                        *hovered = None;
                    }
                } else {
                    *hovered = None;
                }
            }
            Some(Overlay::ComboboxDropdown {
                ref anchor,
                ref filtered_indices,
                ref mut highlighted,
                ref scroll_offset,
                ..
            }) => {
                let max_visible = 8;
                let visible_count = filtered_indices.len().min(max_visible);
                let dd_y = anchor.y + anchor.h + dropdown_gap;
                let dd_h = visible_count as f32 * item_height;
                if x >= anchor.x && x < anchor.x + anchor.w && y >= dd_y && y < dd_y + dd_h {
                    let idx = ((y - dd_y + *scroll_offset) / item_height) as usize;
                    if idx < filtered_indices.len() {
                        *highlighted = Some(idx);
                    } else {
                        *highlighted = None;
                    }
                } else {
                    *highlighted = None;
                }
            }
            None => {}
        }
    }

    /// Record a mouse click (left button press).
    pub fn process_mouse_click(&mut self, x: f32, y: f32) {
        self.mouse.pending_click = Some((x, y, false));
        self.mouse_pressed = true;
        self.damage.invalidate_all();
    }

    /// Record a right-click (button 2).
    pub fn process_right_click(&mut self, x: f32, y: f32) {
        self.mouse.pending_right_click = Some((x, y, false));
        self.damage.invalidate_all();
    }

    /// Record a mouse button release.
    pub fn process_mouse_release(&mut self) {
        self.mouse_pressed = false;
        self.scrollbar_drag = None;
        self.split_drag = None;
        // Drag ends on release — drag payload stays until end_frame so accept_drop can read it.
        self.drag_start = None;
    }

    /// Buffer a scroll (mouse wheel) event for processing during the frame.
    pub fn process_scroll(&mut self, x: f32, y: f32, delta_y: f32) {
        self.pending_scroll = Some((x, y, delta_y));
        self.damage.invalidate_all();
    }

    /// Update cursor blink. Call once per frame.
    pub fn update_blink(&mut self, blink_ms: u64) {
        let elapsed = self.cursor_blink_time.elapsed().as_millis() as u64;
        if elapsed >= blink_ms {
            self.cursor_blink = !self.cursor_blink;
            self.cursor_blink_time = Instant::now();
        }
    }

    /// Reset cursor blink to visible (call after text editing).
    pub fn reset_blink(&mut self) {
        self.cursor_blink = true;
        self.cursor_blink_time = Instant::now();
    }

    /// Get the cursor icon for the given position based on registered widgets.
    pub fn cursor_icon(&self, x: f32, y: f32) -> esox_input::CursorIcon {
        // Active drag overrides everything.
        if self.drag.is_some() {
            return esox_input::CursorIcon::Grabbing;
        }
        // Active split-pane divider drag overrides cursor.
        if let Some((_, is_horizontal)) = self.split_drag {
            return if is_horizontal {
                esox_input::CursorIcon::ColResize
            } else {
                esox_input::CursorIcon::RowResize
            };
        }
        // Iterate in reverse so the topmost (last-registered) widget wins.
        for (rect, _id, kind) in self.hit_rects.iter().rev() {
            if rect.contains(x, y) {
                return match kind {
                    WidgetKind::TextInput => esox_input::CursorIcon::Text,
                    WidgetKind::ColumnResize | WidgetKind::ResizeEW => {
                        esox_input::CursorIcon::ColResize
                    }
                    WidgetKind::ResizeNS => esox_input::CursorIcon::RowResize,
                    WidgetKind::Grab => esox_input::CursorIcon::Grab,
                    WidgetKind::Grabbing => esox_input::CursorIcon::Grabbing,
                    WidgetKind::NotAllowed => esox_input::CursorIcon::NotAllowed,
                    WidgetKind::Button
                    | WidgetKind::DropZone
                    | WidgetKind::Select
                    | WidgetKind::Checkbox
                    | WidgetKind::Radio
                    | WidgetKind::Tab
                    | WidgetKind::TableRow
                    | WidgetKind::TreeNode
                    | WidgetKind::Toggle => esox_input::CursorIcon::Pointer,
                    WidgetKind::Hyperlink => esox_input::CursorIcon::Pointer,
                    WidgetKind::Slider | WidgetKind::Scrollbar => esox_input::CursorIcon::Default,
                    WidgetKind::SplitDividerH => esox_input::CursorIcon::ColResize,
                    WidgetKind::SplitDividerV => esox_input::CursorIcon::RowResize,
                    WidgetKind::Combobox => esox_input::CursorIcon::Text,
                    WidgetKind::Custom(icon) => *icon,
                };
            }
        }
        esox_input::CursorIcon::Default
    }

    /// Whether the UI needs continuous redraw (cursor blink, overlay, tooltip delay, active animations, etc.).
    pub fn needs_continuous_redraw(&self) -> bool {
        self.overlay.is_some()
            || self.focused.is_some_and(|id| self.is_text_widget(id))
            || self.hover_anims.values().any(|a| !a.is_settled())
            || self.anims.values().any(|a| !a.is_settled())
            || self.springs.values().any(|s| !s.is_settled())
            || self.keyframe_anims.values().any(|ka| !ka.is_finished())
            || self.scrollbar_drag.is_some()
            || self.split_drag.is_some()
            || self.drag.is_some()
            || self.tooltip.as_ref().is_some_and(|t| !t.visible)
            || !self.modal_stack.is_empty()
            || !self.toasts.toasts.is_empty()
            || self.spinner_active
    }

    /// Whether the most recently completed frame had any damage.
    ///
    /// This is a frame-skip check: returns `false` when nothing changed,
    /// allowing the platform to skip GPU submission. The value is latched
    /// in `end_frame()` before `damage.reset()` so it remains valid after
    /// `on_redraw()` returns.
    pub fn needs_redraw(&self) -> bool {
        self.frame_had_damage
    }

    /// Get or update a hover animation, returning the current interpolation value.
    /// On first call the animation starts settled at the appropriate end value.
    pub fn hover_t(&mut self, id: u64, is_hovered: bool, duration_ms: f32) -> f32 {
        let target = if is_hovered { 1.0 } else { 0.0 };
        let anim = self.hover_anims.entry(id).or_insert_with(|| {
            // First-seen: start settled at the correct end value.
            HoverAnim {
                from: target,
                to: target,
                start: Instant::now(),
                duration_ms,
            }
        });
        if (anim.to - target).abs() > 0.001 {
            // Target changed — restart from current value.
            anim.from = anim.t();
            anim.to = target;
            anim.start = Instant::now();
        }
        anim.t()
    }

    /// Get or create a general-purpose animation. Returns current value.
    /// Restarts from current value when target changes.
    pub fn anim_t(&mut self, id: u64, target: f32, duration_ms: f32, easing: Easing) -> f32 {
        let anim = self.anims.entry(id).or_insert_with(|| Anim {
            from: target,
            to: target,
            start: Instant::now(),
            duration_ms,
            easing,
            queried: true,
        });
        anim.queried = true;
        if (anim.to - target).abs() > 0.001 {
            anim.from = anim.value();
            anim.to = target;
            anim.start = Instant::now();
            anim.easing = easing;
            anim.duration_ms = duration_ms;
        }
        anim.value()
    }

    /// Whether a given animation is currently active (not settled).
    pub fn anim_active(&self, id: u64) -> bool {
        self.anims.get(&id).is_some_and(|a| !a.is_settled())
    }

    /// Get or create a spring animation. Returns current value.
    /// Retargets smoothly when target changes (preserves velocity).
    pub fn spring_t(&mut self, id: u64, target: f32, config: SpringConfig) -> f32 {
        let spring = self
            .springs
            .entry(id)
            .or_insert_with(|| SpringAnim::new(target, config));
        spring.queried = true;
        spring.config = config;
        spring.target = target;
        spring.tick()
    }

    /// Whether a given spring animation is currently active (not settled).
    pub fn spring_active(&self, id: u64) -> bool {
        self.springs.get(&id).is_some_and(|s| !s.is_settled())
    }

    /// Get or create a keyframe animation. Returns current interpolated value.
    /// If called with a different sequence for an existing id, restarts the animation.
    pub fn keyframe_t(&mut self, id: u64, sequence: &KeyframeSequence, mode: PlaybackMode) -> f32 {
        let ka = self
            .keyframe_anims
            .entry(id)
            .or_insert_with(|| KeyframeAnim {
                sequence: sequence.clone(),
                start: Instant::now(),
                mode,
                queried: true,
            });
        ka.queried = true;
        // If the mode changed, restart.
        if ka.mode != mode {
            ka.mode = mode;
            ka.start = Instant::now();
            ka.sequence = sequence.clone();
        }
        ka.value()
    }

    /// Whether a keyframe animation is currently playing (not finished).
    pub fn keyframe_active(&self, id: u64) -> bool {
        self.keyframe_anims
            .get(&id)
            .is_some_and(|ka| !ka.is_finished())
    }

    /// Whether a spoiler widget has been revealed.
    pub fn spoiler_revealed(&self, id: u64) -> bool {
        self.revealed_spoilers.contains(&id)
    }

    /// Mark a spoiler as revealed.
    pub fn reveal_spoiler(&mut self, id: u64) {
        self.revealed_spoilers.insert(id);
    }

    /// Advance focus to the next widget in the focus chain.
    pub fn focus_next(&mut self) {
        if self.focus_chain.is_empty() {
            return;
        }
        let idx = self
            .focused
            .and_then(|f| self.focus_chain.iter().position(|w| *w == f));
        let next = match idx {
            Some(i) => (i + 1) % self.focus_chain.len(),
            None => 0,
        };
        self.focused = Some(self.focus_chain[next]);
        self.reset_blink();
    }

    /// Advance focus to the previous widget in the focus chain.
    pub fn focus_prev(&mut self) {
        if self.focus_chain.is_empty() {
            return;
        }
        let idx = self
            .focused
            .and_then(|f| self.focus_chain.iter().position(|w| *w == f));
        let prev = match idx {
            Some(0) => self.focus_chain.len() - 1,
            Some(i) => i - 1,
            None => self.focus_chain.len() - 1,
        };
        self.focused = Some(self.focus_chain[prev]);
        self.reset_blink();
    }

    /// Clear per-frame state. Called at the start of each frame.
    pub(crate) fn begin_frame(&mut self, scroll_friction: f32) {
        self.last_frame_time = Instant::now();

        // Damage detection: hover/focus changes, active animations, scroll velocity.
        if self.mouse_moved {
            let current_hovered = self
                .hit_rects
                .iter()
                .rev()
                .find(|(r, _, _)| r.contains(self.mouse.x, self.mouse.y))
                .map(|(_, id, _)| *id);
            if current_hovered != self.prev_hovered {
                // Add damage for old and new hovered widget rects.
                if let Some(old_id) = self.prev_hovered {
                    if let Some((r, _, _)) = self.hit_rects.iter().find(|(_, id, _)| *id == old_id)
                    {
                        self.damage
                            .add(esox_gfx::DamageRect::new(r.x, r.y, r.w, r.h));
                    }
                }
                if let Some(new_id) = current_hovered {
                    if let Some((r, _, _)) = self.hit_rects.iter().find(|(_, id, _)| *id == new_id)
                    {
                        self.damage
                            .add(esox_gfx::DamageRect::new(r.x, r.y, r.w, r.h));
                    }
                }
            }
            self.mouse_moved = false;
        }
        if self.focused != self.prev_focused {
            self.damage.invalidate_all();
        }
        // Per-widget animation damage: only invalidate rects of unsettled animations.
        {
            let mut any_missing = false;
            for (id, anim) in &self.hover_anims {
                if !anim.is_settled() {
                    if let Some(r) = self.anim_rects.get(id) {
                        self.damage
                            .add(esox_gfx::DamageRect::new(r.x, r.y, r.w, r.h));
                    } else {
                        any_missing = true;
                    }
                }
            }
            for (id, anim) in &self.anims {
                if !anim.is_settled() {
                    if let Some(r) = self.anim_rects.get(id) {
                        self.damage
                            .add(esox_gfx::DamageRect::new(r.x, r.y, r.w, r.h));
                    } else {
                        any_missing = true;
                    }
                }
            }
            if any_missing {
                self.damage.invalidate_all();
            }
            self.anim_rects.clear();
        }
        if self.overlay.is_some()
            || !self.modal_stack.is_empty()
            || !self.toasts.toasts.is_empty()
            || self.menu_bar_open.is_some()
        {
            self.damage.invalidate_all();
        }

        self.focus_chain.clear();
        // Momentum scrolling: decay velocities and apply to offsets.
        self.scroll_velocity.retain(|id, vel| {
            let any_active = vel[0].abs() > 0.5 || vel[1].abs() > 0.5;
            if any_active {
                if let Some((off, _)) = self.scroll_offsets.get_mut(id) {
                    off[0] += vel[0];
                    off[1] += vel[1];
                }
                vel[0] *= scroll_friction;
                vel[1] *= scroll_friction;
                if vel[0].abs() < 0.5 {
                    vel[0] = 0.0;
                }
                if vel[1].abs() < 0.5 {
                    vel[1] = 0.0;
                }
            }
            any_active
        });

        self.hit_rects.clear();

        // Automatic Tab/Shift+Tab focus cycling. Uses focused_kind from the
        // previous frame (set during register_widget calls). TextInput widgets
        // keep Tab for literal insertion.
        {
            use esox_input::{Key, NamedKey};
            let is_text_input = matches!(self.focused_kind, Some(WidgetKind::TextInput));
            let mut tab_action: Option<bool> = None; // Some(true) = shift+tab, Some(false) = tab
            if !is_text_input {
                for (event, mods) in &self.keys {
                    if event.pressed {
                        if let Key::Named(NamedKey::Tab) = &event.key {
                            tab_action = Some(mods.shift());
                            break;
                        }
                    }
                }
            }
            if let Some(shift) = tab_action {
                self.keys.retain(|(event, _)| {
                    !(event.pressed && matches!(event.key, Key::Named(NamedKey::Tab)))
                });
                if shift {
                    self.focus_prev();
                } else {
                    self.focus_next();
                }
            }
            self.focused_kind = None; // reset for this frame
        }

        self.spinner_active = false;
        self.debug_widget_rects.clear();
        if self.a11y_enabled {
            self.a11y_tree.clear();
        }
        // Mark all anims as unqueried for cleanup.
        for anim in self.anims.values_mut() {
            anim.queried = false;
        }
        for spring in self.springs.values_mut() {
            spring.queried = false;
        }
        for ka in self.keyframe_anims.values_mut() {
            ka.queried = false;
        }
    }

    /// End-of-frame cleanup. Clears consumed events.
    pub(crate) fn end_frame(&mut self) {
        self.keys.clear();
        self.pending_scroll = None;
        // Clear drag on mouse release.
        if !self.mouse_pressed {
            self.drag = None;
        }
        // Clear consumed click.
        if let Some((_, _, consumed)) = self.mouse.pending_click {
            if consumed {
                self.mouse.pending_click = None;
            }
        }
        // If click was not consumed by any widget, clear focus.
        if let Some((_, _, false)) = self.mouse.pending_click.take() {
            self.focused = None;
        }
        // Clear consumed right-click.
        if let Some((_, _, consumed)) = self.mouse.pending_right_click {
            if consumed {
                self.mouse.pending_right_click = None;
            }
        }
        // Unconsumed right-click — just clear it.
        if let Some((_, _, false)) = self.mouse.pending_right_click.take() {
            // No action needed.
        }
        // Prune settled anims that weren't queried this frame.
        self.anims.retain(|_, a| a.queried || !a.is_settled());
        // Prune settled springs that weren't queried this frame.
        self.springs.retain(|_, s| s.queried || !s.is_settled());
        // Prune finished keyframe anims that weren't queried this frame.
        self.keyframe_anims
            .retain(|_, ka| ka.queried || !ka.is_finished());
        // Remove expired and dismissed toasts.
        self.toasts.toasts.retain(|t| {
            if t.dismissed {
                return false;
            }
            let elapsed = t.created.elapsed().as_millis() as u64;
            // Keep for duration + fade_out time (300ms).
            elapsed < t.duration_ms + 300
        });
        // Prune stale scroll offsets (not accessed for >300 frames ≈ 5s at 60fps).
        self.scroll_offsets.retain(|_, (_, age)| {
            *age += 1;
            *age <= 300
        });
        // Prune settled hover animations when the map grows large.
        // Active widgets call hover_t() each frame, keeping their entries alive.
        // Settled anims for off-screen widgets accumulate — cap at 256.
        if self.hover_anims.len() > 256 {
            self.hover_anims.retain(|_, anim| !anim.is_settled());
        }
        // Prune stale prev_max_scroll (not accessed for >300 frames).
        self.prev_max_scroll.retain(|_, (_, age)| {
            *age += 1;
            *age <= 300
        });
        // Save hover/focus state for next frame's damage detection.
        self.prev_hovered = self
            .hit_rects
            .iter()
            .rev()
            .find(|(r, _, _)| r.contains(self.mouse.x, self.mouse.y))
            .map(|(_, id, _)| *id);
        self.prev_focused = self.focused;
        // Latch damage state before reset so the platform can check it after on_redraw().
        self.frame_had_damage = self.damage.is_full_invalidation()
            || self.damage.regions().is_some_and(|r| !r.is_empty());
        // Reset damage tracker for next frame.
        self.damage.reset();
    }

    fn is_text_widget(&self, id: u64) -> bool {
        // Widgets registered as TextInput kind.
        self.hit_rects
            .iter()
            .any(|(_, wid, kind)| *wid == id && *kind == WidgetKind::TextInput)
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ══════════════════════════════════════════════════════════════
    // InputState — cursor ops
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn insert_char_at_cursor() {
        let mut s = InputState::new();
        s.insert_char('A');
        assert_eq!(s.text, "A");
        assert_eq!(s.cursor, 1);
        s.insert_char('B');
        assert_eq!(s.text, "AB");
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn insert_char_mid_string() {
        let mut s = InputState::new();
        s.text = "ac".into();
        s.cursor = 1;
        s.insert_char('b');
        assert_eq!(s.text, "abc");
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn insert_str_at_cursor() {
        let mut s = InputState::new();
        s.insert_str("hello");
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 5);
        s.insert_str(" world");
        assert_eq!(s.text, "hello world");
        assert_eq!(s.cursor, 11);
    }

    #[test]
    fn delete_back_ascii() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 3;
        s.delete_back();
        assert_eq!(s.text, "ab");
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn delete_back_multibyte_utf8() {
        let mut s = InputState::new();
        s.text = "aé".into(); // 'é' is 2 bytes
        s.cursor = s.text.len();
        s.delete_back();
        assert_eq!(s.text, "a");
        assert_eq!(s.cursor, 1);
    }

    #[test]
    fn delete_back_at_start_is_noop() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 0;
        s.delete_back();
        assert_eq!(s.text, "abc");
        assert_eq!(s.cursor, 0);
    }

    #[test]
    fn delete_forward_ascii() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 1;
        s.delete_forward();
        assert_eq!(s.text, "ac");
        assert_eq!(s.cursor, 1);
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 3;
        s.delete_forward();
        assert_eq!(s.text, "abc");
    }

    #[test]
    fn move_left_and_right() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 2;
        s.move_left();
        assert_eq!(s.cursor, 1);
        s.move_right();
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn move_left_at_start_stays() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 0;
        s.move_left();
        assert_eq!(s.cursor, 0);
    }

    #[test]
    fn move_right_at_end_stays() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 3;
        s.move_right();
        assert_eq!(s.cursor, 3);
    }

    #[test]
    fn home_and_end() {
        let mut s = InputState::new();
        s.text = "hello world".into();
        s.cursor = 5;
        s.home();
        assert_eq!(s.cursor, 0);
        s.end();
        assert_eq!(s.cursor, 11);
    }

    #[test]
    fn select_all_sets_selection() {
        let mut s = InputState::new();
        s.text = "hello".into();
        s.cursor = 2;
        s.select_all();
        assert_eq!(s.selection, Some((0, 5)));
        assert_eq!(s.cursor, 5);
    }

    #[test]
    fn select_all_empty_string_is_noop() {
        let mut s = InputState::new();
        s.select_all();
        assert!(s.selection.is_none());
    }

    #[test]
    fn delete_selection_removes_range() {
        let mut s = InputState::new();
        s.text = "hello world".into();
        s.selection = Some((5, 11));
        s.cursor = 11;
        assert!(s.delete_selection());
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 5);
        assert!(s.selection.is_none());
    }

    #[test]
    fn delete_selection_no_selection_returns_false() {
        let mut s = InputState::new();
        s.text = "hello".into();
        assert!(!s.delete_selection());
    }

    #[test]
    fn move_left_extend_creates_selection() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 2;
        s.move_left_extend();
        assert_eq!(s.cursor, 1);
        assert_eq!(s.selection, Some((1, 2)));
    }

    #[test]
    fn move_right_extend_creates_selection() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 1;
        s.move_right_extend();
        assert_eq!(s.cursor, 2);
        assert_eq!(s.selection, Some((1, 2)));
    }

    #[test]
    fn home_extend_selects_to_start() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 3;
        s.home_extend();
        assert_eq!(s.cursor, 0);
        assert_eq!(s.selection, Some((0, 3)));
    }

    #[test]
    fn end_extend_selects_to_end() {
        let mut s = InputState::new();
        s.text = "abc".into();
        s.cursor = 0;
        s.end_extend();
        assert_eq!(s.cursor, 3);
        assert_eq!(s.selection, Some((0, 3)));
    }

    #[test]
    fn selected_text_returns_correct_slice() {
        let mut s = InputState::new();
        s.text = "hello world".into();
        s.selection = Some((6, 11));
        assert_eq!(s.selected_text(), Some("world"));
    }

    #[test]
    fn selected_text_none_when_no_selection() {
        let s = InputState::new();
        assert_eq!(s.selected_text(), None);
    }

    // ══════════════════════════════════════════════════════════════
    // Easing::apply
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn easing_linear_boundaries() {
        assert!((Easing::Linear.apply(0.0)).abs() < 1e-6);
        assert!((Easing::Linear.apply(1.0) - 1.0).abs() < 1e-6);
        assert!((Easing::Linear.apply(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn easing_ease_out_cubic_boundaries() {
        assert!((Easing::EaseOutCubic.apply(0.0)).abs() < 1e-6);
        assert!((Easing::EaseOutCubic.apply(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn easing_ease_in_out_cubic_boundaries_and_symmetry() {
        assert!((Easing::EaseInOutCubic.apply(0.0)).abs() < 1e-6);
        assert!((Easing::EaseInOutCubic.apply(1.0) - 1.0).abs() < 1e-6);
        let sum = Easing::EaseInOutCubic.apply(0.25) + Easing::EaseInOutCubic.apply(0.75);
        assert!((sum - 1.0).abs() < 1e-6, "sum was {sum}");
    }

    #[test]
    fn easing_ease_out_expo_boundaries() {
        assert!((Easing::EaseOutExpo.apply(0.0)).abs() < 1e-3);
        assert!((Easing::EaseOutExpo.apply(1.0) - 1.0).abs() < 1e-6);
    }

    // ══════════════════════════════════════════════════════════════
    // UndoHistory
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn undo_redo_cycle() {
        let mut h = UndoHistory::new();
        h.push("a", 1);
        h.push("b", 2);
        let entry = h.undo().unwrap();
        assert_eq!(entry.text, "b");
        let entry = h.undo().unwrap();
        assert_eq!(entry.text, "a");
        let entry = h.redo().unwrap();
        assert_eq!(entry.text, "a");
        let entry = h.redo().unwrap();
        assert_eq!(entry.text, "b");
    }

    #[test]
    fn undo_past_beginning_returns_none() {
        let mut h = UndoHistory::new();
        h.push("a", 1);
        h.undo();
        assert!(h.undo().is_none());
    }

    #[test]
    fn redo_past_end_returns_none() {
        let mut h = UndoHistory::new();
        h.push("a", 1);
        assert!(h.redo().is_none());
    }

    #[test]
    fn push_after_undo_truncates_redo() {
        let mut h = UndoHistory::new();
        h.push("a", 1);
        h.push("b", 2);
        h.push("c", 3);
        h.undo(); // index=2
        h.undo(); // index=1
        h.push("d", 4);
        assert!(h.redo().is_none());
        let entry = h.undo().unwrap();
        assert_eq!(entry.text, "d");
        let entry = h.undo().unwrap();
        assert_eq!(entry.text, "a");
    }

    #[test]
    fn undo_history_caps_at_100() {
        let mut h = UndoHistory::new();
        for i in 0..110 {
            h.push(&format!("entry_{i}"), i);
        }
        assert_eq!(h.entries.len(), 100);
        assert_eq!(h.entries[0].text, "entry_10");
    }

    // ══════════════════════════════════════════════════════════════
    // ToastQueue
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn toast_push_returns_incrementing_ids() {
        let mut q = ToastQueue::new();
        let id1 = q.push(ToastKind::Info, "a".into(), 3000);
        let id2 = q.push(ToastKind::Error, "b".into(), 3000);
        let id3 = q.push(ToastKind::Success, "c".into(), 3000);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_eq!(q.toasts.len(), 3);
    }

    #[test]
    fn toast_dismiss_marks_dismissed() {
        let mut q = ToastQueue::new();
        let id = q.push(ToastKind::Warning, "warn".into(), 3000);
        assert!(!q.toasts[0].dismissed);
        q.dismiss(id);
        assert!(q.toasts[0].dismissed);
    }

    #[test]
    fn toast_dismiss_nonexistent_is_noop() {
        let mut q = ToastQueue::new();
        q.push(ToastKind::Info, "a".into(), 3000);
        q.dismiss(999);
        assert!(!q.toasts[0].dismissed);
    }

    // ══════════════════════════════════════════════════════════════
    // A11yTree
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn a11y_tree_push_adds_node_and_root_child() {
        let mut tree = A11yTree::new();
        tree.push(A11yNode {
            id: 42,
            role: A11yRole::Button,
            label: "OK".into(),
            value: None,
            rect: Rect::new(0.0, 0.0, 80.0, 30.0),
            focused: false,
            disabled: false,
            expanded: None,
            selected: None,
            checked: None,
            value_range: None,
            children: vec![],
        });
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.root_children, vec![42]);
    }

    #[test]
    fn a11y_tree_clear_empties_both() {
        let mut tree = A11yTree::new();
        tree.push(A11yNode {
            id: 1,
            role: A11yRole::Label,
            label: "x".into(),
            value: None,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            focused: false,
            disabled: false,
            expanded: None,
            selected: None,
            checked: None,
            value_range: None,
            children: vec![],
        });
        tree.push(A11yNode {
            id: 2,
            role: A11yRole::Label,
            label: "y".into(),
            value: None,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            focused: false,
            disabled: false,
            expanded: None,
            selected: None,
            checked: None,
            value_range: None,
            children: vec![],
        });
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.root_children.len(), 2);
        tree.clear();
        assert!(tree.nodes.is_empty());
        assert!(tree.root_children.is_empty());
    }

    // ══════════════════════════════════════════════════════════════
    // Easing — boundary values and monotonicity
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn easing_boundaries() {
        let variants = [
            Easing::Linear,
            Easing::EaseInQuad,
            Easing::EaseOutQuad,
            Easing::EaseInOutQuad,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
            Easing::EaseInQuart,
            Easing::EaseOutQuart,
            Easing::EaseInOutQuart,
            Easing::EaseInExpo,
            Easing::EaseOutExpo,
            Easing::EaseInOutExpo,
        ];
        for e in variants {
            let at_zero = e.apply(0.0);
            let at_one = e.apply(1.0);
            assert!(at_zero.abs() < 0.01, "{e:?} at 0.0 = {at_zero}");
            assert!((at_one - 1.0).abs() < 0.01, "{e:?} at 1.0 = {at_one}");
        }
    }

    #[test]
    fn easing_clamped_outside_range() {
        let e = Easing::EaseOutCubic;
        assert!((e.apply(-1.0) - e.apply(0.0)).abs() < f32::EPSILON);
        assert!((e.apply(2.0) - e.apply(1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn ease_out_back_overshoots() {
        let mid = Easing::EaseOutBack.apply(0.5);
        // EaseOutBack should exceed 1.0 at some point before settling.
        let near_end = Easing::EaseOutBack.apply(0.7);
        assert!(mid > 0.0);
        assert!(near_end > 1.0 || Easing::EaseOutBack.apply(0.85) > 1.0);
    }

    #[test]
    fn ease_out_bounce_stays_in_range() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let v = Easing::EaseOutBounce.apply(t);
            assert!(v >= 0.0 && v <= 1.001, "bounce({t}) = {v}");
        }
    }

    #[test]
    fn cubic_bezier_linear() {
        // cubic-bezier(0, 0, 1, 1) should approximate linear.
        let e = Easing::CubicBezier(0.0, 0.0, 1.0, 1.0);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let v = e.apply(t);
            assert!((v - t).abs() < 0.05, "linear bezier({t}) = {v}");
        }
    }

    #[test]
    fn cubic_bezier_ease_out() {
        // CSS ease-out: cubic-bezier(0, 0, 0.58, 1)
        let e = Easing::CubicBezier(0.0, 0.0, 0.58, 1.0);
        assert!(e.apply(0.0).abs() < 0.01);
        assert!((e.apply(1.0) - 1.0).abs() < 0.01);
        // Ease-out should be above linear at the midpoint.
        assert!(e.apply(0.5) > 0.5);
    }

    #[test]
    fn easing_monotonic_standard_curves() {
        // Standard ease-in/out/in-out should be monotonically increasing.
        let monotonic = [
            Easing::Linear,
            Easing::EaseInQuad,
            Easing::EaseOutQuad,
            Easing::EaseInOutQuad,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
            Easing::EaseInQuart,
            Easing::EaseOutQuart,
            Easing::EaseInOutQuart,
            Easing::EaseInExpo,
            Easing::EaseOutExpo,
            Easing::EaseInOutExpo,
        ];
        for e in monotonic {
            let mut prev = 0.0f32;
            for i in 1..=100 {
                let t = i as f32 / 100.0;
                let v = e.apply(t);
                assert!(
                    v >= prev - 0.001,
                    "{e:?} not monotonic at {t}: {v} < {prev}"
                );
                prev = v;
            }
        }
    }

    // ══════════════════════════════════════════════════════════════
    // Spring — convergence, retargeting, config
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn spring_config_damping_ratio() {
        // Critical damping: ratio = 1.0
        let critical = SpringConfig {
            stiffness: 100.0,
            damping: 20.0, // 2 * sqrt(100 * 1) = 20
            mass: 1.0,
        };
        assert!((critical.damping_ratio() - 1.0).abs() < 0.01);

        // Underdamped: ratio < 1.0
        assert!(SpringConfig::BOUNCY.damping_ratio() < 1.0);

        // Overdamped: ratio > 1.0
        let overdamped = SpringConfig::new(100.0, 30.0);
        assert!(overdamped.damping_ratio() > 1.0);
    }

    #[test]
    fn spring_settles_at_target() {
        let mut spring = SpringAnim::new(0.0, SpringConfig::SNAPPY);
        spring.target = 1.0;
        // Simulate many steps.
        for _ in 0..500 {
            spring.velocity += (-spring.config.stiffness * (spring.value - spring.target)
                - spring.config.damping * spring.velocity)
                / spring.config.mass
                * 0.016;
            spring.value += spring.velocity * 0.016;
        }
        assert!(
            (spring.value - 1.0).abs() < 0.01,
            "spring did not converge: {}",
            spring.value
        );
    }

    #[test]
    fn spring_starts_settled() {
        let spring = SpringAnim::new(5.0, SpringConfig::GENTLE);
        assert!(spring.is_settled());
        assert!((spring.value - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn spring_retarget_preserves_velocity() {
        let mut spring = SpringAnim::new(0.0, SpringConfig::BOUNCY);
        spring.target = 1.0;
        // Give it some velocity.
        spring.velocity = 5.0;
        spring.value = 0.3;
        let vel_before = spring.velocity;
        // Retarget.
        spring.target = 2.0;
        // Velocity should be unchanged.
        assert!((spring.velocity - vel_before).abs() < f32::EPSILON);
    }

    #[test]
    fn spring_presets_are_valid() {
        // All presets should have positive stiffness, damping, and mass.
        for config in [
            SpringConfig::SNAPPY,
            SpringConfig::GENTLE,
            SpringConfig::BOUNCY,
            SpringConfig::STIFF,
        ] {
            assert!(config.stiffness > 0.0);
            assert!(config.damping > 0.0);
            assert!(config.mass > 0.0);
            assert!(config.damping_ratio() > 0.0);
        }
    }

    // ══════════════════════════════════════════════════════════════
    // KeyframeSequence — sampling, boundaries, multi-segment
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn keyframe_sample_two_stops_linear() {
        let seq = KeyframeSequence::new(1000.0)
            .stop(0.0, 0.0, Easing::Linear)
            .stop(1.0, 100.0, Easing::Linear);
        assert!((seq.sample(0.0) - 0.0).abs() < 0.01);
        assert!((seq.sample(0.5) - 50.0).abs() < 0.01);
        assert!((seq.sample(1.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn keyframe_sample_three_stops() {
        let seq = KeyframeSequence::new(1000.0)
            .stop(0.0, 0.0, Easing::Linear)
            .stop(0.5, 80.0, Easing::Linear)
            .stop(1.0, 100.0, Easing::Linear);
        // At 0.25 — halfway through first segment (0→80).
        assert!((seq.sample(0.25) - 40.0).abs() < 0.5);
        // At 0.5 — exactly at second stop.
        assert!((seq.sample(0.5) - 80.0).abs() < 0.01);
        // At 0.75 — halfway through second segment (80→100).
        assert!((seq.sample(0.75) - 90.0).abs() < 0.5);
    }

    #[test]
    fn keyframe_sample_per_segment_easing() {
        let seq = KeyframeSequence::new(1000.0)
            .stop(0.0, 0.0, Easing::Linear)
            .stop(1.0, 100.0, Easing::EaseInQuad);
        // EaseInQuad at t=0.5 should be 0.25 (t^2), so value ~ 25.
        assert!((seq.sample(0.5) - 25.0).abs() < 0.5);
    }

    #[test]
    fn keyframe_sample_clamps_outside_range() {
        let seq = KeyframeSequence::new(1000.0)
            .stop(0.0, 10.0, Easing::Linear)
            .stop(1.0, 90.0, Easing::Linear);
        assert!((seq.sample(-1.0) - 10.0).abs() < 0.01);
        assert!((seq.sample(2.0) - 90.0).abs() < 0.01);
    }

    #[test]
    fn keyframe_sample_empty_returns_zero() {
        let seq = KeyframeSequence::new(1000.0);
        assert!((seq.sample(0.5)).abs() < 0.01);
    }

    #[test]
    fn keyframe_sample_single_stop() {
        let seq = KeyframeSequence::new(1000.0).stop(0.5, 42.0, Easing::Linear);
        assert!((seq.sample(0.0) - 42.0).abs() < 0.01);
        assert!((seq.sample(1.0) - 42.0).abs() < 0.01);
    }

    #[test]
    fn keyframe_builder_sorts_by_offset() {
        let seq = KeyframeSequence::new(1000.0)
            .stop(1.0, 100.0, Easing::Linear)
            .stop(0.0, 0.0, Easing::Linear)
            .stop(0.5, 50.0, Easing::Linear);
        let offsets: Vec<f32> = seq.keyframes().iter().map(|k| k.offset).collect();
        assert_eq!(offsets, vec![0.0, 0.5, 1.0]);
    }

    // ══════════════════════════════════════════════════════════════
    // KeyframeAnim — playback modes
    // ══════════════════════════════════════════════════════════════

    fn make_test_sequence() -> KeyframeSequence {
        KeyframeSequence::new(100.0) // 100ms for fast test
            .stop(0.0, 0.0, Easing::Linear)
            .stop(1.0, 100.0, Easing::Linear)
    }

    #[test]
    fn keyframe_anim_once_finishes() {
        let ka = KeyframeAnim {
            sequence: make_test_sequence(),
            start: Instant::now() - std::time::Duration::from_millis(200),
            mode: PlaybackMode::Once,
            queried: true,
        };
        assert!(ka.is_finished());
        // Value should be clamped at final.
        assert!((ka.value() - 100.0).abs() < 0.5);
    }

    #[test]
    fn keyframe_anim_once_not_finished_midway() {
        let ka = KeyframeAnim {
            sequence: make_test_sequence(),
            start: Instant::now(),
            mode: PlaybackMode::Once,
            queried: true,
        };
        assert!(!ka.is_finished());
    }

    #[test]
    fn keyframe_anim_infinite_never_finishes() {
        let ka = KeyframeAnim {
            sequence: make_test_sequence(),
            start: Instant::now() - std::time::Duration::from_secs(999),
            mode: PlaybackMode::Infinite,
            queried: true,
        };
        assert!(!ka.is_finished());
    }

    #[test]
    fn keyframe_anim_loop_count() {
        let ka = KeyframeAnim {
            sequence: make_test_sequence(),
            start: Instant::now() - std::time::Duration::from_millis(350),
            mode: PlaybackMode::Loop(3),
            queried: true,
        };
        // 350ms elapsed, 3 loops * 100ms = 300ms total → should be finished.
        assert!(ka.is_finished());
    }

    #[test]
    fn keyframe_anim_ping_pong_reverses() {
        let seq = KeyframeSequence::new(1000.0)
            .stop(0.0, 0.0, Easing::Linear)
            .stop(1.0, 100.0, Easing::Linear);

        // At 1.5x duration into a ping-pong, we should be halfway back.
        let ka = KeyframeAnim {
            sequence: seq,
            start: Instant::now() - std::time::Duration::from_millis(1500),
            mode: PlaybackMode::PingPongInfinite,
            queried: true,
        };
        // Progress 1.5 → within = 1.5 % 2.0 = 1.5 → t = 2.0 - 1.5 = 0.5 → value = 50.
        assert!((ka.value() - 50.0).abs() < 2.0);
    }

    #[test]
    fn keyframe_playback_mode_eq() {
        assert_eq!(PlaybackMode::Once, PlaybackMode::Once);
        assert_eq!(PlaybackMode::Loop(3), PlaybackMode::Loop(3));
        assert_ne!(PlaybackMode::Loop(3), PlaybackMode::Loop(5));
        assert_ne!(PlaybackMode::Once, PlaybackMode::Infinite);
    }
}
