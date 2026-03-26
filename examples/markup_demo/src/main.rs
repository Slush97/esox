use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use esox_gfx::{Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::interpret::MarkupState;
use esox_ui::{
    ClipboardProvider, Easing, KeyframeSequence, PlaybackMode, Rect, SpringConfig, TextRenderer,
    Theme, UiState,
};

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
    // Animation demo state.
    start_time: Instant,
    // Hot-reload: watch the .exui file for changes.
    exui_path: Option<PathBuf>,
    exui_mtime: Option<SystemTime>,
    last_reload_check: Instant,
}

impl MarkupDemoApp {
    fn new() -> Self {
        // Try to find the .exui file relative to the executable for hot-reload.
        // Falls back to the embedded source if the file isn't found on disk.
        let exui_path = Self::find_exui_file();
        let (source, exui_mtime) = match &exui_path {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(s) => {
                    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
                    println!("[hot-reload] watching {}", path.display());
                    (s, mtime)
                }
                Err(_) => (MARKUP_SOURCE.to_string(), None),
            },
            None => (MARKUP_SOURCE.to_string(), None),
        };

        let nodes = esox_markup::parse(&source).expect("failed to parse markup");
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
            start_time: Instant::now(),
            exui_path,
            exui_mtime,
            last_reload_check: Instant::now(),
        }
    }

    /// Locate ui.exui on disk. Checks common locations relative to CWD and the
    /// cargo manifest dir.
    fn find_exui_file() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("examples/markup_demo/ui.exui"),
            PathBuf::from("ui.exui"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }

    /// Check if the .exui file changed on disk and re-parse if so.
    fn check_hot_reload(&mut self) {
        // Only stat the file every 500ms to avoid excessive syscalls.
        if self.last_reload_check.elapsed().as_millis() < 500 {
            return;
        }
        self.last_reload_check = Instant::now();

        let path = match &self.exui_path {
            Some(p) => p,
            None => return,
        };

        let mtime = match std::fs::metadata(path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return,
        };

        if self.exui_mtime == Some(mtime) {
            return;
        }

        // File changed — re-read and re-parse.
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[hot-reload] read error: {e}");
                return;
            }
        };

        match esox_markup::parse(&source) {
            Ok(new_nodes) => {
                self.nodes = new_nodes;
                self.exui_mtime = Some(mtime);
                println!("[hot-reload] reloaded {}", path.display());
            }
            Err(e) => {
                eprintln!("[hot-reload] parse error: {e}");
            }
        }
    }
}

/// Find a node by `bind` name anywhere in the tree and set a property.
fn set_node_prop(nodes: &mut [esox_markup::Node], bind: &str, key: &str, value: f64) {
    for node in nodes.iter_mut() {
        if node.prop_str("bind") == Some(bind) {
            node.props
                .insert(key.to_string(), esox_markup::Value::Number(value));
            return;
        }
        set_node_prop(&mut node.children, bind, key, value);
    }
}

/// Find a node by `bind` name and set its `bg` color.
fn set_node_color(nodes: &mut [esox_markup::Node], bind: &str, color: u32) {
    for node in nodes.iter_mut() {
        if node.prop_str("bind") == Some(bind) {
            node.props
                .insert("bg".to_string(), esox_markup::Value::Color(color));
            return;
        }
        set_node_color(&mut node.children, bind, color);
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
        self.check_hot_reload();
        self.ui_state.update_blink(self.theme.cursor_blink_ms);

        let text = self.text.as_mut().unwrap();
        let vp = Rect::new(0.0, 0.0, self.viewport.0 as f32, self.viewport.1 as f32);

        // Drive animation demo values before rendering.
        let elapsed = self.start_time.elapsed().as_secs_f64();

        // Progress bar: cycle 0→1 over 3 seconds, pause 1 second, then back.
        let cycle = elapsed % 8.0;
        let progress = if cycle < 3.0 {
            cycle / 3.0
        } else if cycle < 4.0 {
            1.0
        } else if cycle < 7.0 {
            1.0 - (cycle - 4.0) / 3.0
        } else {
            0.0
        };
        set_node_prop(&mut self.nodes, "anim-progress", "value", progress);
        set_node_prop(&mut self.nodes, "anim-spring", "value", progress);

        // Easing comparison bars: toggle between 0.1 and 0.9 every 2 seconds.
        let target = if (elapsed as u64 / 2).is_multiple_of(2) {
            0.9
        } else {
            0.1
        };
        set_node_prop(&mut self.nodes, "ease-cubic", "value", target);
        set_node_prop(&mut self.nodes, "ease-bounce", "value", target);
        set_node_prop(&mut self.nodes, "ease-back", "value", target);
        set_node_prop(&mut self.nodes, "ease-expo", "value", target);
        set_node_prop(&mut self.nodes, "ease-spring", "value", target);

        // Color cards: cycle through colors.
        let color_idx = (elapsed as u64 / 2) % 3;
        let colors = [0x1a2744u32, 0x2d1b36, 0x1b362d];
        set_node_color(&mut self.nodes, "color-card-a", colors[color_idx as usize]);
        set_node_color(
            &mut self.nodes,
            "color-card-b",
            colors[(color_idx as usize + 1) % 3],
        );
        set_node_color(
            &mut self.nodes,
            "color-card-c",
            colors[(color_idx as usize + 2) % 3],
        );

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

        // Render Rust-API animation demos after the markup content.
        // These only show when the Animation tab (index 3) is active.
        let active_tab = self.markup_state.get_selected("main-tabs").unwrap_or(0);
        if active_tab == 3 {
            // Keyframe animation demo.
            ui.heading("Keyframe API (Rust)");
            ui.add_space(4.0);

            let pulse = KeyframeSequence::new(1200.0)
                .stop(0.0, 0.0, Easing::Linear)
                .stop(0.3, 0.85, Easing::EaseOutCubic)
                .stop(0.6, 0.5, Easing::EaseInOutQuad)
                .stop(1.0, 1.0, Easing::EaseOutExpo);
            let v = ui.animate_keyframes(esox_ui::id!("kf_pulse"), &pulse, PlaybackMode::Infinite);
            ui.label(&format!("Keyframe (Infinite): {v:.2}"));
            ui.progress_bar(v);

            ui.add_space(4.0);
            let bounce = KeyframeSequence::new(2000.0)
                .stop(0.0, 0.0, Easing::Linear)
                .stop(0.5, 1.0, Easing::EaseOutBounce)
                .stop(1.0, 0.0, Easing::EaseOutBounce);
            let v = ui.animate_keyframes(
                esox_ui::id!("kf_bounce"),
                &bounce,
                PlaybackMode::PingPongInfinite,
            );
            ui.label(&format!("Bounce PingPong: {v:.2}"));
            ui.progress_bar(v);

            ui.add_space(8.0);
            ui.heading("Spring API (Rust)");
            ui.add_space(4.0);

            // Spring target toggles every 2 seconds.
            let spring_target = if (elapsed as u64 / 2).is_multiple_of(2) {
                1.0
            } else {
                0.0
            };
            let snappy = ui.animate_spring(
                esox_ui::id!("spring_snappy"),
                spring_target,
                SpringConfig::SNAPPY,
            );
            ui.label(&format!("SNAPPY: {snappy:.2}"));
            ui.progress_bar(snappy);

            let gentle = ui.animate_spring(
                esox_ui::id!("spring_gentle"),
                spring_target,
                SpringConfig::GENTLE,
            );
            ui.label(&format!("GENTLE: {gentle:.2}"));
            ui.progress_bar(gentle);

            let bouncy = ui.animate_spring(
                esox_ui::id!("spring_bouncy"),
                spring_target,
                SpringConfig::BOUNCY,
            );
            ui.label(&format!("BOUNCY: {bouncy:.2}"));
            ui.progress_bar(bouncy);

            let stiff = ui.animate_spring(
                esox_ui::id!("spring_stiff"),
                spring_target,
                SpringConfig::STIFF,
            );
            ui.label(&format!("STIFF: {stiff:.2}"));
            ui.progress_bar(stiff);
        }

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
        // Always redraw when on the Animation tab (time-driven values) or
        // when watching a file for hot-reload.
        let animation_tab_active = self.markup_state.get_selected("main-tabs") == Some(3);
        self.ui_state.needs_continuous_redraw() || animation_tab_active || self.exui_path.is_some()
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
