//! `esox_input` — Platform-independent input types.
//!
//! Pure data types with no windowing dependency. Used by `esox_ui`
//! and `esox_platform` to decouple input handling from the windowing
//! backend.

pub use smol_str::SmolStr;

/// A finite position in the logical coordinate space of one window viewport.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LogicalPosition {
    pub x: f32,
    pub y: f32,
}

impl LogicalPosition {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A finite two-axis delta in logical viewport coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LogicalDelta {
    pub x: f32,
    pub y: f32,
}

impl LogicalDelta {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// The validated logical extent of one window's drawable viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalViewport {
    pub width: f32,
    pub height: f32,
}

impl LogicalViewport {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn is_valid(self) -> bool {
        self.width.is_finite() && self.height.is_finite() && self.width > 0.0 && self.height > 0.0
    }
}

/// Pointer action captured in logical coordinates at the platform boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPhase {
    Move,
    Press { button: u8 },
    Release { button: u8 },
}

/// Renderer-neutral pointer input captured for one window at event time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    pub phase: PointerPhase,
    pub position: LogicalPosition,
}

/// A normalized two-axis wheel delta.
///
/// Positive components request increasing scroll offsets. The platform adapter
/// is responsible for converting device units and native direction before this
/// event reaches a UI frame owner.
pub type WheelDelta = LogicalDelta;

/// A wheel position in the logical coordinate space of a committed scene.
pub type WheelPosition = LogicalPosition;

/// Renderer-neutral wheel input captured for one window at event time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelEvent {
    pub position: WheelPosition,
    pub delta: WheelDelta,
}

/// A logical key value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    /// A named key (Enter, Tab, arrows, etc.).
    Named(NamedKey),
    /// A character produced by the key.
    Character(SmolStr),
    /// An unidentified key.
    Unidentified,
}

/// Named (non-character) keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// Physical key codes (scan codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    Enter,
    Tab,
    Backspace,
    Delete,
    Escape,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,
    /// An unmapped or unrecognized physical key.
    Unknown,
}

/// Modifier key state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const EMPTY: Modifiers = Modifiers(0);

    const SHIFT: u8 = 1;
    const CTRL: u8 = 2;
    const ALT: u8 = 4;
    const SUPER: u8 = 8;

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn shift(self) -> bool {
        self.0 & Self::SHIFT != 0
    }

    pub fn ctrl(self) -> bool {
        self.0 & Self::CTRL != 0
    }

    pub fn alt(self) -> bool {
        self.0 & Self::ALT != 0
    }

    pub fn super_key(self) -> bool {
        self.0 & Self::SUPER != 0
    }

    pub fn with_shift(mut self) -> Self {
        self.0 |= Self::SHIFT;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.0 |= Self::CTRL;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.0 |= Self::ALT;
        self
    }

    pub fn with_super(mut self) -> Self {
        self.0 |= Self::SUPER;
        self
    }

    /// Build from individual flags.
    pub fn from_flags(shift: bool, ctrl: bool, alt: bool, super_key: bool) -> Self {
        let mut bits = 0u8;
        if shift {
            bits |= Self::SHIFT;
        }
        if ctrl {
            bits |= Self::CTRL;
        }
        if alt {
            bits |= Self::ALT;
        }
        if super_key {
            bits |= Self::SUPER;
        }
        Self(bits)
    }
}

/// A keyboard event.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// The logical key value.
    pub key: Key,
    /// The physical key code.
    pub physical_key: KeyCode,
    /// Whether the key is pressed (true) or released (false).
    pub pressed: bool,
    /// Whether this is a key repeat.
    pub repeat: bool,
    /// Text produced by this key event (after modifier processing).
    pub text: Option<SmolStr>,
}

/// Mouse cursor icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    Default,
    Text,
    Pointer,
    Grab,
    Grabbing,
    ColResize,
    RowResize,
    NotAllowed,
    Crosshair,
    Move,
    NResize,
    SResize,
    EResize,
    WResize,
    NeResize,
    NwResize,
    SeResize,
    SwResize,
    Wait,
    Progress,
    Help,
    ZoomIn,
    ZoomOut,
    Copy,
}

#[cfg(test)]
mod tests {
    use super::{LogicalDelta, LogicalPosition, LogicalViewport};

    #[test]
    fn logical_coordinate_values_validate_without_sentinels() {
        assert!(LogicalPosition::new(0.0, 0.0).is_finite());
        assert!(!LogicalPosition::new(f32::NAN, 0.0).is_finite());
        assert!(LogicalDelta::new(-1.0, 2.0).is_finite());
        assert!(!LogicalDelta::new(0.0, f32::INFINITY).is_finite());
    }

    #[test]
    fn logical_viewport_requires_finite_positive_extents() {
        assert!(LogicalViewport::new(800.0, 600.0).is_valid());
        assert!(!LogicalViewport::new(0.0, 600.0).is_valid());
        assert!(!LogicalViewport::new(800.0, f32::NAN).is_valid());
    }
}
