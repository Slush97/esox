//! Persistent state store for markup-driven UI.

use std::collections::HashMap;

use esox_gfx::Color;

use crate::state::{InputState, SelectState, TabState, TableState, TreeState, VirtualScrollState};
use crate::widgets::pagination::PaginationState;

/// Persistent state for markup-driven UI. Create once, reuse across frames.
///
/// Widgets that need mutable state (text inputs, checkboxes, tabs, etc.) store
/// their state here, keyed by the `bind=name` property from the markup. Widgets
/// without an explicit `bind` get auto-generated keys based on tree position.
///
/// The host app can read and write state by bind name to programmatically
/// control the UI or react to changes.
#[derive(Default)]
pub struct MarkupState {
    /// Text inputs, text areas, checkboxes, toggles, radios, sliders.
    pub(crate) inputs: HashMap<String, InputState>,
    /// Select dropdowns.
    pub(crate) selects: HashMap<String, SelectState>,
    /// Tab bars.
    pub(crate) tabs: HashMap<String, TabState>,
    /// Data tables.
    pub(crate) tables: HashMap<String, TableState>,
    /// Tree views.
    pub(crate) trees: HashMap<String, TreeState>,
    /// Virtual scroll views.
    pub(crate) vscrolls: HashMap<String, VirtualScrollState>,
    /// Pagination controls.
    pub(crate) paginations: HashMap<String, PaginationState>,
    /// Bool state: modal/drawer/popover open, dismissable alert visible.
    pub(crate) bools: HashMap<String, bool>,
    /// Float state: number_input values.
    pub(crate) floats: HashMap<String, f64>,
    /// Rating values (u8).
    pub(crate) u8s: HashMap<String, u8>,
    /// Accordion open section index.
    pub(crate) accordion_open: HashMap<String, Option<usize>>,
    /// Combobox selected index.
    pub(crate) comboboxes: HashMap<String, Option<usize>>,
    /// Previous frame's color values for transition animation.
    /// Keyed by "{widget_id}_{property}" to track when colors change.
    pub(crate) prev_colors: HashMap<String, Color>,
}

impl MarkupState {
    /// Create an empty state store.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Text (InputState.text) ──────────────────────────────────────

    /// Get the text content of an input widget by bind name.
    pub fn get_text(&self, bind: &str) -> Option<&str> {
        self.inputs.get(bind).map(|s| s.text.as_str())
    }

    /// Set the text content of an input widget by bind name.
    pub fn set_text(&mut self, bind: &str, text: &str) {
        self.inputs.entry(bind.to_string()).or_default().text = text.to_string();
    }

    // ── Bool (checkbox/toggle via InputState, or overlay open) ──────

    /// Get a boolean value. Checks `bools` map first, then `inputs` (for
    /// checkbox/toggle where state is stored as `"true"`/`"false"` text).
    pub fn get_bool(&self, bind: &str) -> Option<bool> {
        if let Some(b) = self.bools.get(bind) {
            return Some(*b);
        }
        self.inputs.get(bind).map(|s| s.text == "true")
    }

    /// Set a boolean value in the `bools` map (for modal/drawer open state).
    pub fn set_bool(&mut self, bind: &str, val: bool) {
        self.bools.insert(bind.to_string(), val);
    }

    /// Set a checkbox/toggle value via InputState.
    pub fn set_checked(&mut self, bind: &str, checked: bool) {
        self.inputs.entry(bind.to_string()).or_default().text =
            if checked { "true" } else { "false" }.to_string();
    }

    // ── Float (number_input) ────────────────────────────────────────

    /// Get a float value by bind name.
    pub fn get_f64(&self, bind: &str) -> Option<f64> {
        self.floats.get(bind).copied()
    }

    /// Set a float value by bind name.
    pub fn set_f64(&mut self, bind: &str, val: f64) {
        self.floats.insert(bind.to_string(), val);
    }

    // ── Rating (u8) ─────────────────────────────────────────────────

    /// Get a rating value by bind name.
    pub fn get_u8(&self, bind: &str) -> Option<u8> {
        self.u8s.get(bind).copied()
    }

    /// Set a rating value by bind name.
    pub fn set_u8(&mut self, bind: &str, val: u8) {
        self.u8s.insert(bind.to_string(), val);
    }

    // ── Select (SelectState.selected_index) ─────────────────────────

    /// Get the selected index of a select or combobox widget.
    pub fn get_selected(&self, bind: &str) -> Option<usize> {
        if let Some(s) = self.selects.get(bind) {
            return Some(s.selected_index);
        }
        self.comboboxes.get(bind).and_then(|o| *o)
    }

    /// Set the selected index of a select widget.
    pub fn set_selected(&mut self, bind: &str, idx: usize) {
        self.selects
            .entry(bind.to_string())
            .or_default()
            .selected_index = idx;
    }

    // ── Tabs ────────────────────────────────────────────────────────

    /// Get the active tab index.
    pub fn get_tab(&self, bind: &str) -> Option<usize> {
        self.tabs.get(bind).map(|t| t.selected)
    }

    /// Set the active tab index.
    pub fn set_tab(&mut self, bind: &str, idx: usize) {
        self.tabs.entry(bind.to_string()).or_default().selected = idx;
    }

    // ── Pagination ──────────────────────────────────────────────────

    /// Get the current page (0-indexed).
    pub fn get_page(&self, bind: &str) -> Option<usize> {
        self.paginations.get(bind).map(|p| p.current_page)
    }

    /// Set the current page (0-indexed).
    pub fn set_page(&mut self, bind: &str, page: usize) {
        self.paginations
            .entry(bind.to_string())
            .or_default()
            .current_page = page;
    }

    // ── Accordion ───────────────────────────────────────────────────

    /// Get the open accordion section index.
    pub fn get_accordion_open(&self, bind: &str) -> Option<Option<usize>> {
        self.accordion_open.get(bind).copied()
    }

    /// Set the open accordion section index.
    pub fn set_accordion_open(&mut self, bind: &str, section: Option<usize>) {
        self.accordion_open.insert(bind.to_string(), section);
    }

    // ── Table ───────────────────────────────────────────────────────

    /// Get the selected row in a table.
    pub fn get_selected_row(&self, bind: &str) -> Option<Option<usize>> {
        self.tables.get(bind).map(|t| t.selected_row)
    }

    // ── Utility ─────────────────────────────────────────────────────

    /// Clear all stored state.
    pub fn clear(&mut self) {
        self.inputs.clear();
        self.selects.clear();
        self.tabs.clear();
        self.tables.clear();
        self.trees.clear();
        self.vscrolls.clear();
        self.paginations.clear();
        self.bools.clear();
        self.floats.clear();
        self.u8s.clear();
        self.accordion_open.clear();
        self.comboboxes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_get_set() {
        let mut s = MarkupState::new();
        assert_eq!(s.get_text("name"), None);
        s.set_text("name", "Alice");
        assert_eq!(s.get_text("name"), Some("Alice"));
        s.set_text("name", "Bob");
        assert_eq!(s.get_text("name"), Some("Bob"));
    }

    #[test]
    fn bool_via_bools_map() {
        let mut s = MarkupState::new();
        assert_eq!(s.get_bool("modal"), None);
        s.set_bool("modal", true);
        assert_eq!(s.get_bool("modal"), Some(true));
        s.set_bool("modal", false);
        assert_eq!(s.get_bool("modal"), Some(false));
    }

    #[test]
    fn bool_via_checkbox() {
        let mut s = MarkupState::new();
        s.set_checked("dark_mode", true);
        assert_eq!(s.get_bool("dark_mode"), Some(true));
        s.set_checked("dark_mode", false);
        assert_eq!(s.get_bool("dark_mode"), Some(false));
    }

    #[test]
    fn float_get_set() {
        let mut s = MarkupState::new();
        s.set_f64("volume", 0.75);
        assert_eq!(s.get_f64("volume"), Some(0.75));
    }

    #[test]
    fn select_get_set() {
        let mut s = MarkupState::new();
        s.set_selected("role", 2);
        assert_eq!(s.get_selected("role"), Some(2));
    }

    #[test]
    fn tab_get_set() {
        let mut s = MarkupState::new();
        s.set_tab("main", 1);
        assert_eq!(s.get_tab("main"), Some(1));
    }

    #[test]
    fn page_get_set() {
        let mut s = MarkupState::new();
        s.set_page("results", 3);
        assert_eq!(s.get_page("results"), Some(3));
    }

    #[test]
    fn u8_get_set() {
        let mut s = MarkupState::new();
        s.set_u8("stars", 4);
        assert_eq!(s.get_u8("stars"), Some(4));
    }

    #[test]
    fn clear_removes_all() {
        let mut s = MarkupState::new();
        s.set_text("a", "x");
        s.set_bool("b", true);
        s.set_f64("c", 1.0);
        s.clear();
        assert_eq!(s.get_text("a"), None);
        assert_eq!(s.get_bool("b"), None);
        assert_eq!(s.get_f64("c"), None);
    }
}
