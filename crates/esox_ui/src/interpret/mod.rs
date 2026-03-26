//! Markup interpreter — renders parsed `esox_markup` AST via `Ui` method calls.
//!
//! # Usage
//!
//! ```ignore
//! use esox_markup::parse;
//! use esox_ui::interpret::{render, MarkupState};
//!
//! // Parse once (or when markup changes).
//! let nodes = parse(markup_text).unwrap();
//!
//! // Create state store once.
//! let mut markup_state = MarkupState::new();
//!
//! // Each frame, between Ui::begin() and Ui::finish():
//! let actions = render(&mut ui, &nodes, &mut markup_state);
//! for action in &actions {
//!     match action.name.as_str() {
//!         "save" => { /* handle save */ }
//!         _ => {}
//!     }
//! }
//! ```

mod id_gen;
mod render;
mod resolve;
mod state;

pub use state::MarkupState;

use esox_markup::Node;

/// A named interaction fired by a widget during rendering.
#[derive(Debug, Clone)]
pub struct Action {
    /// The action name from `action=name` in markup.
    pub name: String,
    /// The bind name (or auto-generated key) of the source widget.
    pub source: String,
    /// What kind of interaction triggered the action.
    pub kind: ActionKind,
}

/// The type of interaction that triggered an [`Action`].
#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    /// Button click, link click, chip click, menu item selection.
    Click,
    /// Value changed (input, slider, checkbox, toggle, select, etc.).
    Change,
    /// A specific index was selected (tab, breadcrumb, stepper).
    Select(usize),
    /// Overlay dismissed (modal, drawer, popover, dismissable alert).
    Dismiss,
}

/// Render a parsed markup tree. Call between `Ui::begin()` and `Ui::finish()`.
///
/// Returns a list of actions fired by widgets during this frame (button clicks,
/// input changes, etc.). The host app maps action names to behavior.
pub fn render(ui: &mut crate::Ui<'_>, nodes: &[Node], state: &mut MarkupState) -> Vec<Action> {
    let mut actions = Vec::new();
    let mut id_ctx = id_gen::IdCtx::new();
    render::render_nodes(ui, nodes, state, &mut id_ctx, &mut actions);
    actions
}
