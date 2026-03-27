use esox_gfx::{Color, Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::esox_input;
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::id::fnv1a_runtime;
use esox_ui::{
    ClipboardProvider, InputState, Rect, RichText, Span, SpacingScale, Status, TextRenderer,
    Theme, Ui, UiState, id,
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

// ── Mock data ─────────────────────────────────────────────────────────

struct Member {
    name: &'static str,
    status: Status,
    color: Color,
}

struct Message {
    sender: &'static str,
    content: &'static str,
    timestamp: &'static str,
    show_sender: bool,
}

struct DiscordApp {
    ui_state: UiState,
    text: Option<TextRenderer>,
    theme: Theme,
    viewport: (u32, u32),
    current_channel: usize,
    message_input: InputState,
    channels: Vec<&'static str>,
    members: Vec<Member>,
    messages: Vec<Message>,
}

fn discord_theme() -> Theme {
    let mut t = Theme::dark();
    t.bg_base = Color::new(0.19, 0.20, 0.22, 1.0);       // #313338
    t.bg_surface = Color::new(0.17, 0.18, 0.20, 1.0);     // #2b2d31
    t.bg_raised = Color::new(0.21, 0.22, 0.25, 1.0);      // #383a40
    t.bg_input = Color::new(0.24, 0.25, 0.27, 1.0);       // #3e4046
    t.fg = Color::new(0.86, 0.87, 0.88, 1.0);             // #dbdcde
    t.fg_muted = Color::new(0.57, 0.59, 0.63, 1.0);       // #929599
    t.fg_dim = Color::new(0.40, 0.42, 0.45, 1.0);         // #676b73
    t.accent = Color::new(0.345, 0.396, 0.949, 1.0);      // #5865f2 blurple
    t.accent_hover = Color::new(0.443, 0.490, 0.957, 1.0);
    t.accent_dim = Color::new(0.345, 0.396, 0.949, 0.15);
    t.green = Color::new(0.22, 0.65, 0.36, 1.0);
    t.amber = Color::new(0.98, 0.66, 0.06, 1.0);
    t.red = Color::new(0.93, 0.28, 0.28, 1.0);
    t.border = Color::new(0.24, 0.25, 0.27, 1.0);
    t.corner_radius = 4.0;
    t
}

impl DiscordApp {
    fn new() -> Self {
        Self {
            ui_state: UiState::new(),
            text: None,
            theme: discord_theme(),
            viewport: (1200, 800),
            current_channel: 0,
            message_input: InputState::new(),
            channels: vec!["general", "ui-design", "gpu-rendering", "font-shaping", "off-topic"],
            members: vec![
                Member { name: "alice", status: Status::Online, color: Color::new(0.38, 0.77, 0.55, 1.0) },
                Member { name: "bob", status: Status::Online, color: Color::new(0.55, 0.63, 0.82, 1.0) },
                Member { name: "charlie", status: Status::Idle, color: Color::new(0.85, 0.65, 0.30, 1.0) },
                Member { name: "diana", status: Status::Offline, color: Color::new(0.60, 0.60, 0.60, 1.0) },
                Member { name: "eve", status: Status::Online, color: Color::new(0.82, 0.45, 0.45, 1.0) },
            ],
            messages: vec![
                Message { sender: "alice", content: "hey everyone! just pushed the new sidebar widget", timestamp: "Today at 11:32 AM", show_sender: true },
                Message { sender: "alice", content: "it supports header, scrollable body, and footer zones", timestamp: "Today at 11:32 AM", show_sender: false },
                Message { sender: "bob", content: "nice! does it handle the selected state for items?", timestamp: "Today at 11:34 AM", show_sender: true },
                Message { sender: "alice", content: "yep, .selected(true) gives it the accent background, and there's hover animations too", timestamp: "Today at 11:35 AM", show_sender: true },
                Message { sender: "charlie", content: "what about badges for unread counts?", timestamp: "Today at 11:38 AM", show_sender: true },
                Message { sender: "alice", content: ".badge(count) on the item builder, renders a red pill on the right", timestamp: "Today at 11:39 AM", show_sender: true },
                Message { sender: "eve", content: "tested it with veil-client, works great with split_pane_h_mut", timestamp: "Today at 11:42 AM", show_sender: true },
                Message { sender: "bob", content: "the fractional glyph positioning experiment was interesting", timestamp: "Today at 11:45 AM", show_sender: true },
                Message { sender: "bob", content: "but bilinear filtering on bitmap glyphs just makes everything blurry", timestamp: "Today at 11:45 AM", show_sender: false },
                Message { sender: "charlie", content: "yeah you need SDF text or sub-pixel offset rasterization for that to work", timestamp: "Today at 11:47 AM", show_sender: true },
                Message { sender: "diana", content: "i'll look into MSDF text rendering next week", timestamp: "Today at 12:01 PM", show_sender: true },
                Message { sender: "alice", content: "sounds good. for now the integer-rounded positions with nearest sampling are crisp enough", timestamp: "Today at 12:03 PM", show_sender: true },
            ],
        }
    }

}

impl AppDelegate for DiscordApp {
    fn on_init(&mut self, gpu: &GpuContext, _resources: &mut RenderResources) {
        match TextRenderer::new(gpu) {
            Ok(tr) => self.text = Some(tr),
            Err(e) => eprintln!("failed to init text: {e}"),
        }
    }

    fn on_redraw(
        &mut self,
        gpu: &GpuContext,
        resources: &mut RenderResources,
        frame: &mut Frame,
        _perf: &esox_platform::perf::PerfMonitor,
    ) {
        let text = self.text.as_mut().unwrap();
        let (w, h) = self.viewport;
        let vp = Rect::new(0.0, 0.0, w as f32, h as f32);

        self.ui_state.update_blink(self.theme.cursor_blink_ms);
        self.ui_state.clipboard = Some(Box::new(PlatformClipboard));

        let mut ui = Ui::begin(frame, gpu, resources, text, &mut self.ui_state, &self.theme, vp);

        // Snapshot data needed by closures to avoid borrow conflicts.
        let sidebar_ratio = (240.0 / w.max(600) as f32).clamp(0.15, 0.25);
        let current_channel = self.current_channel;
        let channels = self.channels.to_vec();
        let member_data: Vec<(&str, Status, Color)> = self.members.iter().map(|m| (m.name, m.status, m.color)).collect();
        let messages: Vec<(&str, &str, &str, bool)> = self.messages.iter().map(|m| (m.sender, m.content, m.timestamp, m.show_sender)).collect();
        let mut new_channel = current_channel;

        ui.split_pane_h_mut(id!("main_split"), sidebar_ratio, |ui, panel| {
            match panel {
                0 => {
                    // Sidebar.
                    ui.sidebar(id!("nav"), |sb| {
                        sb.header(|ui| {
                            ui.rich_label(&RichText::new().push(Span {
                                text: "esox dev",
                                color: Some(ui.theme().fg),
                                bold: true,
                                size: Some(ui.theme().font_size),
                                letter_spacing: None,
                                weight: None,
                                background: None,
                                decoration: esox_ui::TextDecoration::None,
                            }));
                        });

                        sb.section("TEXT CHANNELS", |ui| {
                            for (i, &ch) in channels.iter().enumerate() {
                                let ch_id = fnv1a_runtime(&format!("ch_{ch}"));
                                let resp = ui.sidebar_item(ch_id, ch)
                                    .prefix("#")
                                    .selected(i == current_channel)
                                    .muted(i != current_channel)
                                    .show(ui);
                                if resp.clicked {
                                    new_channel = i;
                                }
                            }
                        });

                        sb.section("MEMBERS", |ui| {
                            for &(name, status, color) in &member_data {
                                ui.padding(SpacingScale::Lg, |ui| {
                                    ui.row_spaced(ui.theme().content_spacing, |ui| {
                                        ui.avatar_colored_with_status(&name[..2], 24.0, color, status);
                                        let c = if matches!(status, Status::Offline) { ui.theme().fg_dim } else { ui.theme().fg };
                                        ui.label_colored(name, c);
                                    });
                                });
                            }
                        });
                    });
                }
                1 => {
                    // Messages.
                    let channel = channels[current_channel];

                    ui.surface(|ui| {
                        ui.padding(SpacingScale::Lg, |ui| {
                            ui.row(|ui| {
                                let dim = ui.theme().fg_dim;
                                ui.rich_label(&RichText::new().colored("#  ", dim).bold(channel));
                                ui.spacer();
                                let muted = ui.theme().fg_muted;
                                let online = member_data.iter().filter(|(_, s, _)| !matches!(s, Status::Offline)).count();
                                ui.label_colored(&format!("{online} online"), muted);
                            });
                        });
                    });

                    ui.scrollable_fill(id!("msg_scroll"), |ui| {
                        ui.padding(SpacingScale::Lg, |ui| {
                            for (i, &(sender, content, timestamp, show_sender)) in messages.iter().enumerate() {
                                if show_sender {
                                    if i > 0 { ui.add_space(ui.theme().content_spacing * 1.5); }
                                    let name_color = member_data.iter().find(|(n, _, _)| *n == sender).map(|(_, _, c)| *c).unwrap_or(ui.theme().fg);
                                    ui.row_spaced(ui.theme().content_spacing, |ui| {
                                        ui.avatar_colored(&sender[..2], 32.0, name_color);
                                        let dim = ui.theme().fg_dim;
                                        ui.rich_label(&RichText::new()
                                            .colored_bold(sender, name_color)
                                            .push(Span {
                                                text: &format!("  {timestamp}"),
                                                color: Some(dim),
                                                bold: false,
                                                size: Some(ui.theme().font_size * 0.80),
                                                letter_spacing: None,
                                                weight: None,
                                                background: None,
                                                decoration: esox_ui::TextDecoration::None,
                                            }),
                                        );
                                    });
                                }
                                let indent = 32.0 + ui.theme().content_spacing;
                                let style = esox_ui::WidgetStyle {
                                    padding: Some(esox_ui::Spacing { top: 2.0, bottom: 0.0, left: indent, right: 0.0 }),
                                    ..Default::default()
                                };
                                ui.with_style(style, |ui| { ui.label(content); });
                            }
                        });
                    });

                    ui.surface(|ui| {
                        ui.padding(SpacingScale::Md, |ui| {
                            ui.text_input(id!("msg_input"), &mut self.message_input, &format!("Message #{channel}"));
                        });
                    });
                }
                _ => {}
            }
        });
        self.current_channel = new_channel;

        let _ = ui.finish();
    }

    fn on_resize(&mut self, width: u32, height: u32, _gpu: &GpuContext) {
        self.viewport = (width.max(1), height.max(1));
    }

    fn on_key(&mut self, event: &esox_input::KeyEvent, modifiers: esox_input::Modifiers) {
        self.ui_state.process_key(event.clone(), modifiers);
    }

    fn on_mouse(&mut self, event: MouseInputEvent) {
        match event {
            MouseInputEvent::Moved { x, y } => {
                self.ui_state.process_mouse_move(x as f32, y as f32, self.theme.item_height, self.theme.dropdown_gap);
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
    fn on_ime_commit(&mut self, text: &str) { self.ui_state.on_ime_commit(text.to_string()); }
    fn on_ime_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) { self.ui_state.on_ime_preedit(text, cursor); }
    fn on_ime_enabled(&mut self, enabled: bool) { self.ui_state.on_ime_enabled(enabled); }
    fn on_copy(&mut self) -> Option<String> { None }
    fn needs_redraw(&self) -> bool { self.ui_state.needs_redraw() }
    fn needs_continuous_redraw(&self) -> bool { self.ui_state.needs_continuous_redraw() }
    fn cursor_icon(&self, x: f64, y: f64) -> esox_input::CursorIcon { self.ui_state.cursor_icon(x as f32, y as f32) }
    fn on_scale_changed(&mut self, _scale: f64, _gpu: &GpuContext) {}
}

fn main() -> Result<(), esox_platform::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = PlatformConfig {
        window: WindowConfig {
            title: "Discord Demo \u{2014} esox".into(),
            width: Some(1200),
            height: Some(800),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(DiscordApp::new()))
}
