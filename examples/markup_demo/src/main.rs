use esox_gfx::{Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::interpret::MarkupState;
use esox_ui::{ClipboardProvider, Rect, TextRenderer, Theme, UiState};

const MARKUP_SOURCE: &str = include_str!("../ui.exui");

struct PlatformClipboard;

impl ClipboardProvider for PlatformClipboard {
    fn read_text(&self) -> Option<String> {
        Clipboard::read(0).ok()
    }

    fn write_text(&self, text: &str) {
        let _ = Clipboard::write(text);
    }
}

struct MarkupDemoApp {
    ui_state: UiState,
    text: Option<TextRenderer>,
    base_theme: Theme,
    theme: Theme,
    viewport: (u32, u32),
    // Markup-specific state.
    nodes: Vec<esox_markup::Node>,
    markup_state: MarkupState,
}

impl MarkupDemoApp {
    fn new() -> Self {
        let nodes = esox_markup::parse(MARKUP_SOURCE).expect("failed to parse markup");
        let mut ui_state = UiState::new();
        ui_state.clipboard = Some(Box::new(PlatformClipboard));

        // Pre-populate some state for the demo.
        let mut markup_state = MarkupState::new();
        markup_state.set_text("username", "alice");
        markup_state.set_f64("quantity", 5.0);

        Self {
            ui_state,
            text: None,
            base_theme: Theme::dark(),
            theme: Theme::dark(),
            viewport: (800, 900),
            nodes,
            markup_state,
        }
    }
}

impl AppDelegate for MarkupDemoApp {
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
            frame,
            gpu,
            resources,
            text,
            &mut self.ui_state,
            &self.theme,
            vp,
        );

        // The entire UI is driven by the markup interpreter.
        let actions = esox_ui::interpret::render(&mut ui, &self.nodes, &mut self.markup_state);

        // Handle actions from the markup.
        for action in &actions {
            match action.name.as_str() {
                "save" => {
                    let username = self.markup_state.get_text("username").unwrap_or("?");
                    let email = self.markup_state.get_text("email").unwrap_or("?");
                    let role = self.markup_state.get_selected("role");
                    let rating = self.markup_state.get_u8("user-rating");
                    println!(
                        "[save] username={username}, email={email}, role={role:?}, rating={rating:?}"
                    );
                }
                "cancel" => println!("[cancel]"),
                "reset" => {
                    self.markup_state.clear();
                    self.markup_state.set_text("username", "alice");
                    self.markup_state.set_f64("quantity", 5.0);
                    println!("[reset] state cleared");
                }
                "navigate" => {
                    println!("[navigate] {:?}", action.kind);
                }
                "step" => {
                    println!("[step] {:?}", action.kind);
                }
                other => {
                    println!("[{other}] {:?} from {}", action.kind, action.source);
                }
            }
        }

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
            title: "Markup Demo".into(),
            width: Some(800),
            height: Some(900),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(MarkupDemoApp::new())).unwrap();
}
