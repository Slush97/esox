//! Widget showcase — demonstrates the typed widget APIs and nested scrollable layout.
//!
//! Every interactive widget uses the new primary API that takes `&mut T` directly
//! (e.g. `checkbox(&mut bool)`, `slider(&mut f32)`, `select(&mut usize)`).
//! A live state readout panel on the right shows all values updating in real time.
//! The left panel uses nested scrollables to exercise the layout consolidation.

use esox_gfx::{Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::{
    ClipboardProvider, GridPlacement, GridTrack, Rect, TextRenderer, Theme, ThemeTransition,
    UiState, id,
};

struct PlatformClipboard;

impl ClipboardProvider for PlatformClipboard {
    fn read_text(&self) -> Option<String> {
        Clipboard::read(0).ok()
    }
    fn write_text(&self, text: &str) {
        let _ = Clipboard::write(text);
    }
}

struct WidgetShowcase {
    ui_state: UiState,
    text: Option<TextRenderer>,
    viewport: (u32, u32),

    // Theme
    is_dark: bool,
    base_light: Theme,
    base_dark: Theme,
    theme: Theme,
    transition: Option<ThemeTransition>,

    // Widget state — all typed
    check_terms: bool,
    check_newsletter: bool,
    dark_toggle: bool,
    notifications_toggle: bool,
    volume: f32,
    brightness: f32,
    radio_choice: usize,
    select_color: usize,
    tab_active: usize,
    combo_fruit: Option<usize>,
    accordion_open: Option<usize>,
    page: usize,
}

impl WidgetShowcase {
    fn new() -> Self {
        let base_light = Theme::light();
        let base_dark = Theme::dark();
        let theme = base_dark.clone();

        let mut ui_state = UiState::new();
        ui_state.clipboard = Some(Box::new(PlatformClipboard));

        Self {
            ui_state,
            text: None,
            viewport: (1100, 750),
            is_dark: true,
            base_light,
            base_dark,
            theme,
            transition: None,
            check_terms: false,
            check_newsletter: true,
            dark_toggle: true,
            notifications_toggle: false,
            volume: 75.0,
            brightness: 0.5,
            radio_choice: 0,
            select_color: 0,
            tab_active: 0,
            combo_fruit: None,
            accordion_open: Some(0),
            page: 0,
        }
    }
}

impl AppDelegate for WidgetShowcase {
    fn on_init(&mut self, gpu: &GpuContext, _resources: &mut RenderResources) {
        match TextRenderer::new(gpu) {
            Ok(tr) => self.text = Some(tr),
            Err(e) => eprintln!("failed to initialize text renderer: {e}"),
        }
    }

    fn on_redraw(
        &mut self,
        gpu: &GpuContext,
        resources: &mut RenderResources,
        frame: &mut Frame,
        _perf: &esox_platform::perf::PerfMonitor,
    ) {
        if let Some(ref t) = self.transition {
            self.theme = t.current();
            if t.is_done() {
                self.theme = if self.is_dark {
                    self.base_dark.clone()
                } else {
                    self.base_light.clone()
                };
                self.transition = None;
            }
        }

        self.ui_state.update_blink(self.theme.cursor_blink_ms);

        let text = self.text.as_mut().unwrap();
        let vp = Rect::new(0.0, 0.0, self.viewport.0 as f32, self.viewport.1 as f32);
        let mut ui = esox_ui::Ui::begin(
            frame,
            gpu,
            resources,
            text,
            &mut self.ui_state,
            &self.theme,
            vp,
        );

        let scroll_h = self.viewport.1 as f32;
        let vp_w = self.viewport.0 as f32;

        // Two-column layout using grid: widgets on left, state readout on right.
        let state_panel_w = 280.0;
        let widgets_w = vp_w - state_panel_w - 24.0;

        ui.padding(12.0, |ui| {
            // ── Header ──
            ui.surface(|ui| {
                ui.row(|ui| {
                    ui.heading("Widget API Showcase");
                    ui.fill_space(100.0);
                    let btn_label = if self.is_dark { "Light Mode" } else { "Dark Mode" };
                    let btn_bg = ui.theme().secondary_button_bg;
                    if ui.small_button(id!("theme"), btn_label, btn_bg).clicked {
                        self.is_dark = !self.is_dark;
                        let target = if self.is_dark {
                            self.base_dark.clone()
                        } else {
                            self.base_light.clone()
                        };
                        self.transition =
                            Some(ThemeTransition::new(self.theme.clone(), target, 300.0));
                    }
                });
            });

            ui.add_space(8.0);

            ui.grid(
                &[GridTrack::Fixed(widgets_w), GridTrack::Fr(1.0)],
                &[GridTrack::Fixed(scroll_h - 80.0)],
            )
            .gap(12.0)
            .show(|grid| {
                // ═══════════════════════════════════════════════
                // Left column — widgets in a nested scrollable
                // ═══════════════════════════════════════════════
                grid.cell(GridPlacement::at(0, 0), |ui| {
                    let panel_h = scroll_h - 80.0;
                    ui.scrollable(id!("widget_scroll"), panel_h, |ui| {
                        // ── Checkboxes & Toggles ──
                        ui.card(|ui| {
                            ui.header_label("CHECKBOXES & TOGGLES");
                            ui.add_space(4.0);
                            ui.muted_label("checkbox(&mut bool)  /  toggle(&mut bool)");
                            ui.add_space(8.0);

                            ui.checkbox(id!("check_terms"), &mut self.check_terms, "Accept terms");
                            ui.add_space(4.0);
                            ui.checkbox(
                                id!("check_news"),
                                &mut self.check_newsletter,
                                "Subscribe to newsletter",
                            );
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);
                            ui.toggle(id!("dark_toggle"), &mut self.dark_toggle, "Dark mode");
                            ui.add_space(4.0);
                            ui.toggle(
                                id!("notif_toggle"),
                                &mut self.notifications_toggle,
                                "Notifications",
                            );
                        });

                        ui.add_space(12.0);

                        // ── Sliders ──
                        ui.card(|ui| {
                            ui.header_label("SLIDERS");
                            ui.add_space(4.0);
                            ui.muted_label("slider(&mut f32, min, max)");
                            ui.add_space(8.0);

                            ui.label("Volume");
                            ui.slider(id!("volume"), &mut self.volume, 0.0, 100.0);
                            ui.add_space(8.0);
                            ui.label("Brightness");
                            ui.slider(id!("brightness"), &mut self.brightness, 0.0, 1.0);
                        });

                        ui.add_space(12.0);

                        // ── Radio Buttons ──
                        ui.card(|ui| {
                            ui.header_label("RADIO BUTTONS");
                            ui.add_space(4.0);
                            ui.muted_label("radio(&mut usize, option_index, label)");
                            ui.add_space(8.0);

                            ui.radio(id!("radio"), &mut self.radio_choice, 0, "Small");
                            ui.add_space(4.0);
                            ui.radio(id!("radio"), &mut self.radio_choice, 1, "Medium");
                            ui.add_space(4.0);
                            ui.radio(id!("radio"), &mut self.radio_choice, 2, "Large");
                        });

                        ui.add_space(12.0);

                        // ── Select & Combobox ──
                        ui.card(|ui| {
                            ui.header_label("SELECT & COMBOBOX");
                            ui.add_space(4.0);
                            ui.muted_label("select(&mut usize)  /  combobox(&mut Option<usize>)");
                            ui.add_space(8.0);

                            ui.label("Favorite color");
                            ui.select(
                                id!("color"),
                                &mut self.select_color,
                                &["Red", "Green", "Blue", "Yellow", "Purple"],
                            );
                            ui.add_space(8.0);
                            ui.label("Fruit");
                            ui.combobox(
                                id!("fruit"),
                                &mut self.combo_fruit,
                                &["Apple", "Banana", "Cherry", "Date", "Elderberry"],
                            );
                        });

                        ui.add_space(12.0);

                        // ── Tabs ──
                        ui.card(|ui| {
                            ui.header_label("TABS");
                            ui.add_space(4.0);
                            ui.muted_label("tabs(&mut usize, labels, content_fn)");
                            ui.add_space(8.0);

                            ui.tabs(
                                id!("demo_tabs"),
                                &mut self.tab_active,
                                &["Overview", "Details", "Settings"],
                                |ui, tab| {
                                    ui.padding(8.0, |ui| match tab {
                                        0 => ui.label("Overview: a summary of the project."),
                                        1 => ui.label("Details: in-depth technical info."),
                                        2 => ui.label("Settings: configure preferences here."),
                                        _ => {}
                                    });
                                },
                            );
                        });

                        ui.add_space(12.0);

                        // ── Accordion ──
                        ui.card(|ui| {
                            ui.header_label("ACCORDION");
                            ui.add_space(4.0);
                            ui.muted_label("accordion(&mut Option<usize>, sections, content_fn)");
                            ui.add_space(8.0);

                            ui.accordion(
                                id!("faq"),
                                &mut self.accordion_open,
                                &["What is Esox?", "How does layout work?", "What about a11y?"],
                                |ui, i| {
                                    match i {
                                        0 => {
                                            ui.paragraph(
                                                id!("acc0"),
                                                "Esox is a GPU-accelerated immediate-mode UI toolkit for \
                                                 native Linux apps, written in Rust.",
                                            );
                                        }
                                        1 => {
                                            ui.paragraph(
                                                id!("acc1"),
                                                "Layout uses a two-pass tree solver: measure bottom-up, \
                                                 then arrange top-down. The tree stores scroll-relative \
                                                 positions for correct nested scrollable behavior.",
                                            );
                                        }
                                        2 => {
                                            ui.paragraph(
                                                id!("acc2"),
                                                "Esox includes an AT-SPI2 bridge so screen readers can \
                                                 interact with all widgets. Keyboard nav is built in.",
                                            );
                                        }
                                        _ => {}
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        // ── Pagination ──
                        ui.card(|ui| {
                            ui.header_label("PAGINATION");
                            ui.add_space(4.0);
                            ui.muted_label("pagination(&mut usize, total_pages)");
                            ui.add_space(8.0);

                            ui.pagination(id!("pager"), &mut self.page, 10);
                            ui.add_space(4.0);
                            let page_label = format!("Showing results for page {}", self.page + 1);
                            ui.muted_label(&page_label);
                        });

                        ui.add_space(12.0);

                        // ── Nested Scrollable ──
                        ui.card(|ui| {
                            ui.header_label("NESTED SCROLLABLE");
                            ui.add_space(4.0);
                            ui.muted_label(
                                "Scroll container inside a scroll container — positions \
                                 resolve correctly via the ancestor chain.",
                            );
                            ui.add_space(8.0);

                            ui.scrollable(id!("inner_scroll"), 150.0, |ui| {
                                for i in 0..20 {
                                    let label = format!("Inner item {i}");
                                    ui.label(&label);
                                    ui.add_space(4.0);
                                }
                            });
                        });

                        ui.add_space(24.0);
                    }); // outer scrollable
                });

                // ═══════════════════════════════════════════════
                // Right column — live state readout
                // ═══════════════════════════════════════════════
                grid.cell(GridPlacement::at(1, 0), |ui| {
                    ui.card(|ui| {
                        ui.header_label("LIVE STATE");
                        ui.add_space(4.0);
                        ui.muted_label("All values update in real time");
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let accent = ui.theme().accent;
                        let green = ui.theme().green;

                        // Bools
                        ui.label("Checkboxes");
                        let v = format!(
                            "  terms: {}  newsletter: {}",
                            self.check_terms, self.check_newsletter
                        );
                        ui.label_colored(&v, accent);
                        ui.add_space(6.0);

                        ui.label("Toggles");
                        let v = format!(
                            "  dark: {}  notifications: {}",
                            self.dark_toggle, self.notifications_toggle
                        );
                        ui.label_colored(&v, accent);
                        ui.add_space(6.0);

                        ui.separator();
                        ui.add_space(6.0);

                        // Floats
                        ui.label("Sliders");
                        let v = format!("  volume: {:.0}  brightness: {:.2}", self.volume, self.brightness);
                        ui.label_colored(&v, green);
                        ui.add_space(6.0);

                        ui.separator();
                        ui.add_space(6.0);

                        // Indices
                        let sizes = ["Small", "Medium", "Large"];
                        let v = format!(
                            "  radio: {} ({})",
                            self.radio_choice,
                            sizes[self.radio_choice]
                        );
                        ui.label("Radio");
                        ui.label_colored(&v, accent);
                        ui.add_space(6.0);

                        let colors = ["Red", "Green", "Blue", "Yellow", "Purple"];
                        let v = format!(
                            "  select: {} ({})",
                            self.select_color,
                            colors[self.select_color]
                        );
                        ui.label("Select");
                        ui.label_colored(&v, accent);
                        ui.add_space(6.0);

                        let fruits = ["Apple", "Banana", "Cherry", "Date", "Elderberry"];
                        let fruit_str = self
                            .combo_fruit
                            .map(|i| fruits[i])
                            .unwrap_or("(none)");
                        let v = format!("  combobox: {:?} ({})", self.combo_fruit, fruit_str);
                        ui.label("Combobox");
                        ui.label_colored(&v, accent);
                        ui.add_space(6.0);

                        ui.separator();
                        ui.add_space(6.0);

                        let tabs = ["Overview", "Details", "Settings"];
                        let v = format!("  tab: {} ({})", self.tab_active, tabs[self.tab_active]);
                        ui.label("Tabs");
                        ui.label_colored(&v, green);
                        ui.add_space(6.0);

                        let acc_str = self
                            .accordion_open
                            .map(|i| format!("{i}"))
                            .unwrap_or_else(|| "None".into());
                        let v = format!("  accordion: {acc_str}");
                        ui.label("Accordion");
                        ui.label_colored(&v, green);
                        ui.add_space(6.0);

                        let v = format!("  page: {} / 10", self.page + 1);
                        ui.label("Pagination");
                        ui.label_colored(&v, green);
                    });
                });
            });
        });

        ui.finish();
    }

    fn on_key(
        &mut self,
        event: &esox_platform::esox_input::KeyEvent,
        modifiers: esox_platform::esox_input::Modifiers,
    ) {
        self.ui_state.process_key(event.clone(), modifiers);
    }

    fn on_resize(&mut self, width: u32, height: u32, _gpu: &GpuContext) {
        self.viewport = (width, height);
    }

    fn on_mouse(&mut self, event: MouseInputEvent) {
        match event {
            MouseInputEvent::Moved { x, y } => {
                self.ui_state.process_mouse_move(
                    x as f32,
                    y as f32,
                    self.theme.item_height,
                    self.theme.dropdown_gap,
                );
            }
            MouseInputEvent::Press { x, y, button: 0 } => {
                self.ui_state.process_mouse_click(x as f32, y as f32);
            }
            MouseInputEvent::Press { x, y, button: 2 } => {
                self.ui_state.process_right_click(x as f32, y as f32);
            }
            MouseInputEvent::Release { button: 0, .. } => {
                self.ui_state.process_mouse_release();
            }
            MouseInputEvent::Scroll { x, y, delta_y, .. } => {
                self.ui_state.process_scroll(x as f32, y as f32, delta_y);
            }
            _ => {}
        }
    }

    fn on_paste(&mut self, _text: &str) {}
    fn on_ime_commit(&mut self, text: &str) {
        self.ui_state.on_ime_commit(text.to_string());
    }
    fn on_ime_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        self.ui_state.on_ime_preedit(text, cursor);
    }
    fn on_ime_enabled(&mut self, enabled: bool) {
        self.ui_state.on_ime_enabled(enabled);
    }
    fn on_copy(&mut self) -> Option<String> {
        None
    }

    fn needs_redraw(&self) -> bool {
        self.ui_state.needs_redraw()
    }

    fn needs_continuous_redraw(&self) -> bool {
        self.transition.is_some() || self.ui_state.needs_continuous_redraw()
    }

    fn cursor_icon(&self, x: f64, y: f64) -> esox_platform::esox_input::CursorIcon {
        self.ui_state.cursor_icon(x as f32, y as f32)
    }

    fn on_scale_changed(&mut self, scale_factor: f64, _gpu: &GpuContext) {
        let factor = scale_factor as f32;
        self.ui_state.scale_factor = factor;
        self.base_light = Theme::light().scaled(factor);
        self.base_dark = Theme::dark().scaled(factor);
        self.theme = if self.is_dark {
            self.base_dark.clone()
        } else {
            self.base_light.clone()
        };
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = PlatformConfig {
        window: WindowConfig {
            title: "Widget API Showcase".into(),
            width: Some(1100),
            height: Some(750),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(WidgetShowcase::new())).unwrap();
}
