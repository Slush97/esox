//! Deterministic widget ID generation from tree position.

use crate::id::{fnv1a_mix, fnv1a_runtime};

/// Constant seed so interpreter-generated IDs live in a distinct namespace
/// from compile-time `id!()` values.
const INTERPRET_SEED: u64 = 0x494E_5445_5250_5254;

/// Tracks position in the node tree for deterministic, unique ID generation.
pub(crate) struct IdCtx {
    stack: Vec<u64>,
}

impl IdCtx {
    pub fn new() -> Self {
        Self {
            stack: vec![INTERPRET_SEED],
        }
    }

    /// Generate a widget ID.
    ///
    /// - If `bind` is `Some`, hash the bind name for a stable, name-based ID.
    /// - Otherwise, derive from parent ID + child index for positional stability.
    pub fn widget_id(&self, bind: Option<&str>, child_index: usize) -> u64 {
        match bind {
            Some(name) => fnv1a_runtime(name),
            None => fnv1a_mix(self.parent_id(), child_index as u64),
        }
    }

    /// Generate a state key string for `MarkupState` lookups.
    ///
    /// - Named: returns the bind name directly.
    /// - Unnamed: returns `"__auto_{parent_id}_{index}"`.
    pub fn state_key(&self, bind: Option<&str>, child_index: usize) -> String {
        match bind {
            Some(name) => name.to_string(),
            None => format!("__auto_{}_{}", self.parent_id(), child_index),
        }
    }

    /// Current parent ID (top of the stack).
    pub fn parent_id(&self) -> u64 {
        *self.stack.last().unwrap_or(&INTERPRET_SEED)
    }

    /// Push a new container context. Call before rendering children.
    pub fn push(&mut self, id: u64) {
        self.stack.push(id);
    }

    /// Pop the container context. Call after rendering children.
    pub fn pop(&mut self) {
        self.stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_name_produces_stable_id() {
        let ctx = IdCtx::new();
        let id1 = ctx.widget_id(Some("username"), 0);
        let id2 = ctx.widget_id(Some("username"), 5); // index ignored for named
        assert_eq!(id1, id2);
        assert_eq!(id1, fnv1a_runtime("username"));
    }

    #[test]
    fn positional_id_is_deterministic() {
        let ctx = IdCtx::new();
        let id1 = ctx.widget_id(None, 3);
        let id2 = ctx.widget_id(None, 3);
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_positions_produce_different_ids() {
        let ctx = IdCtx::new();
        let id0 = ctx.widget_id(None, 0);
        let id1 = ctx.widget_id(None, 1);
        let id2 = ctx.widget_id(None, 2);
        assert_ne!(id0, id1);
        assert_ne!(id1, id2);
        assert_ne!(id0, id2);
    }

    #[test]
    fn nested_context_changes_child_ids() {
        let mut ctx = IdCtx::new();
        let outer_id = ctx.widget_id(None, 0);

        ctx.push(outer_id);
        let inner_id = ctx.widget_id(None, 0);
        ctx.pop();

        // Same child_index=0 but different parent → different ID
        assert_ne!(outer_id, inner_id);
    }

    #[test]
    fn state_key_named() {
        let ctx = IdCtx::new();
        assert_eq!(ctx.state_key(Some("email"), 0), "email");
    }

    #[test]
    fn state_key_auto() {
        let ctx = IdCtx::new();
        let key = ctx.state_key(None, 7);
        assert!(key.starts_with("__auto_"));
        assert!(key.contains("_7"));
    }

    #[test]
    fn push_pop_restores_parent() {
        let mut ctx = IdCtx::new();
        let root = ctx.parent_id();
        ctx.push(42);
        assert_eq!(ctx.parent_id(), 42);
        ctx.pop();
        assert_eq!(ctx.parent_id(), root);
    }
}
