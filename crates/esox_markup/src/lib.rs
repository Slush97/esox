//! `esox_markup` — An indentation-based UI markup language.
//!
//! Think "markdown, but for UI." A minimal, human-readable, AI-friendly format
//! that describes widget trees declaratively.
//!
//! ## Example
//!
//! ```text
//! page padding=24
//!   heading "User Settings" size=xl
//!
//!   card
//!     field "Name"
//!       input placeholder="Enter your name"
//!     field "Email"
//!       input placeholder="user@example.com"
//!
//!     row gap=16 justify=end
//!       button.secondary "Cancel"
//!       button.primary "Save"
//! ```
//!
//! ## Format rules
//!
//! - **Indentation** defines hierarchy (2-space recommended, any consistent amount works)
//! - **First word** is the widget type, optionally with `.variant` suffix
//! - **Quoted string** after the type is the text content
//! - **`key=value`** pairs are properties (strings, numbers, bools, colors, arrays, identifiers)
//! - **Blank lines** are ignored (use them for readability)
//! - **`//`** starts a line comment

pub mod ast;
mod parser;

pub use ast::{Node, Value, WidgetKind};
pub use parser::{ParseError, parse};
