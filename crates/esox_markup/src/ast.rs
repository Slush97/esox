//! Abstract syntax tree for the markup language.

use std::collections::HashMap;
use std::fmt;

/// A single node in the UI tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Widget type (e.g. `button`, `card`, `row`).
    pub kind: WidgetKind,
    /// Optional variant after the dot (e.g. `primary` in `button.primary`).
    pub variant: Option<String>,
    /// Text content in quotes (e.g. `"Click me"` in `button "Click me"`).
    pub text: Option<String>,
    /// Key-value properties (e.g. `gap=16`, `placeholder="..."`).
    pub props: HashMap<String, Value>,
    /// Child nodes (determined by indentation).
    pub children: Vec<Node>,
    /// Source line number (1-based) for error reporting.
    pub line: usize,
}

impl Node {
    /// Get a property as a string, if it exists and is a string or identifier.
    pub fn prop_str(&self, key: &str) -> Option<&str> {
        match self.props.get(key)? {
            Value::String(s) | Value::Ident(s) => Some(s),
            _ => None,
        }
    }

    /// Get a property as an f64, if it exists and is a number.
    pub fn prop_f64(&self, key: &str) -> Option<f64> {
        match self.props.get(key)? {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get a property as an f32.
    pub fn prop_f32(&self, key: &str) -> Option<f32> {
        self.prop_f64(key).map(|n| n as f32)
    }

    /// Get a property as a bool.
    pub fn prop_bool(&self, key: &str) -> Option<bool> {
        match self.props.get(key)? {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get a property as a color (0xRRGGBB).
    pub fn prop_color(&self, key: &str) -> Option<u32> {
        match self.props.get(key)? {
            Value::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// Get a property as a string array.
    pub fn prop_string_array(&self, key: &str) -> Option<Vec<&str>> {
        match self.props.get(key)? {
            Value::Array(arr) => {
                let mut out = Vec::with_capacity(arr.len());
                for v in arr {
                    match v {
                        Value::String(s) | Value::Ident(s) => out.push(s.as_str()),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }

    /// Get a property as a number array.
    pub fn prop_number_array(&self, key: &str) -> Option<Vec<f64>> {
        match self.props.get(key)? {
            Value::Array(arr) => {
                let mut out = Vec::with_capacity(arr.len());
                for v in arr {
                    match v {
                        Value::Number(n) => out.push(*n),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }
}

/// Property value types.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A quoted string: `"hello world"`.
    String(String),
    /// A number: `16`, `0.5`, `-3`.
    Number(f64),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// A hex color: `#ff0000` → `0xFF0000`.
    Color(u32),
    /// An array: `[1, 2, 3]` or `["a", "b"]`.
    Array(Vec<Value>),
    /// A bare identifier: `center`, `xl`, `between`.
    Ident(String),
}

/// Known widget types.
///
/// The parser maps lowercase names to these variants. Unknown names become
/// [`WidgetKind::Custom`] so the format is extensible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetKind {
    // ── Layout ──────────────────────────────────────────────
    /// `page` — scrollable padded page. Props: `padding`, `max-width`, `height`.
    Page,
    /// `row` — horizontal layout. Props: `gap`, `align`, `justify`.
    Row,
    /// `col` / `column` — vertical layout. Props: `gap`.
    Column,
    /// `padding` / `pad` — padding container. Props: `amount` or top/right/bottom/left.
    Padding,
    /// `card` — elevated card surface. Props: `bg`.
    Card,
    /// `surface` — flat surface. Props: `bg`.
    Surface,
    /// `section` — titled section. Text = title.
    Section,
    /// `columns` / `cols` — weighted columns. Props: `weights=[1,2,1]`, `gap`.
    Columns,
    /// `scrollable` / `scroll` — scroll container. Props: `height`, `width`.
    /// Variants: `.horizontal`, `.2d`.
    Scrollable,
    /// `max-width` — constrain max width. Props: `width` or `value`.
    MaxWidth,
    /// `center` / `center-h` — center content horizontally. Props: `width`.
    CenterH,
    /// `flex` — flex layout builder. Variants: `.row` (default), `.col`.
    /// Props: `gap`, `align`, `justify`, `wrap`.
    Flex,
    /// `grid` — CSS-like grid. Props: `cols`, `rows`, `gap`, `col-gap`, `row-gap`.
    /// Track syntax: `cols="1fr 200 auto minmax(100,2fr)"`.
    Grid,
    /// `split` / `split-pane` — split pane. Variants: `.h` (default), `.v`.
    /// Props: `ratio`.
    SplitPane,
    /// `container` / `box` — generic container with full style control.
    /// Props: `bg`, `border-color`, `border-width`, `radius`, `padding`, `elevation`.
    Container,
    /// `clip` — clip children to bounds (overflow hidden).
    Clip,
    /// `labeled` — label + widget in a row. Text = label.
    Labeled,

    // ── Text ────────────────────────────────────────────────
    /// `label` / `text` — single-line text.
    /// Variants: `.muted`, `.header`, `.wrapped`, `.truncated`, `.colored`, `.sized`.
    /// Props: `size` (xs/sm/base/lg/xl/xxl), `color`, `truncation` (end/start/middle).
    Label,
    /// `heading` / `h` — heading text. Props: `size`.
    Heading,
    /// `paragraph` / `p` — word-wrapped paragraph. Text = content.
    Paragraph,
    /// `separator` / `hr` / `---` — horizontal rule.
    Separator,
    /// `space` / `gap` — explicit spacing. Props: `amount`.
    Space,
    /// `rich-text` — multi-span styled text. Children must be `span` nodes.
    /// Variants: `.wrapped`.
    RichText,
    /// `span` — a styled text segment within `rich-text`.
    /// Text = content. Props: `color`, `size`, `weight` (light/regular/medium/semibold/bold/extrabold),
    /// `letter-spacing`, `bold` (shorthand).
    Span,

    // ── Buttons ─────────────────────────────────────────────
    /// `button` / `btn` — button widget. Text = label.
    /// Variants: `.primary`, `.secondary`, `.danger`, `.ghost`, `.outlined`, `.text`, `.small`.
    /// Props: `action`, `max-width`, `bg`.
    Button,
    /// `link` / `hyperlink` — clickable hyperlink. Text = label. Props: `href`, `action`.
    Link,

    // ── Form inputs ─────────────────────────────────────────
    /// `input` — text input. Props: `placeholder`, `bind`.
    /// Variants: `.validated`. Props for validated: `status` (ok/error/warning).
    Input,
    /// `textarea` — multi-line text input. Props: `rows`, `placeholder`, `bind`.
    /// Variants: `.wrapped`.
    Textarea,
    /// `checkbox` — checkbox. Text = label. Props: `bind`.
    Checkbox,
    /// `toggle` / `switch` — toggle switch. Text = label. Props: `bind`.
    Toggle,
    /// `radio` — radio button. Text = label. Props: `value` (option index), `bind`.
    Radio,
    /// `slider` / `range` — slider. Props: `min`, `max`, `bind`, `unit`.
    /// Variants: `.labeled` (shows value + unit).
    Slider,
    /// `select` / `dropdown` — dropdown select. Props: `options=[...]`, `bind`.
    Select,
    /// `combobox` / `combo` — searchable combobox. Props: `options=[...]`, `bind`.
    Combobox,
    /// `number-input` / `number` — number input with +/- buttons.
    /// Props: `step`, `min`, `max`, `bind`.
    NumberInput,
    /// `field` / `form-field` — form field wrapper. Text = label.
    /// Props: `status` (ok/error/warning/none), `hint`.
    Field,
    /// `drop-zone` — file drop target. Props: `bind`.
    DropZone,

    // ── Display ─────────────────────────────────────────────
    /// `progress` / `progress-bar` — progress bar. Props: `value` (0.0-1.0), `color`.
    /// Variants: `.colored`, `.thresholds`.
    /// Thresholds variant: children are threshold stops (not yet supported).
    Progress,
    /// `spinner` / `loading` — loading spinner. Props: `size`.
    Spinner,
    /// `badge` — notification badge. Props: `count`, `bg`, `fg`.
    /// Variants: `.dot`, `.colored`.
    Badge,
    /// `avatar` — user avatar with initials. Text = initials. Props: `size`, `bg`.
    /// Variants: `.colored`.
    Avatar,
    /// `chip` / `tag` — chip/tag. Text = label. Props: `color`, `action`.
    Chip,
    /// `rating` / `stars` — star rating. Props: `value`, `max`, `bind`.
    /// Variants: `.display` (read-only).
    Rating,
    /// `skeleton` — loading skeleton placeholder. Props: `width`, `height`.
    /// Variants: `.text`, `.circle` (props: `diameter`).
    Skeleton,
    /// `image` / `img` — image display. Props: `src`, `width`, `height`.
    Image,
    /// `pill` / `status-pill` — status pill indicator. Text = label.
    /// Variants: `.success`, `.warning`, `.error`.
    /// Props: `bg`, `fg`.
    StatusPill,

    // ── Navigation ──────────────────────────────────────────
    /// `tabs` — tab bar + content. Props: `labels=[...]`, `bind`, `action`.
    Tabs,
    /// `breadcrumb` / `breadcrumbs` — navigation breadcrumb.
    /// Props: `segments=[...]`, `action`.
    Breadcrumb,
    /// `stepper` / `steps` — multi-step progress.
    /// Props: `labels=[...]`, `current`, `action`.
    Stepper,
    /// `pagination` — page navigation. Props: `total-pages`, `bind`, `action`.
    Pagination,

    // ── Feedback ────────────────────────────────────────────
    /// `alert` — alert banner. Text = message.
    /// Variants: `.info`, `.success`, `.warning`, `.error`, `.dismissable`.
    /// Props: `bg`, `accent`, `bind` (for dismissable visibility).
    Alert,
    /// `status-bar` / `statusbar` — bottom status bar. Props: `left`, `right`.
    StatusBar,
    /// `empty-state` / `empty` — empty state placeholder. Text = message.
    /// Variants: `.action` (props: `action-label`, `action`), `.rich`.
    EmptyState,

    // ── Overlays ────────────────────────────────────────────
    /// `modal` / `dialog` — modal dialog. Text = title. Props: `width`, `bind` (open state).
    Modal,
    /// `drawer` — side drawer. Props: `width`, `bind`.
    /// Variants: `.right`.
    Drawer,
    /// `popover` — anchored popover. Props: `bind`.
    Popover,

    // ── Complex ─────────────────────────────────────────────
    /// `table` — data table. Props: `rows`, `visible`, `bind`.
    /// Children: `table-column` nodes defining columns, then cell content via interpreter.
    Table,
    /// `table-column` — column definition within a table.
    /// Text = header label. Props: `width` (number=fixed, "Nfr"=weight, "auto"),
    /// `sortable` (default true).
    TableColumn,
    /// `tree` — tree view. Props: `bind`.
    Tree,
    /// `tree-node` — node within a tree. Text = label.
    /// Props: `has-children`, `bind`.
    TreeNode,
    /// `accordion` — accordion sections. Props: `sections=[...]`, `bind`.
    Accordion,
    /// `collapsing` / `collapse` — collapsing header. Text = label.
    /// Props: `open` (default open state).
    Collapsing,
    /// `virtual-scroll` — virtualized list. Props: `item-height`, `height`, `count`.
    VirtualScroll,

    // ── Menus ───────────────────────────────────────────────
    /// `menu-bar` — horizontal menu bar. Children: `menu` nodes.
    MenuBar,
    /// `menu` — a menu within a menu bar. Text = title. Children: `menu-item` nodes.
    Menu,
    /// `menu-item` — a menu item. Text = label. Props: `action`.
    MenuItem,

    // ── Style scope ─────────────────────────────────────────
    /// `style` — apply style overrides to children.
    /// Props: any WidgetStyle property (bg, fg, radius, opacity, etc.).
    Style,
    /// `disabled` — disable all children. Props: `value` (default true).
    Disabled,

    // ── Escape hatch ────────────────────────────────────────
    /// Unknown widget type — preserved as-is for custom interpreters.
    Custom(String),
}

impl WidgetKind {
    /// Parse a widget type name to a known variant, or `Custom`.
    pub fn from_name(name: &str) -> Self {
        match name {
            // Layout
            "page" => Self::Page,
            "row" => Self::Row,
            "col" | "column" => Self::Column,
            "padding" | "pad" => Self::Padding,
            "card" => Self::Card,
            "surface" => Self::Surface,
            "section" => Self::Section,
            "columns" | "cols" => Self::Columns,
            "scrollable" | "scroll" => Self::Scrollable,
            "max-width" | "maxwidth" => Self::MaxWidth,
            "center-h" | "centerh" | "center" => Self::CenterH,
            "flex" => Self::Flex,
            "grid" => Self::Grid,
            "split" | "split-pane" => Self::SplitPane,
            "container" | "box" => Self::Container,
            "clip" => Self::Clip,
            "labeled" => Self::Labeled,

            // Text
            "label" | "text" => Self::Label,
            "heading" | "h" => Self::Heading,
            "paragraph" | "p" => Self::Paragraph,
            "separator" | "hr" | "---" => Self::Separator,
            "space" | "gap" => Self::Space,
            "rich-text" | "richtext" | "rich" => Self::RichText,
            "span" => Self::Span,

            // Buttons
            "button" | "btn" => Self::Button,
            "link" | "hyperlink" => Self::Link,

            // Form inputs
            "input" => Self::Input,
            "textarea" => Self::Textarea,
            "checkbox" => Self::Checkbox,
            "toggle" | "switch" => Self::Toggle,
            "radio" => Self::Radio,
            "slider" | "range" => Self::Slider,
            "select" | "dropdown" => Self::Select,
            "combobox" | "combo" => Self::Combobox,
            "number-input" | "number" => Self::NumberInput,
            "field" | "form-field" => Self::Field,
            "drop-zone" | "dropzone" => Self::DropZone,

            // Display
            "progress" | "progress-bar" => Self::Progress,
            "spinner" | "loading" => Self::Spinner,
            "badge" => Self::Badge,
            "avatar" => Self::Avatar,
            "chip" | "tag" => Self::Chip,
            "rating" | "stars" => Self::Rating,
            "skeleton" => Self::Skeleton,
            "image" | "img" => Self::Image,
            "pill" | "status-pill" => Self::StatusPill,

            // Navigation
            "tabs" => Self::Tabs,
            "breadcrumb" | "breadcrumbs" => Self::Breadcrumb,
            "stepper" | "steps" => Self::Stepper,
            "pagination" | "pages" => Self::Pagination,

            // Feedback
            "alert" => Self::Alert,
            "status-bar" | "statusbar" => Self::StatusBar,
            "empty-state" | "empty" => Self::EmptyState,

            // Overlays
            "modal" | "dialog" => Self::Modal,
            "drawer" => Self::Drawer,
            "popover" => Self::Popover,

            // Complex
            "table" => Self::Table,
            "table-column" | "th" => Self::TableColumn,
            "tree" => Self::Tree,
            "tree-node" => Self::TreeNode,
            "accordion" => Self::Accordion,
            "collapsing" | "collapse" => Self::Collapsing,
            "virtual-scroll" | "vscroll" => Self::VirtualScroll,

            // Menus
            "menu-bar" | "menubar" => Self::MenuBar,
            "menu" => Self::Menu,
            "menu-item" => Self::MenuItem,

            // Style
            "style" => Self::Style,
            "disabled" => Self::Disabled,

            other => Self::Custom(other.to_string()),
        }
    }
}

impl fmt::Display for WidgetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Page => "page",
            Self::Row => "row",
            Self::Column => "col",
            Self::Padding => "padding",
            Self::Card => "card",
            Self::Surface => "surface",
            Self::Section => "section",
            Self::Columns => "columns",
            Self::Scrollable => "scrollable",
            Self::MaxWidth => "max-width",
            Self::CenterH => "center",
            Self::Flex => "flex",
            Self::Grid => "grid",
            Self::SplitPane => "split",
            Self::Container => "container",
            Self::Clip => "clip",
            Self::Labeled => "labeled",
            Self::Label => "label",
            Self::Heading => "heading",
            Self::Paragraph => "paragraph",
            Self::Separator => "separator",
            Self::Space => "space",
            Self::RichText => "rich-text",
            Self::Span => "span",
            Self::Button => "button",
            Self::Link => "link",
            Self::Input => "input",
            Self::Textarea => "textarea",
            Self::Checkbox => "checkbox",
            Self::Toggle => "toggle",
            Self::Radio => "radio",
            Self::Slider => "slider",
            Self::Select => "select",
            Self::Combobox => "combobox",
            Self::NumberInput => "number-input",
            Self::Field => "field",
            Self::DropZone => "drop-zone",
            Self::Progress => "progress",
            Self::Spinner => "spinner",
            Self::Badge => "badge",
            Self::Avatar => "avatar",
            Self::Chip => "chip",
            Self::Rating => "rating",
            Self::Skeleton => "skeleton",
            Self::Image => "image",
            Self::StatusPill => "pill",
            Self::Tabs => "tabs",
            Self::Breadcrumb => "breadcrumb",
            Self::Stepper => "stepper",
            Self::Pagination => "pagination",
            Self::Alert => "alert",
            Self::StatusBar => "status-bar",
            Self::EmptyState => "empty-state",
            Self::Modal => "modal",
            Self::Drawer => "drawer",
            Self::Popover => "popover",
            Self::Table => "table",
            Self::TableColumn => "table-column",
            Self::Tree => "tree",
            Self::TreeNode => "tree-node",
            Self::Accordion => "accordion",
            Self::Collapsing => "collapsing",
            Self::VirtualScroll => "virtual-scroll",
            Self::MenuBar => "menu-bar",
            Self::Menu => "menu",
            Self::MenuItem => "menu-item",
            Self::Style => "style",
            Self::Disabled => "disabled",
            Self::Custom(name) => name,
        };
        write!(f, "{s}")
    }
}
