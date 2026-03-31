//! Feature showcase — demonstrates all recently added widgets and capabilities:
//! stepper, drawer, popover, alert, skeleton, avatar, breadcrumb, accordion,
//! rating, CSS grid layout, 2D transforms, text decoration, and text transform.

use esox_gfx::{Color, Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::{
    ClipboardProvider, GridPlacement, GridTrack, Rect, RichText, Status, TextDecoration,
    TextRenderer, TextTransform, Theme, ThemeTransition, Transform2D, UiState, WidgetStyle, id,
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

struct FeatureShowcase {
    ui_state: UiState,
    text: Option<TextRenderer>,
    viewport: (u32, u32),

    // Theme
    is_dark: bool,
    base_light: Theme,
    base_dark: Theme,
    theme: Theme,
    transition: Option<ThemeTransition>,

    // Navigation
    tab_state: usize,

    // Widgets tab
    accordion_open: Option<usize>,
    rating_value: u8,
    alert_visible: bool,

    // Navigation tab
    stepper_step: usize,
    drawer_left_open: bool,
    drawer_right_open: bool,
    popover_open: bool,
    breadcrumb_location: Vec<&'static str>,

    // Grid tab
    grid_demo_choice: usize,

    // Transform tab
    anim_t: f32,
}

impl FeatureShowcase {
    fn new() -> Self {
        let base_light = Theme::light();
        let base_dark = Theme::dark();
        let theme = base_dark.clone();

        let mut ui_state = UiState::new();
        ui_state.clipboard = Some(Box::new(PlatformClipboard));

        Self {
            ui_state,
            text: None,
            viewport: (960, 720),
            is_dark: true,
            base_light,
            base_dark,
            theme,
            transition: None,
            tab_state: 0,
            accordion_open: Some(0),
            rating_value: 3,
            alert_visible: true,
            stepper_step: 1,
            drawer_left_open: false,
            drawer_right_open: false,
            popover_open: false,
            breadcrumb_location: vec!["Home", "Features", "Showcase"],
            grid_demo_choice: 0,
            anim_t: 0.0,
        }
    }
}

impl AppDelegate for FeatureShowcase {
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
        // Update theme transition.
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

        // Animate transform demo.
        self.anim_t += 1.0 / 60.0;
        if self.anim_t > std::f32::consts::TAU {
            self.anim_t -= std::f32::consts::TAU;
        }

        let anim_t = self.anim_t;

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

        ui.scrollable(id!("page_scroll"), scroll_h, |ui| {
            // ── App bar ──
            ui.surface(|ui| {
                ui.row(|ui| {
                    ui.heading("Feature Showcase");
                    ui.fill_space(100.0);
                    let toggle_label = if self.is_dark { "Light" } else { "Dark" };
                    let btn_bg = ui.theme().secondary_button_bg;
                    if ui
                        .small_button(id!("theme_toggle"), toggle_label, btn_bg)
                        .clicked
                    {
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

            // ── Tabs ──
            let selected = self.tab_state;
            ui.tabs(
                id!("main_tabs"),
                &mut self.tab_state,
                &["Widgets", "Navigation", "Grid", "Typography", "Transforms", "Veil P0"],
                |_ui, _selected| {},
            );

            ui.add_space(8.0);
            ui.padding(24.0, |ui| {
                ui.max_width(860.0, |ui| {
                    match selected {
                        // ════════════════════════════════════════════
                        // Tab 0 — Widgets
                        // ════════════════════════════════════════════
                        0 => {
                            // -- Alerts --
                            ui.card(|ui| {
                                ui.header_label("ALERTS");
                                ui.add_space(4.0);
                                ui.alert_info("Deployment is in progress. This may take a few minutes.");
                                ui.add_space(4.0);
                                ui.alert_success("All tests passed. Build artifacts are ready.");
                                ui.add_space(4.0);
                                ui.alert_warning("Your API key expires in 3 days.");
                                ui.add_space(4.0);
                                ui.alert_error("Connection lost. Retrying in 5 seconds...");
                                ui.add_space(8.0);

                                if self.alert_visible {
                                    let bg = ui.theme().toast_info_bg;
                                    let accent = ui.theme().accent;
                                    ui.alert_dismissable(
                                        id!("dismiss_alert"),
                                        "This alert can be dismissed. Click the X to close it.",
                                        &mut self.alert_visible,
                                        bg,
                                        accent,
                                    );
                                } else if ui
                                    .button(id!("reset_alert"), "Show Dismissable Alert")
                                    .clicked
                                {
                                    self.alert_visible = true;
                                }
                            });

                            ui.add_space(12.0);

                            // -- Avatars & Rating --
                            ui.card(|ui| {
                                ui.header_label("AVATARS");
                                ui.add_space(4.0);
                                ui.row(|ui| {
                                    ui.avatar("AC", 48.0);
                                    ui.avatar("BZ", 48.0);
                                    ui.avatar("JD", 48.0);
                                    ui.avatar("MK", 48.0);
                                    ui.avatar_colored("VIP", 48.0, Color::new(0.8, 0.2, 0.4, 1.0));
                                });
                                ui.add_space(4.0);
                                ui.muted_label("Auto-colored from initials, or custom background");

                                ui.add_space(12.0);
                                ui.separator();
                                ui.add_space(8.0);

                                ui.header_label("RATING");
                                ui.add_space(4.0);
                                ui.label("Interactive:");
                                ui.rating(id!("rating"), &mut self.rating_value, 5);
                                ui.add_space(4.0);
                                let stars_label = format!("You rated: {} / 5", self.rating_value);
                                ui.muted_label(&stars_label);
                                ui.add_space(8.0);
                                ui.label("Read-only displays:");
                                ui.row(|ui| {
                                    ui.rating_display(4.5, 5);
                                    ui.label(" 4.5");
                                });
                                ui.row(|ui| {
                                    ui.rating_display(2.0, 5);
                                    ui.label(" 2.0");
                                });
                            });

                            ui.add_space(12.0);

                            // -- Accordion --
                            ui.card(|ui| {
                                ui.header_label("ACCORDION");
                                ui.add_space(4.0);
                                ui.accordion(
                                    id!("faq"),
                                    &mut self.accordion_open,
                                    &["What is Esox?", "How does rendering work?", "Is it accessible?"],
                                    |ui, i| match i {
                                        0 => {
                                            ui.paragraph(
                                                id!("acc_p0"),
                                                "Esox is a GPU-accelerated, immediate-mode UI toolkit \
                                                 for native Linux applications, written in Rust. It \
                                                 targets small binaries, zero runtime dependencies, \
                                                 and first-class accessibility.",
                                            );
                                        }
                                        1 => {
                                            ui.paragraph(
                                                id!("acc_p1"),
                                                "Widgets emit QuadInstances into a Frame each tick. \
                                                 The platform layer submits them to wgpu as instanced \
                                                 draws. A damage tracker enables partial redraw for \
                                                 efficiency.",
                                            );
                                        }
                                        2 => {
                                            ui.paragraph(
                                                id!("acc_p2"),
                                                "Yes! Esox includes an AT-SPI2 accessibility bridge \
                                                 so screen readers can interact with the UI. All \
                                                 interactive widgets support keyboard navigation.",
                                            );
                                        }
                                        _ => {}
                                    },
                                );
                            });

                            ui.add_space(12.0);

                            // -- Skeleton loading placeholders --
                            ui.card(|ui| {
                                ui.header_label("SKELETON LOADING PLACEHOLDERS");
                                ui.add_space(4.0);
                                ui.muted_label("Animated shimmer effect for loading states:");
                                ui.add_space(8.0);

                                ui.row(|ui| {
                                    ui.skeleton_circle(48.0);
                                    ui.padding(4.0, |ui| {
                                        ui.skeleton(200.0, 16.0);
                                        ui.add_space(6.0);
                                        ui.skeleton(140.0, 12.0);
                                    });
                                });
                                ui.add_space(8.0);
                                ui.skeleton_text();
                                ui.add_space(4.0);
                                ui.skeleton_text();
                                ui.add_space(4.0);
                                ui.skeleton(280.0, 14.0);
                            });
                        }

                        // ════════════════════════════════════════════
                        // Tab 1 — Navigation
                        // ════════════════════════════════════════════
                        1 => {
                            // -- Breadcrumb --
                            ui.card(|ui| {
                                ui.header_label("BREADCRUMB");
                                ui.add_space(4.0);
                                let segments = self.breadcrumb_location.clone();
                                if let Some(clicked) =
                                    ui.breadcrumb(id!("breadcrumb"), &segments)
                                {
                                    self.breadcrumb_location.truncate(clicked + 1);
                                }
                                ui.add_space(8.0);
                                if self.breadcrumb_location.len() < 5 {
                                    let next = match self.breadcrumb_location.len() {
                                        3 => "Details",
                                        4 => "Settings",
                                        _ => "More",
                                    };
                                    if ui.button(id!("bc_deeper"), &format!("Go to {next}")).clicked {
                                        self.breadcrumb_location.push(next);
                                    }
                                }
                                ui.muted_label("Click a breadcrumb segment to navigate back");
                            });

                            ui.add_space(12.0);

                            // -- Stepper --
                            ui.card(|ui| {
                                ui.header_label("STEPPER");
                                ui.add_space(4.0);
                                ui.muted_label("Multi-step workflow progress indicator:");
                                ui.add_space(8.0);

                                if let Some(clicked) = ui.stepper(
                                    id!("stepper"),
                                    &["Account", "Profile", "Preferences", "Review"],
                                    self.stepper_step,
                                ) {
                                    self.stepper_step = clicked;
                                }

                                ui.add_space(12.0);
                                let step_desc = match self.stepper_step {
                                    0 => "Step 1: Set up your account credentials.",
                                    1 => "Step 2: Fill in your profile information.",
                                    2 => "Step 3: Configure notification preferences.",
                                    3 => "Step 4: Review and confirm your settings.",
                                    _ => "",
                                };
                                ui.label(step_desc);

                                ui.add_space(8.0);
                                ui.row(|ui| {
                                    if self.stepper_step > 0
                                        && ui
                                            .secondary_button(id!("step_prev"), "Previous")
                                            .clicked
                                    {
                                        self.stepper_step -= 1;
                                    }
                                    if self.stepper_step < 3 {
                                        if ui.button(id!("step_next"), "Next").clicked {
                                            self.stepper_step += 1;
                                        }
                                    } else if ui.button(id!("step_finish"), "Finish").clicked {
                                        ui.toast_success("Setup complete!");
                                        self.stepper_step = 0;
                                    }
                                });
                            });

                            ui.add_space(12.0);

                            // -- Drawer & Popover triggers --
                            ui.card(|ui| {
                                ui.header_label("OVERLAYS");
                                ui.add_space(4.0);

                                ui.columns_spaced(12.0, &[1.0, 1.0, 1.0], |ui, col| match col {
                                    0 => {
                                        if ui.button(id!("open_left_drawer"), "Left Drawer").clicked {
                                            self.drawer_left_open = true;
                                        }
                                    }
                                    1 => {
                                        if ui.button(id!("open_right_drawer"), "Right Drawer").clicked {
                                            self.drawer_right_open = true;
                                        }
                                    }
                                    2 => {
                                        let before_y = ui.cursor_y();
                                        let resp = ui.button(id!("open_popover"), "Popover");
                                        let anchor = Rect::new(
                                            ui.cursor_x(),
                                            before_y,
                                            ui.region_width(),
                                            ui.cursor_y() - before_y,
                                        );
                                        if resp.clicked {
                                            self.popover_open = !self.popover_open;
                                        }
                                        ui.popover(
                                            id!("demo_popover"),
                                            &mut self.popover_open,
                                            anchor,
                                            |ui| {
                                                ui.padding(8.0, |ui| {
                                                    ui.label("Popover Content");
                                                    ui.add_space(4.0);
                                                    ui.muted_label(
                                                        "This floats above the UI, anchored \
                                                         to the trigger button.",
                                                    );
                                                    ui.add_space(8.0);
                                                    ui.row(|ui| {
                                                        ui.avatar("PO", 28.0);
                                                        ui.label("Positioned overlay");
                                                    });
                                                    ui.add_space(4.0);
                                                    ui.rating_display(3.5, 5);
                                                });
                                            },
                                        );
                                    }
                                    _ => {}
                                });

                                ui.add_space(8.0);
                                ui.muted_label(
                                    "Drawers slide in from the edge. Popovers anchor to a widget.",
                                );
                            });
                        }

                        // ════════════════════════════════════════════
                        // Tab 2 — Grid
                        // ════════════════════════════════════════════
                        2 => {
                            ui.card(|ui| {
                                ui.header_label("CSS GRID LAYOUT");
                                ui.add_space(4.0);
                                ui.muted_label("Fractional units, fixed tracks, and cell spanning:");
                                ui.add_space(8.0);

                                ui.row(|ui| {
                                    if ui.small_button(id!("grid_basic"), "Basic", ui.theme().accent).clicked {
                                        self.grid_demo_choice = 0;
                                    }
                                    if ui.small_button(id!("grid_span"), "Spanning", ui.theme().green).clicked {
                                        self.grid_demo_choice = 1;
                                    }
                                    if ui.small_button(id!("grid_dashboard"), "Dashboard", ui.theme().amber).clicked {
                                        self.grid_demo_choice = 2;
                                    }
                                });
                                ui.add_space(8.0);

                                match self.grid_demo_choice {
                                    // Basic 3-column grid
                                    0 => {
                                        ui.grid(
                                            &[GridTrack::Fr(1.0), GridTrack::Fr(1.0), GridTrack::Fr(1.0)],
                                            &[GridTrack::Fixed(60.0), GridTrack::Fixed(60.0)],
                                        )
                                        .gap(8.0)
                                        .show(|grid| {
                                            for row in 0..2u16 {
                                                for col in 0..3u16 {
                                                    grid.cell(GridPlacement::at(col, row), |ui| {
                                                        ui.surface(|ui| {
                                                            ui.padding(8.0, |ui| {
                                                                let label = format!("Cell ({col}, {row})");
                                                                ui.label(&label);
                                                            });
                                                        });
                                                    });
                                                }
                                            }
                                        });
                                        ui.add_space(4.0);
                                        ui.muted_label("3 equal columns (Fr(1.0) each), 2 fixed-height rows");
                                    }

                                    // Grid with spanning
                                    1 => {
                                        ui.grid(
                                            &[GridTrack::Fr(1.0), GridTrack::Fr(2.0), GridTrack::Fr(1.0)],
                                            &[GridTrack::Fixed(50.0), GridTrack::Fixed(80.0), GridTrack::Fixed(50.0)],
                                        )
                                        .gap(8.0)
                                        .show(|grid| {
                                            // Header spanning all 3 columns
                                            grid.cell(GridPlacement::at(0, 0).span(3, 1), |ui| {
                                                ui.surface(|ui| {
                                                    ui.padding(8.0, |ui| {
                                                        ui.label("Header (spans 3 columns)");
                                                    });
                                                });
                                            });
                                            // Sidebar spanning 2 rows
                                            grid.cell(GridPlacement::at(0, 1).span(1, 2), |ui| {
                                                ui.surface(|ui| {
                                                    ui.padding(8.0, |ui| {
                                                        ui.label("Sidebar");
                                                        ui.muted_label("(2 rows)");
                                                    });
                                                });
                                            });
                                            // Main content
                                            grid.cell(GridPlacement::at(1, 1).span(2, 1), |ui| {
                                                ui.surface(|ui| {
                                                    ui.padding(8.0, |ui| {
                                                        ui.label("Main content area");
                                                        ui.muted_label("Fr(2.0) + Fr(1.0)");
                                                    });
                                                });
                                            });
                                            // Footer spanning 2 columns
                                            grid.cell(GridPlacement::at(1, 2).span(2, 1), |ui| {
                                                ui.surface(|ui| {
                                                    ui.padding(8.0, |ui| {
                                                        ui.label("Footer (spans 2 columns)");
                                                    });
                                                });
                                            });
                                        });
                                        ui.add_space(4.0);
                                        ui.muted_label("Header, sidebar, content, and footer with span()");
                                    }

                                    // Dashboard-style grid
                                    2 => {
                                        let accent = ui.theme().accent;
                                        let green = ui.theme().green;
                                        let amber = ui.theme().amber;
                                        let red = ui.theme().red;

                                        ui.grid(
                                            &[GridTrack::Fr(1.0), GridTrack::Fr(1.0), GridTrack::Fr(1.0), GridTrack::Fr(1.0)],
                                            &[GridTrack::Fixed(80.0), GridTrack::Fixed(100.0)],
                                        )
                                        .gap(8.0)
                                        .show(|grid| {
                                            // Stat cards
                                            let stats: &[(&str, &str, Color)] = &[
                                                ("Users", "12,847", accent),
                                                ("Revenue", "$48.2k", green),
                                                ("Orders", "1,429", amber),
                                                ("Errors", "23", red),
                                            ];
                                            for (i, (title, value, color)) in stats.iter().enumerate() {
                                                grid.cell(GridPlacement::at(i as u16, 0), |ui| {
                                                    ui.surface(|ui| {
                                                        ui.padding(8.0, |ui| {
                                                            ui.label_colored(title, *color);
                                                            ui.add_space(4.0);
                                                            ui.label_sized(value, esox_ui::TextSize::Xl);
                                                        });
                                                    });
                                                });
                                            }
                                            // Wide chart area
                                            grid.cell(GridPlacement::at(0, 1).span(3, 1), |ui| {
                                                ui.surface(|ui| {
                                                    ui.padding(8.0, |ui| {
                                                        ui.label("Chart area");
                                                        ui.progress_bar_colored(0.65, accent);
                                                        ui.add_space(4.0);
                                                        ui.progress_bar_colored(0.42, green);
                                                        ui.add_space(4.0);
                                                        ui.progress_bar_colored(0.88, amber);
                                                    });
                                                });
                                            });
                                            // Activity feed
                                            grid.cell(GridPlacement::at(3, 1), |ui| {
                                                ui.surface(|ui| {
                                                    ui.padding(8.0, |ui| {
                                                        ui.label("Activity");
                                                        ui.add_space(4.0);
                                                        ui.row(|ui| {
                                                            ui.avatar("AK", 20.0);
                                                            ui.muted_label("deployed");
                                                        });
                                                        ui.row(|ui| {
                                                            ui.avatar("SL", 20.0);
                                                            ui.muted_label("merged PR");
                                                        });
                                                    });
                                                });
                                            });
                                        });
                                        ui.add_space(4.0);
                                        ui.muted_label("4-column dashboard with spanning chart area");
                                    }
                                    _ => {}
                                }
                            });

                            ui.add_space(12.0);

                            ui.card(|ui| {
                                ui.header_label("GRID TRACK TYPES");
                                ui.add_space(4.0);
                                ui.grid(
                                    &[
                                        GridTrack::Fixed(120.0),
                                        GridTrack::Fr(1.0),
                                        GridTrack::Fr(2.0),
                                        GridTrack::Fixed(80.0),
                                    ],
                                    &[GridTrack::Fixed(48.0)],
                                )
                                .gap(8.0)
                                .show(|grid| {
                                    grid.cell(GridPlacement::at(0, 0), |ui| {
                                        ui.surface(|ui| {
                                            ui.padding(4.0, |ui| {
                                                ui.muted_label("Fixed(120)");
                                            });
                                        });
                                    });
                                    grid.cell(GridPlacement::at(1, 0), |ui| {
                                        ui.surface(|ui| {
                                            ui.padding(4.0, |ui| {
                                                ui.muted_label("Fr(1.0)");
                                            });
                                        });
                                    });
                                    grid.cell(GridPlacement::at(2, 0), |ui| {
                                        ui.surface(|ui| {
                                            ui.padding(4.0, |ui| {
                                                ui.muted_label("Fr(2.0)");
                                            });
                                        });
                                    });
                                    grid.cell(GridPlacement::at(3, 0), |ui| {
                                        ui.surface(|ui| {
                                            ui.padding(4.0, |ui| {
                                                ui.muted_label("Fixed(80)");
                                            });
                                        });
                                    });
                                });
                                ui.add_space(4.0);
                                ui.muted_label("Mix Fixed and Fr tracks. Fr distributes remaining space proportionally.");
                            });
                        }

                        // ════════════════════════════════════════════
                        // Tab 3 — Typography
                        // ════════════════════════════════════════════
                        3 => {
                            ui.card(|ui| {
                                ui.header_label("TEXT TRANSFORMS");
                                ui.add_space(4.0);

                                ui.label("Original: the quick brown fox");
                                ui.add_space(4.0);

                                ui.with_style(
                                    WidgetStyle {
                                        text_transform: Some(TextTransform::Uppercase),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("Uppercase: the quick brown fox");
                                    },
                                );
                                ui.add_space(4.0);

                                ui.with_style(
                                    WidgetStyle {
                                        text_transform: Some(TextTransform::Lowercase),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("Lowercase: THE QUICK BROWN FOX");
                                    },
                                );
                                ui.add_space(4.0);

                                ui.with_style(
                                    WidgetStyle {
                                        text_transform: Some(TextTransform::Capitalize),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("Capitalize: the quick brown fox");
                                    },
                                );
                            });

                            ui.add_space(12.0);

                            ui.card(|ui| {
                                ui.header_label("TEXT DECORATIONS");
                                ui.add_space(4.0);

                                ui.with_style(
                                    WidgetStyle {
                                        text_decoration: Some(TextDecoration::Underline),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("This text has an underline decoration");
                                    },
                                );
                                ui.add_space(8.0);

                                ui.with_style(
                                    WidgetStyle {
                                        text_decoration: Some(TextDecoration::Strikethrough),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("This text has a strikethrough decoration");
                                    },
                                );
                                ui.add_space(8.0);

                                ui.with_style(
                                    WidgetStyle {
                                        text_decoration: Some(TextDecoration::Both),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("This text has both underline and strikethrough");
                                    },
                                );
                            });

                            ui.add_space(12.0);

                            ui.card(|ui| {
                                ui.header_label("COMBINED STYLES");
                                ui.add_space(4.0);

                                let accent = ui.theme().accent;
                                let red = ui.theme().red;
                                let green = ui.theme().green;

                                // Uppercase + underline in accent color
                                ui.with_style(
                                    WidgetStyle {
                                        text_transform: Some(TextTransform::Uppercase),
                                        text_decoration: Some(TextDecoration::Underline),
                                        fg: Some(accent),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("important heading");
                                    },
                                );
                                ui.add_space(8.0);

                                // Strikethrough in red (deleted text)
                                ui.with_style(
                                    WidgetStyle {
                                        text_decoration: Some(TextDecoration::Strikethrough),
                                        fg: Some(red),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("This item has been removed from the list");
                                    },
                                );
                                ui.add_space(4.0);

                                // Capitalize + underline in green (new addition)
                                ui.with_style(
                                    WidgetStyle {
                                        text_transform: Some(TextTransform::Capitalize),
                                        text_decoration: Some(TextDecoration::Underline),
                                        fg: Some(green),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.label("this item was added as a replacement");
                                    },
                                );

                                ui.add_space(12.0);
                                ui.separator();
                                ui.add_space(8.0);

                                // Paragraph with decoration
                                ui.muted_label("Decorations also work on paragraphs:");
                                ui.add_space(4.0);
                                ui.with_style(
                                    WidgetStyle {
                                        text_decoration: Some(TextDecoration::Underline),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        ui.paragraph(
                                            id!("deco_paragraph"),
                                            "This paragraph demonstrates that text decorations \
                                             work seamlessly across word-wrapped multi-line text. \
                                             Each line receives its own underline, creating a \
                                             consistent visual effect throughout the block.",
                                        );
                                    },
                                );
                            });
                        }

                        // ════════════════════════════════════════════
                        // Tab 4 — Transforms
                        // ════════════════════════════════════════════
                        4 => {
                            ui.card(|ui| {
                                ui.header_label("2D TRANSFORMS");
                                ui.add_space(4.0);
                                ui.muted_label("GPU-level translate and scale on any widget:");
                                ui.add_space(12.0);

                                ui.columns_spaced(16.0, &[1.0, 1.0], |ui, col| match col {
                                    0 => {
                                        ui.label("Translation:");
                                        ui.add_space(8.0);
                                        ui.with_transform(Transform2D::translate(20.0, 0.0), |ui| {
                                            ui.button(id!("t_shift_right"), "Shifted +20px");
                                        });
                                        ui.add_space(4.0);
                                        ui.with_transform(Transform2D::translate(-10.0, 0.0), |ui| {
                                            ui.button(id!("t_shift_left"), "Shifted -10px");
                                        });
                                    }
                                    1 => {
                                        ui.label("Scale:");
                                        ui.add_space(8.0);
                                        ui.with_transform(Transform2D::scale(1.25), |ui| {
                                            ui.button(id!("t_scale_up"), "125%");
                                        });
                                        ui.add_space(8.0);
                                        ui.with_transform(Transform2D::scale(0.75), |ui| {
                                            ui.button(id!("t_scale_down"), "75%");
                                        });
                                    }
                                    _ => {}
                                });
                            });

                            ui.add_space(12.0);

                            ui.card(|ui| {
                                ui.header_label("ANIMATED TRANSFORMS");
                                ui.add_space(4.0);
                                ui.muted_label("Smooth translation and scale driven by a sine wave:");
                                ui.add_space(16.0);

                                let offset_x = anim_t.sin() * 30.0;
                                let scale = 1.0 + anim_t.sin().abs() * 0.3;

                                ui.with_transform(Transform2D::translate(offset_x, 0.0), |ui| {
                                    ui.label("Oscillating text");
                                });
                                ui.add_space(12.0);

                                ui.with_transform(Transform2D::scale(scale), |ui| {
                                    ui.label("Pulsing text");
                                });
                                ui.add_space(16.0);

                                // Combined: bouncing avatar
                                let bounce_y = (anim_t * 2.0).sin().abs() * -12.0;
                                ui.row(|ui| {
                                    ui.with_transform(Transform2D::translate(0.0, bounce_y), |ui| {
                                        ui.avatar("GO", 40.0);
                                    });
                                    ui.with_transform(
                                        Transform2D::translate(0.0, (bounce_y - 4.0).min(0.0)),
                                        |ui| {
                                            ui.avatar("ES", 40.0);
                                        },
                                    );
                                    ui.with_transform(
                                        Transform2D::translate(0.0, (bounce_y - 8.0).min(0.0)),
                                        |ui| {
                                            ui.avatar("OX", 40.0);
                                        },
                                    );
                                    ui.label("  Bouncing avatars");
                                });
                            });

                            ui.add_space(12.0);

                            ui.card(|ui| {
                                ui.header_label("TRANSFORMS + WIDGETS");
                                ui.add_space(4.0);
                                ui.muted_label("Transforms apply to any widget content:");
                                ui.add_space(8.0);

                                let scale_val = 0.9 + anim_t.cos().abs() * 0.2;
                                ui.with_transform(Transform2D::scale(scale_val), |ui| {
                                    ui.alert_info("This alert pulses gently via scale transform.");
                                });
                                ui.add_space(8.0);

                                ui.with_transform(
                                    Transform2D::translate(anim_t.sin() * 10.0, 0.0),
                                    |ui| {
                                        ui.rating_display(4.0, 5);
                                    },
                                );
                            });
                        }

                        // ════════════════════════════════════════════
                        // Tab 5 — Veil P0 Widgets
                        // ════════════════════════════════════════════
                        5 => {
                            // -- Rich Text: Background + Decorations --
                            ui.card(|ui| {
                                ui.header_label("RICH TEXT — BACKGROUNDS & DECORATIONS");
                                ui.add_space(4.0);

                                let code_bg = Color::new(0.18, 0.20, 0.25, 1.0);
                                let highlight_bg = Color::new(1.0, 0.92, 0.23, 0.25);

                                ui.rich_label(
                                    &RichText::new()
                                        .span("Use ")
                                        .code(" cargo build ", code_bg)
                                        .span(" to compile, or ")
                                        .code(" cargo run ", code_bg)
                                        .span(" to execute."),
                                );
                                ui.add_space(6.0);

                                ui.rich_label(
                                    &RichText::new()
                                        .span("This has ")
                                        .strikethrough("deleted text")
                                        .span(" and ")
                                        .underline("underlined text")
                                        .span(" in one line."),
                                );
                                ui.add_space(6.0);

                                ui.rich_label(
                                    &RichText::new()
                                        .span("Search result: ")
                                        .highlight("matching term", highlight_bg)
                                        .span(" found in document."),
                                );
                                ui.add_space(6.0);

                                ui.rich_label(
                                    &RichText::new()
                                        .bold("Bold")
                                        .span(" + ")
                                        .code(" inline code ", code_bg)
                                        .span(" + ")
                                        .strikethrough("struck")
                                        .span(" + ")
                                        .colored("colored", Color::new(0.4, 0.7, 1.0, 1.0))
                                        .span(" — all in one line."),
                                );
                            });

                            ui.add_space(12.0);

                            // -- Rich Text Wrapped --
                            ui.card(|ui| {
                                ui.header_label("RICH TEXT — WRAPPED WITH STYLES");
                                ui.add_space(4.0);

                                let code_bg = Color::new(0.18, 0.20, 0.25, 1.0);

                                ui.rich_label_wrapped(
                                    &RichText::new()
                                        .span("This is a longer paragraph that demonstrates ")
                                        .bold("word wrapping")
                                        .span(" with ")
                                        .code(" inline code ", code_bg)
                                        .span(" and ")
                                        .strikethrough("deleted content")
                                        .span(
                                            " interleaved. The text should flow naturally across \
                                             multiple lines while preserving per-word styling.",
                                        ),
                                );
                            });

                            ui.add_space(12.0);

                            // -- Avatar Status Dots --
                            ui.card(|ui| {
                                ui.header_label("AVATAR — STATUS DOTS");
                                ui.add_space(4.0);

                                ui.columns_spaced(
                                    8.0,
                                    &[1.0, 1.0, 1.0, 1.0],
                                    |ui, col| match col {
                                        0 => {
                                            ui.avatar_with_status("JD", 48.0, Status::Online);
                                            ui.add_space(4.0);
                                            ui.muted_label("Online");
                                        }
                                        1 => {
                                            ui.avatar_with_status("AB", 48.0, Status::Idle);
                                            ui.add_space(4.0);
                                            ui.muted_label("Idle");
                                        }
                                        2 => {
                                            ui.avatar_with_status(
                                                "MK",
                                                48.0,
                                                Status::DoNotDisturb,
                                            );
                                            ui.add_space(4.0);
                                            ui.muted_label("DND");
                                        }
                                        3 => {
                                            ui.avatar_with_status("ZZ", 48.0, Status::Offline);
                                            ui.add_space(4.0);
                                            ui.muted_label("Offline");
                                        }
                                        _ => {}
                                    },
                                );

                                ui.add_space(8.0);
                                ui.muted_label("Different sizes:");
                                ui.add_space(4.0);
                                ui.row(|ui| {
                                    ui.avatar_with_status("SM", 24.0, Status::Online);
                                    ui.avatar_with_status("MD", 40.0, Status::Online);
                                    ui.avatar_with_status("LG", 56.0, Status::Online);
                                    ui.avatar_with_status("XL", 72.0, Status::Online);
                                });
                            });

                            ui.add_space(12.0);

                            // -- Code Block --
                            ui.card(|ui| {
                                ui.header_label("CODE BLOCK");
                                ui.add_space(4.0);

                                let clicked = ui
                                    .code_block_lang(
                                        id!("code_demo"),
                                        "rust",
                                        "fn main() {\n    let greeting = \"Hello, Veil!\";\n    println!(\"{greeting}\");\n\n    for i in 0..5 {\n        println!(\"  count: {i}\");\n    }\n}",
                                    )
                                    .clicked;

                                if clicked {
                                    ui.label_colored(
                                        "Copied!",
                                        Color::new(0.3, 0.8, 0.4, 1.0),
                                    );
                                }

                                ui.add_space(8.0);

                                ui.code_block(
                                    id!("code_short"),
                                    "$ cargo run -p feature_showcase --release",
                                );
                            });

                            ui.add_space(12.0);

                            // -- Blockquote --
                            ui.card(|ui| {
                                ui.header_label("BLOCKQUOTE");
                                ui.add_space(4.0);

                                ui.blockquote(|ui| {
                                    ui.label("The best way to predict the future is to invent it.");
                                    ui.muted_label("— Alan Kay");
                                });

                                ui.add_space(8.0);

                                ui.blockquote_colored(
                                    Color::new(0.9, 0.3, 0.3, 1.0),
                                    |ui| {
                                        ui.label_colored(
                                            "Warning: this operation cannot be undone.",
                                            Color::new(0.9, 0.3, 0.3, 1.0),
                                        );
                                    },
                                );

                                ui.add_space(8.0);

                                // Nested blockquotes
                                ui.blockquote(|ui| {
                                    ui.label("Outer quote with nested reply:");
                                    ui.add_space(4.0);
                                    ui.blockquote_colored(
                                        Color::new(0.5, 0.5, 0.5, 0.6),
                                        |ui| {
                                            ui.muted_label("Inner quote — like a reply chain.");
                                        },
                                    );
                                });
                            });

                            ui.add_space(12.0);

                            // -- Spoiler --
                            ui.card(|ui| {
                                ui.header_label("SPOILER");
                                ui.add_space(4.0);

                                ui.label("Click the hidden blocks to reveal content:");
                                ui.add_space(6.0);

                                ui.spoiler(id!("spoiler1"), |ui| {
                                    ui.label("The cake is a lie.");
                                });

                                ui.add_space(8.0);

                                ui.spoiler(id!("spoiler2"), |ui| {
                                    ui.label("Spoiler with rich content:");
                                    ui.add_space(4.0);
                                    ui.code_block(
                                        id!("spoiler_code"),
                                        "secret_key = \"hunter2\"",
                                    );
                                });
                            });
                        }

                        _ => {}
                    }
                });
            });
        }); // page_scroll

        // -- Drawers (rendered outside scrollable, over everything) --
        ui.drawer(
            id!("left_drawer"),
            &mut self.drawer_left_open,
            300.0,
            |ui| {
                ui.padding(16.0, |ui| {
                    ui.heading("Navigation");
                    ui.add_space(12.0);
                    ui.avatar("US", 64.0);
                    ui.add_space(8.0);
                    ui.label("Signed in as User");
                    ui.muted_label("user@example.com");
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let items = ["Dashboard", "Projects", "Settings", "Help"];
                    for (i, item) in items.iter().enumerate() {
                        if ui
                            .button(id!("drawer_nav").wrapping_add(i as u64), item)
                            .clicked
                        {
                            ui.toast_info(&format!("Navigated to {item}"));
                        }
                        ui.add_space(4.0);
                    }
                });
            },
        );

        ui.drawer_right(
            id!("right_drawer"),
            &mut self.drawer_right_open,
            350.0,
            |ui| {
                ui.padding(16.0, |ui| {
                    ui.heading("Details Panel");
                    ui.add_space(12.0);

                    ui.header_label("ITEM INFO");
                    ui.add_space(4.0);
                    ui.row(|ui| {
                        ui.avatar("FE", 36.0);
                        ui.label("Feature Showcase");
                    });
                    ui.add_space(8.0);
                    ui.label("Rating:");
                    ui.rating_display(4.5, 5);
                    ui.add_space(8.0);
                    ui.label("Status:");
                    ui.alert_success("Published");
                    ui.add_space(12.0);

                    ui.separator();
                    ui.add_space(8.0);
                    ui.header_label("LOADING PREVIEW");
                    ui.add_space(4.0);
                    ui.skeleton(250.0, 16.0);
                    ui.add_space(4.0);
                    ui.skeleton_text();
                    ui.add_space(4.0);
                    ui.skeleton(180.0, 12.0);
                });
            },
        );

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
        self.transition.is_some() || self.ui_state.needs_continuous_redraw() || self.tab_state == 4 // Transforms tab has animations
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
            title: "Feature Showcase".into(),
            width: Some(960),
            height: Some(720),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(FeatureShowcase::new())).unwrap();
}
