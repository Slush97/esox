//! Layout showcase — demonstrates flex wrap, responsive layout, nested scrolling,
//! styled components, paragraph widget, truncation modes, and debug overlay.

use esox_gfx::{Color, Frame, GpuContext, RenderResources, ShapeBuilder};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::{
    ClipboardProvider, FlexItem, FlexWrap, Rect, SpacingScale, StyleState, TextRenderer, TextSize,
    Theme, TruncationMode, UiState, WidgetStyle, WidthClass, id,
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
    theme: Theme,
    viewport: (u32, u32),
}

impl App {
    fn new() -> Self {
        let mut ui_state = UiState::new();
        ui_state.clipboard = Some(Box::new(PlatformClipboard));
        ui_state.debug_overlay = true;

        Self {
            ui_state,
            text: None,
            theme: Theme::dark(),
            viewport: (900, 700),
        }
    }
}

impl AppDelegate for App {
    fn on_init(&mut self, gpu: &GpuContext, _resources: &mut RenderResources) {
        self.text = Some(TextRenderer::new(gpu).expect("font"));
    }

    fn on_redraw(
        &mut self,
        gpu: &GpuContext,
        resources: &mut RenderResources,
        frame: &mut Frame,
        _perf: &esox_platform::perf::PerfMonitor,
    ) {
        let text = self.text.as_mut().unwrap();
        let viewport = Rect::new(0.0, 0.0, self.viewport.0 as f32, self.viewport.1 as f32);
        let mut ui = esox_ui::Ui::begin(
            frame,
            gpu,
            resources,
            text,
            &mut self.ui_state,
            &self.theme,
            viewport,
        );

        let scroll_h = self.viewport.1 as f32;
        ui.scrollable(id!("page"), scroll_h, |ui| {
        ui.padding(16.0, |ui| {
            ui.heading("Layout Showcase");
            ui.add_space(8.0);

            // Responsive layout.
            let width_class = ui.width_class();
            let class_name = match width_class {
                WidthClass::Compact => "Compact",
                WidthClass::Medium => "Medium",
                WidthClass::Expanded => "Expanded",
            };
            ui.label(&format!("Width class: {class_name} ({:.0}px)", ui.region_width()));

            let debug_status = if ui.is_debug_overlay() { "ON" } else { "OFF" };
            ui.label(&format!("Debug overlay: {debug_status}"));

            ui.add_space(12.0);

            // --- Paragraph widget ---
            ui.header_label("PARAGRAPH WIDGET");
            ui.paragraph(
                id!("intro_paragraph"),
                "This paragraph widget automatically wraps text and responds to hover. \
                 It uses the theme's line_spacing for inter-line gaps and measures \
                 its height with measure_text_wrapped().",
            );

            ui.add_space(12.0);

            // --- Truncation modes ---
            ui.header_label("TRUNCATION MODES");
            let long_text = "This is a very long text that will be truncated in different modes to show the three available truncation options";
            ui.label("End:");
            ui.label_truncated_mode(long_text, TruncationMode::End);
            ui.label("Start:");
            ui.label_truncated_mode(long_text, TruncationMode::Start);
            ui.label("Middle:");
            ui.label_truncated_mode(long_text, TruncationMode::Middle);

            ui.add_space(12.0);

            // --- Style derivation ---
            ui.header_label("STYLE STATE DERIVATION");
            ui.row(|ui| {
                let base = ui.theme().accent;
                for (i, &state) in [StyleState::Normal, StyleState::Hovered, StyleState::Active, StyleState::Disabled].iter().enumerate() {
                    let bg = ui.theme().derive_bg(base, state);
                    let style = WidgetStyle {
                        bg: Some(bg),
                        ..Default::default()
                    };
                    ui.with_style(style, |ui| {
                        let name = match state {
                            StyleState::Normal => "Normal",
                            StyleState::Hovered => "Hovered",
                            StyleState::Focused => "Focused",
                            StyleState::Active => "Active",
                            StyleState::Disabled => "Disabled",
                        };
                        ui.button(id!("style_demo").wrapping_add(i as u64), name);
                    });
                }
            });

            ui.add_space(12.0);

            // --- Flex row with grow ---
            ui.header_label("FLEX ROW WITH GROW");
            ui.flex_row().gap(8.0).show_flex(id!("flex_grow"), |flex| {
                flex.item(FlexItem::default().grow(1.0), |ui| {
                    ui.button(id!("grow1"), "Grow 1");
                });
                flex.item_default(|ui| {
                    ui.button(id!("fixed"), "Fixed");
                });
                flex.item(FlexItem::default().grow(2.0), |ui| {
                    ui.button(id!("grow2"), "Grow 2");
                });
            });

            ui.add_space(12.0);

            // --- Flex wrap ---
            ui.header_label("FLEX WRAP");
            ui.flex_row().gap(8.0).wrap(FlexWrap::Wrap).show(|ui| {
                for i in 0..12 {
                    ui.button(id!("wrap_btn").wrapping_add(i), &format!("Item {i}"));
                }
            });

            ui.add_space(12.0);

            // --- Nested scrolling ---
            ui.header_label("NESTED SCROLLING");
            ui.scrollable(id!("outer_scroll"), 200.0, |ui| {
                for i in 0..5 {
                    ui.label(&format!("Outer item {i}"));
                }
                ui.muted_label("Inner scrollable (propagates at limit):");
                ui.scrollable(id!("inner_scroll"), 100.0, |ui| {
                    for j in 0..20 {
                        ui.label(&format!("  Inner item {j}"));
                    }
                });
                for i in 5..15 {
                    ui.label(&format!("Outer item {i}"));
                }
            });

            ui.add_space(12.0);

            // --- Text color inheritance ---
            ui.header_label("TEXT COLOR INHERITANCE");
            let green_style = WidgetStyle {
                text_color: Some(esox_gfx::Color::new(0.2, 0.9, 0.3, 1.0)),
                ..Default::default()
            };
            ui.with_style(green_style, |ui| {
                ui.label("This text inherits green via text_color");
                ui.label("So does this one");
            });

            ui.add_space(12.0);

            // --- Sub-region API: custom app shell ---
            ui.header_label("SUB-REGION API: CUSTOM APP SHELL");
            ui.muted_label("A sidebar + header + content layout built with sub_region — no split_pane needed.");
            ui.add_space(4.0);

            // Allocate a fixed-height region for the demo.
            let shell_w = ui.region_width();
            let shell_h = 280.0;
            let shell_rect = ui.allocate_rect(shell_w, shell_h);

            // Draw shell background.
            let p = ui.painter();
            p.frame.push(
                ShapeBuilder::rect(shell_rect.x, shell_rect.y, shell_rect.w, shell_rect.h)
                    .color(p.theme.bg_base)
                    .border_radius(esox_gfx::BorderRadius::uniform(p.theme.corner_radius))
                    .build(),
            );

            // Define regions.
            let sidebar_w = 160.0;
            let header_h = 40.0;
            let border = 1.0;
            let inset = ui.theme().spacing_unit;

            let sidebar_rect = Rect::new(
                shell_rect.x,
                shell_rect.y,
                sidebar_w,
                shell_rect.h,
            );
            let header_rect = Rect::new(
                shell_rect.x + sidebar_w + border,
                shell_rect.y,
                shell_rect.w - sidebar_w - border,
                header_h,
            );
            let content_rect = Rect::new(
                shell_rect.x + sidebar_w + border,
                shell_rect.y + header_h + border,
                shell_rect.w - sidebar_w - border,
                shell_rect.h - header_h - border,
            );

            // Draw sidebar background.
            let p = ui.painter();
            p.frame.push(
                ShapeBuilder::rect(sidebar_rect.x, sidebar_rect.y, sidebar_rect.w, sidebar_rect.h)
                    .color(p.theme.bg_surface)
                    .build(),
            );

            // Draw divider lines.
            let border_color = ui.theme().border;
            let p = ui.painter();
            // Vertical divider between sidebar and main.
            p.frame.push(
                ShapeBuilder::rect(sidebar_rect.x + sidebar_rect.w, sidebar_rect.y, border, sidebar_rect.h)
                    .color(border_color)
                    .build(),
            );
            // Horizontal divider below header.
            p.frame.push(
                ShapeBuilder::rect(header_rect.x, header_rect.y + header_rect.h, header_rect.w, border)
                    .color(border_color)
                    .build(),
            );

            // Sidebar — clipped sub-region with inset.
            ui.sub_region(sidebar_rect, inset, true, false, |ui| {
                ui.add_space(8.0);
                ui.muted_label("CHANNELS");
                ui.add_space(4.0);
                for (i, name) in ["# general", "# random", "# design", "# engineering", "# announcements", "# off-topic"].iter().enumerate() {
                    if ui.button(id!("chan").wrapping_add(i as u64), name).clicked {
                        // Channel selection would go here.
                    }
                }
            });

            // Header — clipped sub-region.
            ui.sub_region(header_rect, inset, true, false, |ui| {
                ui.row_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label_sized("# general", TextSize::Lg);
                    ui.fill_space(100.0);
                    ui.muted_label("3 members online");
                });
            });

            // Content — clipped sub-region with overflowing items.
            ui.sub_region(content_rect, inset, true, false, |ui| {
                ui.add_space(4.0);
                for i in 0..12 {
                    ui.row(|ui| {
                        let avatar_rect = ui.allocate_rect(24.0, 24.0);
                        let p = ui.painter();
                        esox_ui::paint::draw_rounded_rect(
                            p.frame,
                            avatar_rect,
                            Color::new(0.3 + (i as f32) * 0.05, 0.5, 0.7, 1.0),
                            12.0,
                        );
                        ui.label(&format!("User {i}: This is a chat message that might overflow the panel"));
                    });
                }
            });

            ui.add_space(12.0);

            // --- Sub-region API: clipped panel grid ---
            ui.header_label("SUB-REGION API: CLIPPED PANEL GRID");
            ui.muted_label("Four panels, each independently clipped. Content overflows are hidden.");
            ui.add_space(4.0);

            let grid_w = ui.region_width();
            let grid_h = 200.0;
            let gap = 8.0;
            let panel_w = (grid_w - gap) / 2.0;
            let panel_h = (grid_h - gap) / 2.0;
            let grid_rect = ui.allocate_rect(grid_w, grid_h);

            let panels = [
                (Rect::new(grid_rect.x, grid_rect.y, panel_w, panel_h), "Clipped Labels", Color::new(0.15, 0.15, 0.25, 1.0)),
                (Rect::new(grid_rect.x + panel_w + gap, grid_rect.y, panel_w, panel_h), "Buttons", Color::new(0.15, 0.25, 0.15, 1.0)),
                (Rect::new(grid_rect.x, grid_rect.y + panel_h + gap, panel_w, panel_h), "Overflowing Text", Color::new(0.25, 0.15, 0.15, 1.0)),
                (Rect::new(grid_rect.x + panel_w + gap, grid_rect.y + panel_h + gap, panel_w, panel_h), "Mixed Widgets", Color::new(0.20, 0.20, 0.12, 1.0)),
            ];

            // Draw panel backgrounds.
            let corner_radius = ui.theme().corner_radius;
            for &(rect, _, bg) in &panels {
                let p = ui.painter();
                p.frame.push(
                    ShapeBuilder::rect(rect.x, rect.y, rect.w, rect.h)
                        .color(bg)
                        .border_radius(esox_gfx::BorderRadius::uniform(corner_radius))
                        .build(),
                );
            }

            // Panel 0: Labels that overflow vertically.
            ui.sub_region(panels[0].0, 8.0, true, false, |ui| {
                ui.add_space(4.0);
                ui.muted_label("Clipped Labels");
                for i in 0..10 {
                    ui.label(&format!("Label {i} — this text is clipped by the panel boundary"));
                }
            });

            // Panel 1: Buttons.
            ui.sub_region(panels[1].0, 8.0, true, false, |ui| {
                ui.add_space(4.0);
                ui.muted_label("Buttons");
                for i in 0..8 {
                    ui.button(id!("grid_btn").wrapping_add(i), &format!("Button {i}"));
                }
            });

            // Panel 2: Long wrapped text.
            ui.sub_region(panels[2].0, 8.0, true, false, |ui| {
                ui.add_space(4.0);
                ui.muted_label("Overflowing Text");
                ui.label_wrapped("This panel has a lot of text that wraps and overflows vertically. The sub_region clips it automatically — no manual clip intersection or save/restore needed. The content extends well beyond the panel bounds but only the visible portion renders.");
                ui.label_wrapped("Second paragraph to demonstrate multi-block overflow clipping.");
            });

            // Panel 3: Mixed widgets.
            ui.sub_region(panels[3].0, 8.0, true, false, |ui| {
                ui.add_space(4.0);
                ui.muted_label("Mixed Widgets");
                ui.progress_bar(0.65);
                ui.button(id!("grid_action"), "Action");
                ui.progress_bar(0.3);
                ui.label("More content below");
                ui.button(id!("grid_action2"), "Another Button");
                ui.label("This overflows");
            });

            ui.add_space(20.0);

            // ═══════════════════════════════════════════════════════════
            // New Layout Primitives
            // ═══════════════════════════════════════════════════════════

            ui.header_label("NEW LAYOUT PRIMITIVES");
            ui.muted_label("col_spaced, spacer, scrollable_fill, padding(SpacingScale)");

            ui.add_space(12.0);

            // --- col_spaced: consistent vertical gap ---
            ui.header_label("COL_SPACED");
            ui.muted_label("Automatic vertical spacing between children (no manual add_space):");
            ui.card(|ui| {
                ui.col_spaced(12.0, |ui| {
                    ui.label("Item A — 12px gap below");
                    ui.label("Item B — 12px gap below");
                    ui.label("Item C — 12px gap below");
                    ui.button(id!("col_btn"), "Button at bottom");
                });
            });

            ui.add_space(12.0);

            // --- spacer: push content apart ---
            ui.header_label("SPACER IN ROW");
            ui.muted_label("spacer() absorbs remaining space — replaces fill_space():");
            ui.card(|ui| {
                ui.row(|ui| {
                    ui.label("Left");
                    ui.spacer();
                    ui.label("Right (pushed by spacer)");
                });
            });

            ui.add_space(8.0);

            ui.muted_label("Two spacers split a row into thirds:");
            ui.card(|ui| {
                ui.row(|ui| {
                    ui.label("A");
                    ui.spacer();
                    ui.label("B");
                    ui.spacer();
                    ui.label("C");
                });
            });

            ui.add_space(12.0);

            // --- padding with SpacingScale ---
            ui.header_label("PADDING WITH SPACING SCALE");
            ui.muted_label("padding(SpacingScale) instead of padding(16.0):");
            ui.flex_row().gap(8.0).show(|ui| {
                ui.padding(SpacingScale::Xs, |ui| { ui.button(id!("pad_xs"), "Xs 4px"); });
                ui.padding(SpacingScale::Sm, |ui| { ui.button(id!("pad_sm"), "Sm 8px"); });
                ui.padding(SpacingScale::Md, |ui| { ui.button(id!("pad_md"), "Md 12px"); });
                ui.padding(SpacingScale::Lg, |ui| { ui.button(id!("pad_lg"), "Lg 16px"); });
                ui.padding(SpacingScale::Xl, |ui| { ui.button(id!("pad_xl"), "Xl 24px"); });
            });

            ui.add_space(12.0);

            // --- scrollable_fill: auto-height scrollable ---
            ui.header_label("SCROLLABLE_FILL");
            ui.muted_label("scrollable_fill() fills remaining height — no manual math:");

            // Use sub_region to create a fixed-height panel (properly constrains region).
            let panel_rect = ui.allocate_rect(ui.region_width(), 200.0);
            let p = ui.painter();
            p.frame.push(
                ShapeBuilder::rect(panel_rect.x, panel_rect.y, panel_rect.w, panel_rect.h)
                    .color(p.theme.bg_raised)
                    .border_radius(esox_gfx::BorderRadius::uniform(p.theme.corner_radius))
                    .build(),
            );
            ui.sub_region(panel_rect, 0.0, true, false, |ui| {
                ui.padding(8.0, |ui| {
                    ui.col(|ui| {
                        ui.label_sized("Messages", TextSize::Lg);
                        ui.add_space(4.0);
                        ui.scrollable_fill(id!("demo_scroll_fill"), |ui| {
                            for i in 0..30 {
                                ui.label(&format!("  Message {i}: Lorem ipsum dolor sit amet"));
                            }
                        });
                    });
                });
            });

            ui.add_space(12.0);

            // --- Realistic layout: sidebar + content ---
            ui.header_label("REALISTIC LAYOUT");
            ui.muted_label("Combining primitives: row + col_spaced + spacer + scrollable_fill");

            let shell_rect = ui.allocate_rect(ui.region_width(), 280.0);
            let p = ui.painter();
            p.frame.push(
                ShapeBuilder::rect(shell_rect.x, shell_rect.y, shell_rect.w, shell_rect.h)
                    .color(p.theme.bg_surface)
                    .border_radius(esox_gfx::BorderRadius::uniform(p.theme.corner_radius))
                    .build(),
            );
            ui.sub_region(shell_rect, 0.0, true, false, |ui| {
                ui.flex_row().gap(0.0).show_flex(id!("app_shell"), |flex| {
                    // Sidebar
                    flex.item(FlexItem::default().basis(180.0), |ui| {
                        ui.padding(SpacingScale::Md, |ui| {
                            ui.col_spaced(4.0, |ui| {
                                ui.label_sized("Sidebar", TextSize::Lg);
                                ui.add_space(4.0);
                                for (i, item) in
                                    ["Dashboard", "Messages", "Settings", "Profile", "Help"]
                                        .iter()
                                        .enumerate()
                                {
                                    ui.button(id!("nav").wrapping_add(i as u64), item);
                                }
                                ui.spacer();
                                ui.muted_label("v0.1.0");
                            });
                        });
                    });

                    // Main content
                    flex.item(FlexItem::default().grow(1.0), |ui| {
                        ui.padding(SpacingScale::Md, |ui| {
                            ui.col_spaced(8.0, |ui| {
                                ui.row(|ui| {
                                    ui.label_sized("Inbox", TextSize::Lg);
                                    ui.spacer();
                                    ui.button(id!("compose"), "Compose");
                                });
                                ui.scrollable_fill(id!("inbox_scroll"), |ui| {
                                    for i in 0..25 {
                                        ui.label(&format!(
                                            "Email {i}: Re: Project update"
                                        ));
                                    }
                                });
                            });
                        });
                    });
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

    fn on_scale_changed(&mut self, scale_factor: f64, _gpu: &GpuContext) {
        let factor = scale_factor as f32;
        self.ui_state.scale_factor = factor;
        self.theme = Theme::dark().scaled(factor);
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
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = esox_platform::config::PlatformConfig {
        window: esox_platform::config::WindowConfig {
            title: "Layout Showcase".into(),
            width: Some(900),
            height: Some(700),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(App::new())).unwrap();
}
