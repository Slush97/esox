use esox_gfx::{Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::{
    ClipboardProvider, InputState, Rect, TextRenderer, Theme, UiState, id,
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

struct App {
    ui_state: UiState,
    text: Option<TextRenderer>,
    base_theme: Theme,
    theme: Theme,
    viewport: (u32, u32),
    // Widget state.
    counter: u32,
    slider_val: f32,
    checkbox_on: bool,
    toggle_on: bool,
    text_input: InputState,
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
            viewport: (500, 600),
            counter: 0,
            slider_val: 50.0,
            checkbox_on: false,
            toggle_on: false,
            text_input: InputState::new(),
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
        self.ui_state.update_blink(self.theme.cursor_blink_ms);

        let text = self.text.as_mut().unwrap();
        let vp = Rect::new(0.0, 0.0, self.viewport.0 as f32, self.viewport.1 as f32);
        let mut ui = esox_ui::Ui::begin(
            frame, gpu, resources, text, &mut self.ui_state, &self.theme, vp,
        );

        let scroll_h = self.viewport.1 as f32;
        ui.scrollable(id!("main"), scroll_h, |ui| {
            ui.padding(24.0, |ui| {
                ui.heading("esox demo");
                ui.add_space(8.0);
                ui.label("A minimal widget showcase.");
                ui.add_space(24.0);

                // Button with counter.
                ui.header_label("BUTTON");
                let label = format!("Clicked {} times", self.counter);
                if ui.button(id!("click"), &label).clicked {
                    self.counter += 1;
                }
                ui.add_space(16.0);

                // Slider.
                ui.header_label("SLIDER");
                ui.slider(id!("slider"), &mut self.slider_val, 0.0, 100.0);
                let val_label = format!("{:.0}", self.slider_val);
                ui.label(&val_label);
                ui.add_space(16.0);

                // Checkbox.
                ui.header_label("CHECKBOX");
                ui.checkbox(id!("check"), &mut self.checkbox_on, "Enable feature");
                ui.add_space(16.0);

                // Toggle.
                ui.header_label("TOGGLE");
                ui.toggle(id!("toggle"), &mut self.toggle_on, "Dark mode");
                ui.add_space(16.0);

                // Text input.
                ui.header_label("TEXT INPUT");
                ui.text_input(id!("input"), &mut self.text_input, "Type here...");
                ui.add_space(16.0);

                ui.separator();
                ui.add_space(8.0);
                ui.label("End of demo.");
            });
        });

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
        self.ui_state.invalidate_layout();
    }

    fn on_mouse(&mut self, event: MouseInputEvent) {
        match event {
            MouseInputEvent::Moved { x, y } => {
                self.ui_state
                    .process_mouse_move(x as f32, y as f32, self.theme.item_height, self.theme.dropdown_gap);
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
            width: Some(500),
            height: Some(600),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(App::new())).unwrap();
}
