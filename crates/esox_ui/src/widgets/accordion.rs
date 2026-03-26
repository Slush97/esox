//! Accordion widget — mutually-exclusive collapsible sections.
//!
//! # Examples
//!
//! ```ignore
//! ui.accordion(id!("faq"), &["General", "Billing", "Support"], &mut open, |ui, i| {
//!     match i {
//!         0 => ui.label("General FAQ content..."),
//!         1 => ui.label("Billing FAQ content..."),
//!         2 => ui.label("Support FAQ content..."),
//!         _ => {}
//!     }
//! });
//! ```

use crate::id::fnv1a_mix;
use crate::paint;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw an accordion with mutually-exclusive sections.
    ///
    /// Only one section can be open at a time. `open_index` tracks which
    /// section is open (`None` = all closed). The `content` closure is called
    /// for the currently open section.
    pub fn accordion(
        &mut self,
        id: u64,
        sections: &[&str],
        open_index: &mut Option<usize>,
        mut content: impl FnMut(&mut Self, usize),
    ) {
        for (i, &title) in sections.iter().enumerate() {
            let section_id = fnv1a_mix(id, i as u64);
            let is_open = *open_index == Some(i);

            // Sync collapsing state with our exclusive-open logic.
            if is_open {
                self.state.collapsing_open.insert(section_id);
            } else {
                self.state.collapsing_open.remove(&section_id);
            }

            let response = self.collapsing_header(section_id, title, is_open, |ui| {
                content(ui, i);
            });

            // If this section was just toggled open, close all others.
            if response.clicked {
                let now_open = self.state.collapsing_open.contains(&section_id);
                if now_open {
                    *open_index = Some(i);
                    // Close all others.
                    for (j, _) in sections.iter().enumerate() {
                        if j != i {
                            let other_id = fnv1a_mix(id, j as u64);
                            self.state.collapsing_open.remove(&other_id);
                        }
                    }
                } else {
                    *open_index = None;
                }
            }

            // Draw separator between sections (not after the last one).
            if i + 1 < sections.len() {
                let sep_rect = self.allocate_rect(self.region.w, 1.0);
                paint::draw_rounded_rect(self.frame, sep_rect, self.theme.border, 0.0);
            }
        }
    }
}
