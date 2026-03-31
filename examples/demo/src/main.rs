use esox_gfx::{Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::{ClipboardProvider, InputState, ModalAction, Rect, TextRenderer, Theme, UiState, id};

struct PlatformClipboard;

impl ClipboardProvider for PlatformClipboard {
    fn read_text(&self) -> Option<String> {
        Clipboard::read(0).ok()
    }

    fn write_text(&self, text: &str) {
        let _ = Clipboard::write(text);
    }
}

struct App {
    ui_state: UiState,
    text: Option<TextRenderer>,
    base_theme: Theme,
    theme: Theme,
    viewport: (u32, u32),
    dark_mode: bool,
    theme_dirty: bool,

    // Widget state.
    counter: u32,
    slider_val: f32,
    checkbox_on: bool,
    toggle_on: bool,
    text_input: InputState,
    radio_selection: usize,
    select_choice: usize,
    tab_index: usize,
    progress: f32,
    modal_open: bool,
    confirm_open: bool,
    number_val: f64,
}

impl App {
    fn new() -> Self {
        let mut ui_state = UiState::new();
        ui_state.clipboard = Some(Box::new(PlatformClipboard));
        Self {
            ui_state,
            text: None,
            base_theme: Theme::dark(),
            theme: Theme::dark(),
            viewport: (520, 700),
            dark_mode: true,
            theme_dirty: false,

            counter: 0,
            slider_val: 50.0,
            checkbox_on: false,
            toggle_on: false,
            text_input: InputState::new(),
            radio_selection: 0,
            select_choice: 0,
            tab_index: 0,
            progress: 0.35,
            modal_open: false,
            confirm_open: false,
            number_val: 42.0,
        }
    }
}

impl AppDelegate for App {
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
        // Apply deferred theme change from previous frame.
        if self.theme_dirty {
            self.theme_dirty = false;
            if self.dark_mode {
                self.base_theme = Theme::dark();
            } else {
                self.base_theme = Theme::light();
            }
            self.theme = self.base_theme.scaled(self.ui_state.scale_factor);
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
        ui.scrollable(id!("main"), scroll_h, |ui| {
            ui.padding(24.0, |ui| {
                ui.heading("esox demo");
                ui.add_space(4.0);
                ui.label("A widget showcase for testing and development.");
                ui.add_space(12.0);

                // Theme toggle at the top (applied next frame to avoid borrow conflict).
                if ui
                    .toggle(id!("theme"), &mut self.dark_mode, "Dark mode")
                    .changed
                {
                    self.theme_dirty = true;
                }
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // -- Buttons --
                ui.collapsing_header(id!("sec_button"), "Buttons", true, |ui| {
                    let label = format!("Clicked {} times", self.counter);
                    if ui.button(id!("click"), &label).clicked {
                        self.counter += 1;
                        ui.toast_info(&format!("Counter is now {}", self.counter));
                    }
                });

                // -- Slider --
                ui.collapsing_header(id!("sec_slider"), "Slider", true, |ui| {
                    ui.slider(id!("slider"), &mut self.slider_val, 0.0, 100.0);
                    let val_label = format!("Value: {:.0}", self.slider_val);
                    ui.label(&val_label);
                });

                // -- Checkbox & Toggle --
                ui.collapsing_header(id!("sec_checks"), "Checkbox & Toggle", true, |ui| {
                    ui.checkbox(id!("check"), &mut self.checkbox_on, "Enable feature");
                    ui.toggle(id!("toggle"), &mut self.toggle_on, "Notifications");
                });

                // -- Text Input --
                ui.collapsing_header(id!("sec_text"), "Text Input", true, |ui| {
                    ui.text_input(id!("input"), &mut self.text_input, "Type here...");
                    if !self.text_input.text.is_empty() {
                        let echo = format!("You typed: {}", self.text_input.text);
                        ui.label(&echo);
                    }
                });

                // -- Radio Buttons --
                ui.collapsing_header(id!("sec_radio"), "Radio Buttons", true, |ui| {
                    ui.radio(id!("r0"), &mut self.radio_selection, 0, "Option A");
                    ui.radio(id!("r1"), &mut self.radio_selection, 1, "Option B");
                    ui.radio(id!("r2"), &mut self.radio_selection, 2, "Option C");
                    let selected = ["A", "B", "C"][self.radio_selection];
                    ui.label(&format!("Selected: Option {selected}"));
                });

                // -- Select / Dropdown --
                ui.collapsing_header(id!("sec_select"), "Select / Dropdown", true, |ui| {
                    let choices = ["Apple", "Banana", "Cherry", "Date", "Elderberry"];
                    ui.select(id!("fruit"), &mut self.select_choice, &choices);
                    ui.label(&format!("Chosen: {}", choices[self.select_choice]));
                });

                // -- Tabs --
                ui.collapsing_header(id!("sec_tabs"), "Tabs", true, |ui| {
                    let labels = ["Overview", "Details", "Settings"];
                    ui.tabs(
                        id!("tabs"),
                        &mut self.tab_index,
                        &labels,
                        |ui, idx| match idx {
                            0 => ui.label("This is the overview panel."),
                            1 => ui.label("Here are some details."),
                            2 => ui.label("Settings would go here."),
                            _ => {}
                        },
                    );
                });

                // -- Progress & Spinner --
                ui.collapsing_header(id!("sec_progress"), "Progress & Spinner", true, |ui| {
                    ui.label("Progress bar:");
                    ui.progress_bar(self.progress);
                    ui.slider(id!("prog_slider"), &mut self.progress, 0.0, 1.0);
                    let pct = format!("{:.0}%", self.progress * 100.0);
                    ui.label(&pct);
                    ui.add_space(8.0);
                    ui.label("Spinner:");
                    ui.spinner();
                });

                // -- Number Input --
                ui.collapsing_header(id!("sec_number"), "Number Input", true, |ui| {
                    ui.number_input_clamped(id!("num"), &mut self.number_val, 1.0, 0.0, 100.0);
                    ui.label(&format!("Value: {:.1}", self.number_val));
                });

                // -- Modal --
                ui.collapsing_header(id!("sec_modal"), "Modal Dialogs", true, |ui| {
                    if ui.button(id!("open_modal"), "Open modal").clicked {
                        self.modal_open = true;
                    }
                    if ui
                        .button(id!("open_confirm"), "Open confirm dialog")
                        .clicked
                    {
                        self.confirm_open = true;
                    }
                });

                // -- Toast --
                ui.collapsing_header(id!("sec_toast"), "Toast Notifications", true, |ui| {
                    if ui.button(id!("t_info"), "Info toast").clicked {
                        ui.toast_info("This is an info message.");
                    }
                    if ui.button(id!("t_success"), "Success toast").clicked {
                        ui.toast_success("Operation completed.");
                    }
                    if ui.button(id!("t_error"), "Error toast").clicked {
                        ui.toast_error("Something went wrong!");
                    }
                    if ui.button(id!("t_warning"), "Warning toast").clicked {
                        ui.toast_warning("Careful with that.");
                    }
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label("End of demo.");
            });
        });

        // Modals must be drawn outside the scrollable.
        ui.modal(
            id!("demo_modal"),
            &mut self.modal_open,
            "Example Modal",
            340.0,
            |ui| {
                ui.label("This is a modal dialog.");
                ui.label("Click outside or press Escape to close.");
            },
        );

        match ui.modal_confirm(
            id!("confirm_modal"),
            &mut self.confirm_open,
            "Confirm Action",
            "Are you sure you want to proceed?",
        ) {
            ModalAction::Confirm => {
                // The modal already set confirm_open = false.
                ui.toast_success("Confirmed!");
            }
            ModalAction::Cancel => {
                ui.toast_info("Cancelled.");
            }
            ModalAction::None => {}
        }

        let _ = ui.finish();
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
        self.ui_state.needs_continuous_redraw()
    }

    fn cursor_icon(&self, x: f64, y: f64) -> esox_platform::esox_input::CursorIcon {
        self.ui_state.cursor_icon(x as f32, y as f32)
    }

    fn on_scale_changed(&mut self, scale_factor: f64, _gpu: &GpuContext) {
        let factor = scale_factor as f32;
        self.ui_state.scale_factor = factor;
        self.theme = self.base_theme.scaled(factor);
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = PlatformConfig {
        window: WindowConfig {
            title: "esox demo".into(),
            width: Some(520),
            height: Some(700),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(App::new())).unwrap();
}
