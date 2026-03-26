use std::cell::Cell;
use std::collections::HashMap;

use esox_gfx::{Color, Frame, GpuContext, QuadInstance, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::{AppDelegate, Clipboard, MouseInputEvent};
use esox_ui::{
    ClipboardProvider, ColumnWidth, FieldStatus, InputState, ModalAction, Rect, RichText,
    SelectState, TabState, TableColumn, TableState, TextRenderer, Theme, ThemeBuilder,
    ThemeTransition, TreeState, UiState, VirtualScrollState, id,
};

// ── Static data ──────────────────────────────────────────────────────────────

const NAMES: &[&str] = &[
    "Alice Chen",
    "Bob Smith",
    "Carol Wu",
    "David Kim",
    "Eva Brown",
    "Frank Liu",
    "Grace Lee",
    "Henry Park",
    "Iris Wang",
    "Jack Jones",
    "Kate Yang",
    "Leo Torres",
    "Mia Clark",
    "Noah Adams",
    "Olivia Hall",
    "Paul Scott",
    "Quinn Ross",
    "Ruby Green",
    "Sam White",
    "Tina Black",
    "Uma Patel",
    "Victor Diaz",
    "Wendy Zhao",
    "Xander Brooks",
    "Yuki Tanaka",
    "Zara Ahmed",
    "Alex Morgan",
    "Blake Taylor",
    "Casey Jordan",
    "Drew Riley",
];

const EMAILS: &[&str] = &[
    "alice@company.com",
    "bob@company.com",
    "carol@company.com",
    "david@company.com",
    "eva@company.com",
    "frank@company.com",
    "grace@company.com",
    "henry@company.com",
    "iris@company.com",
    "jack@company.com",
    "kate@company.com",
    "leo@company.com",
    "mia@company.com",
    "noah@company.com",
    "olivia@company.com",
    "paul@company.com",
    "quinn@company.com",
    "ruby@company.com",
    "sam@company.com",
    "tina@company.com",
    "uma@company.com",
    "victor@company.com",
    "wendy@company.com",
    "xander@company.com",
    "yuki@company.com",
    "zara@company.com",
    "alex@company.com",
    "blake@company.com",
    "casey@company.com",
    "drew@company.com",
];

const DEPTS: &[&str] = &[
    "Engineering",
    "Design",
    "Marketing",
    "Sales",
    "Support",
    "Finance",
    "Legal",
    "HR",
];

const ROLES: &[&str] = &["Member", "Admin", "Owner", "Viewer"];

const SUBJECTS: &[&str] = &[
    "Q4 Planning Review",
    "Updated Design Specs",
    "New Feature Request",
    "Sprint Retrospective Notes",
    "Budget Approval Needed",
    "Infrastructure Migration Plan",
    "Customer Feedback Summary",
    "Team Offsite Logistics",
    "Security Audit Results",
    "Performance Review Templates",
    "Product Roadmap Update",
    "Onboarding Checklist",
    "Release Notes Draft",
    "API Documentation Review",
    "Weekly Status Update",
    "Holiday Schedule",
    "Training Resources",
    "Vendor Contract Renewal",
    "Office Supplies Request",
    "Conference Sponsorship",
];

const ACTIVITIES: &[&str] = &[
    "logged in",
    "updated their profile",
    "created a new document",
    "shared a file",
    "joined a meeting",
    "completed a task",
    "submitted a review",
    "uploaded a report",
    "invited a new member",
    "changed their password",
    "exported data",
    "archived a project",
];

// ── Google brand colors ──────────────────────────────────────────────────────

const GOOGLE_BLUE: Color = Color::new(0.259, 0.522, 0.957, 1.0); // #4285F4
const GOOGLE_RED: Color = Color::new(0.918, 0.263, 0.208, 1.0); // #EA4335
const GOOGLE_YELLOW: Color = Color::new(0.984, 0.737, 0.020, 1.0); // #FBBC05
const GOOGLE_GREEN: Color = Color::new(0.204, 0.659, 0.325, 1.0); // #34A853

fn google_light() -> Theme {
    let mut t = ThemeBuilder::from_light()
        .accent(GOOGLE_BLUE)
        .accent_dim(Color::new(0.259, 0.522, 0.957, 0.12))
        .accent_hover(Color::new(0.345, 0.580, 0.973, 1.0))
        .focus_ring_color(Color::new(0.259, 0.522, 0.957, 0.45))
        .green(GOOGLE_GREEN)
        .amber(GOOGLE_YELLOW)
        .red(GOOGLE_RED)
        .bg_base(Color::new(1.0, 1.0, 1.0, 1.0))
        .bg_surface(Color::new(0.973, 0.976, 0.980, 1.0)) // #F8F9FA
        .bg_raised(Color::new(0.945, 0.953, 0.957, 1.0)) // #F1F3F4
        .bg_input(Color::new(1.0, 1.0, 1.0, 1.0))
        .fg(Color::new(0.125, 0.129, 0.141, 1.0)) // #202124
        .fg_muted(Color::new(0.263, 0.275, 0.302, 1.0)) // #43464D
        .fg_dim(Color::new(0.337, 0.349, 0.376, 1.0)) // #565960
        .fg_label(Color::new(0.200, 0.212, 0.239, 1.0))
        .border(Color::new(0.855, 0.863, 0.878, 1.0)) // #DADCE0
        .corner_radius(8.0)
        .card_gap(12.0)
        .section_gap(16.0)
        .build();
    t.secondary_button_bg = Color::new(0.914, 0.937, 0.996, 1.0); // light blue tint
    t.secondary_button_hover = Color::new(0.843, 0.882, 0.992, 1.0);
    t.danger_button_bg = GOOGLE_RED;
    t.danger_button_hover = Color::new(0.800, 0.200, 0.160, 1.0);
    t.green_button_bg = Color::new(0.878, 0.957, 0.898, 1.0);
    t.fg_on_accent = Color::new(1.0, 1.0, 1.0, 1.0);
    t.table_zebra_bg = Color::new(0.955, 0.960, 0.968, 1.0);
    t.tab_indicator_height = 3.0;
    t.tree_indent = 28.0;
    t.toast_info_bg = Color::new(0.914, 0.937, 0.996, 1.0);
    t.toast_success_bg = Color::new(0.878, 0.957, 0.898, 1.0);
    t.toast_error_bg = Color::new(0.992, 0.906, 0.898, 1.0);
    t.toast_warning_bg = Color::new(0.996, 0.957, 0.886, 1.0);
    t
}

fn google_dark() -> Theme {
    let mut t = ThemeBuilder::from_dark()
        .accent(GOOGLE_BLUE)
        .accent_dim(Color::new(0.259, 0.522, 0.957, 0.18))
        .accent_hover(Color::new(0.400, 0.600, 0.980, 1.0))
        .focus_ring_color(Color::new(0.259, 0.522, 0.957, 0.50))
        .green(GOOGLE_GREEN)
        .amber(GOOGLE_YELLOW)
        .red(GOOGLE_RED)
        .corner_radius(8.0)
        .card_gap(12.0)
        .section_gap(16.0)
        .build();
    t.fg_on_accent = Color::new(1.0, 1.0, 1.0, 1.0);
    t.tab_indicator_height = 3.0;
    t.tree_indent = 28.0;
    t
}

// ── Clipboard ────────────────────────────────────────────────────────────────

struct PlatformClipboard;

impl ClipboardProvider for PlatformClipboard {
    fn read_text(&self) -> Option<String> {
        Clipboard::read(0).ok()
    }
    fn write_text(&self, text: &str) {
        let _ = Clipboard::write(text);
    }
}

// ── Application ──────────────────────────────────────────────────────────────

struct GoogleShowcase {
    ui_state: UiState,
    text: Option<TextRenderer>,
    viewport: (u32, u32),

    // Theme
    is_dark: bool,
    base_light: Theme,
    base_dark: Theme,
    theme: Theme,
    transition: Option<ThemeTransition>,
    pending_clear: Option<[f32; 4]>,

    // Navigation
    tab_state: TabState,
    search_input: InputState,

    // Dashboard
    upload_progress: f32,
    activity_scroll: VirtualScrollState,

    // Users
    user_search: InputState,
    user_table: TableState,
    org_tree: TreeState,
    add_user_open: bool,
    new_user_name: InputState,
    new_user_email: InputState,
    new_user_dept: SelectState,
    new_user_role: SelectState,

    // Settings
    profile_name: InputState,
    profile_email: InputState,
    profile_bio: InputState,
    pref_role: SelectState,
    pref_country: Option<usize>,
    font_size_value: f64,
    notif_toggle: InputState,
    checkboxes: HashMap<u64, InputState>,
    update_radio: InputState,
    delete_confirm_open: bool,

    // Messages
    msg_scroll: VirtualScrollState,
    selected_msg: Option<usize>,
    compose_open: bool,
    compose_to: InputState,
    compose_subject: InputState,
    compose_body: InputState,
}

impl GoogleShowcase {
    fn new() -> Self {
        let base_light = google_light();
        let base_dark = google_dark();
        let theme = base_light.clone();

        let mut ui_state = UiState::new();
        ui_state.clipboard = Some(Box::new(PlatformClipboard));

        let mut email = InputState::new();
        email.text = "bad-email@@".into();

        let mut radio = InputState::new();
        radio.text = "0".into();

        let bg = base_light.bg_base;
        Self {
            ui_state,
            text: None,
            viewport: (1100, 750),
            is_dark: false,
            base_light,
            base_dark,
            theme,
            transition: None,
            pending_clear: Some([bg.r, bg.g, bg.b, bg.a]),
            tab_state: TabState::new(),
            search_input: InputState::new(),
            upload_progress: 0.0,
            activity_scroll: VirtualScrollState::new(50),
            user_search: InputState::new(),
            user_table: TableState::new(),
            org_tree: {
                let mut t = TreeState::new();
                t.expanded.insert(id!("org_root"));
                t
            },
            add_user_open: false,
            new_user_name: InputState::new(),
            new_user_email: InputState::new(),
            new_user_dept: SelectState::new(),
            new_user_role: SelectState::new(),
            profile_name: InputState::new(),
            profile_email: email,
            profile_bio: InputState::new(),
            pref_role: SelectState::new(),
            pref_country: Some(0),
            font_size_value: 14.0,
            notif_toggle: InputState::new(),
            checkboxes: HashMap::new(),
            update_radio: radio,
            delete_confirm_open: false,
            msg_scroll: VirtualScrollState::new(1_000),
            selected_msg: None,
            compose_open: false,
            compose_to: InputState::new(),
            compose_subject: InputState::new(),
            compose_body: InputState::new(),
        }
    }

    // ── Tab renderers ────────────────────────────────────────────────────

    fn tab_dashboard(
        ui: &mut esox_ui::Ui<'_>,
        upload_progress: f32,
        activity_scroll: &mut VirtualScrollState,
    ) {
        let wide = matches!(
            ui.width_class(),
            esox_ui::WidthClass::Medium | esox_ui::WidthClass::Expanded
        );

        if wide {
            ui.columns_spaced(16.0, &[1.0, 1.0], |ui, col| match col {
                0 => Self::dashboard_left(ui, upload_progress),
                1 => Self::dashboard_right(ui, activity_scroll),
                _ => {}
            });
        } else {
            Self::dashboard_left(ui, upload_progress);
            Self::dashboard_right(ui, activity_scroll);
        }

        ui.status_bar("Connected", "Workspace Console v1.0.0");
    }

    fn dashboard_left(ui: &mut esox_ui::Ui<'_>, upload_progress: f32) {
        // KPI row 1.
        ui.columns_spaced(12.0, &[1.0, 1.0], |ui, col| match col {
            0 => {
                ui.card(|ui| {
                    ui.section("ACTIVE USERS", |ui| {
                        ui.label_sized("12,847", esox_ui::TextSize::Xxl);
                        let green = ui.theme().green;
                        ui.progress_bar_colored(0.82, green);
                        ui.row(|ui| {
                            ui.label_colored("+12.3%", ui.theme().green);
                            ui.badge(142);
                        });
                    });
                });
            }
            1 => {
                ui.card(|ui| {
                    ui.section("STORAGE USED", |ui| {
                        ui.label_sized("2.4 TB", esox_ui::TextSize::Lg);
                        let amber = ui.theme().amber;
                        ui.progress_bar_colored(0.68, amber);
                        ui.muted_label("of 3.5 TB allocated");
                    });
                });
            }
            _ => {}
        });

        // KPI row 2.
        ui.columns_spaced(12.0, &[1.0, 1.0], |ui, col| match col {
            0 => {
                ui.card(|ui| {
                    ui.section("MONTHLY REVENUE", |ui| {
                        ui.label_sized("$847K", esox_ui::TextSize::Xxl);
                        let accent = ui.theme().accent;
                        ui.progress_bar_colored(0.75, accent);
                        ui.row(|ui| {
                            ui.chip(id!("chip_q4"), "Q4");
                            ui.label_colored("+8.1%", ui.theme().green);
                        });
                    });
                });
            }
            1 => {
                ui.card(|ui| {
                    ui.section("UPTIME", |ui| {
                        ui.label_sized("99.97%", esox_ui::TextSize::Lg);
                        let green = ui.theme().green;
                        ui.progress_bar_colored(0.9997, green);
                        ui.muted_label("Last 30 days");
                    });
                });
            }
            _ => {}
        });

        // Upload progress card.
        ui.card(|ui| {
            ui.section("DEPLOYMENT PROGRESS", |ui| {
                let accent = ui.theme().accent;
                ui.progress_bar_colored(upload_progress, accent);
                ui.row_centered(|ui| {
                    ui.spinner();
                    let pct = format!(" Building... {:.0}%", upload_progress * 100.0);
                    ui.muted_label(&pct);
                });
                ui.muted_label("Deploying: frontend-v2.1.3");
                ui.row(|ui| {
                    ui.muted_label("ETA: ~3 min");
                    ui.fill_space(80.0);
                    ui.ghost_button(id!("deploy_details"), "View Details");
                });
            });
        });

        // About card.
        ui.card(|ui| {
            let accent = ui.theme().accent;
            let green = ui.theme().green;
            ui.rich_label_wrapped(
                &RichText::new()
                    .span("Powered by ")
                    .colored_bold("esox", accent)
                    .span(" — a GPU-accelerated, ")
                    .colored("zero-dependency", green)
                    .span(" UI toolkit for Linux."),
            );
        });
    }

    fn dashboard_right(ui: &mut esox_ui::Ui<'_>, activity_scroll: &mut VirtualScrollState) {
        // System status.
        ui.card(|ui| {
            ui.section("SYSTEM STATUS", |ui| {
                let green = ui.theme().green;
                let amber = ui.theme().amber;
                let red = ui.theme().red;

                ui.label("CPU");
                ui.progress_bar_colored(0.42, green);
                ui.muted_label("42%");

                ui.label("Memory");
                ui.progress_bar_colored(0.78, amber);
                ui.muted_label("78%");

                ui.label("Disk");
                ui.progress_bar_colored(0.91, red);
                ui.muted_label("91%");
            });
        });

        // Quick actions.
        ui.card(|ui| {
            ui.section("QUICK ACTIONS", |ui| {
                ui.secondary_button(id!("qa_export"), "Export Report");
                ui.secondary_button(id!("qa_logs"), "View Logs");
                ui.secondary_button(id!("qa_docs"), "Documentation");
            });
        });

        // Recent activity.
        ui.card(|ui| {
            ui.section("RECENT ACTIVITY", |ui| {
                activity_scroll.item_count = 50;
                ui.virtual_scroll(id!("activity_vs"), activity_scroll, 28.0, 200.0, |ui, i| {
                    let name = NAMES[i % NAMES.len()];
                    let action = ACTIVITIES[i % ACTIVITIES.len()];
                    let muted = ui.theme().fg_muted;
                    let hours = (i % 24) + 1;
                    ui.rich_label(
                        &RichText::new()
                            .bold(name)
                            .span(&format!(" {action}"))
                            .colored(&format!(" \u{00B7} {hours}h ago"), muted),
                    );
                });
            });
        });
    }

    fn tab_users(
        ui: &mut esox_ui::Ui<'_>,
        user_search: &mut InputState,
        add_user_open: &mut bool,
        user_table: &mut TableState,
        org_tree: &mut TreeState,
    ) {
        // Search + Add User.
        ui.columns_spaced(12.0, &[3.0, 1.0], |ui, col| match col {
            0 => {
                ui.text_input(id!("user_search"), user_search, "Search users...");
            }
            1 => {
                let accent = ui.theme().accent;
                if ui
                    .small_button(id!("add_user_btn"), "+ Add User", accent)
                    .clicked
                {
                    *add_user_open = true;
                }
            }
            _ => {}
        });
        ui.muted_label("Filter by name, email, or department");

        // Employee table.
        ui.card(|ui| {
            ui.section("EMPLOYEE DIRECTORY", |ui| {
                let columns = [
                    TableColumn::new("#", ColumnWidth::Fixed(30.0)).not_sortable(),
                    TableColumn::new("Name", ColumnWidth::Weight(2.0)),
                    TableColumn::new("Email", ColumnWidth::Weight(3.0)),
                    TableColumn::new("Department", ColumnWidth::Weight(1.5)),
                    TableColumn::new("Role", ColumnWidth::Weight(1.0)),
                    TableColumn::new("Status", ColumnWidth::Weight(1.0)),
                ];

                ui.table(
                    id!("user_table"),
                    user_table,
                    &columns,
                    30,
                    10,
                    |ui, row, col| match col {
                        0 => {
                            let s = format!("{}", row + 1);
                            ui.label(&s);
                        }
                        1 => ui.label(NAMES[row % NAMES.len()]),
                        2 => ui.label(EMAILS[row % EMAILS.len()]),
                        3 => ui.label(DEPTS[row % DEPTS.len()]),
                        4 => ui.label(ROLES[row % ROLES.len()]),
                        5 => match row % 3 {
                            0 => ui.status_pill_success("Active"),
                            1 => ui.status_pill_warning("Away"),
                            _ => ui.status_pill_error("Offline"),
                        },
                        _ => {}
                    },
                );
            });
        });

        // Organization tree.
        ui.card(|ui| {
            ui.section("ORGANIZATION", |ui| {
                let r = ui.tree_node(id!("org_root"), org_tree, "Workspace Inc.", true);
                ui.animated_tree_indent(id!("org_root_anim"), r.expanded, |ui| {
                    let r2 = ui.tree_node(id!("org_eng"), org_tree, "Engineering", true);
                    ui.animated_tree_indent(id!("org_eng_anim"), r2.expanded, |ui| {
                        ui.tree_node(id!("org_frontend"), org_tree, "Frontend", false);
                        ui.tree_node(id!("org_backend"), org_tree, "Backend", false);
                        ui.tree_node(id!("org_devops"), org_tree, "DevOps", false);
                    });

                    let r3 = ui.tree_node(id!("org_design"), org_tree, "Design", true);
                    ui.animated_tree_indent(id!("org_design_anim"), r3.expanded, |ui| {
                        ui.tree_node(id!("org_ux"), org_tree, "UX Research", false);
                        ui.tree_node(id!("org_visual"), org_tree, "Visual Design", false);
                    });

                    let r4 = ui.tree_node(id!("org_marketing"), org_tree, "Marketing", true);
                    ui.animated_tree_indent(id!("org_marketing_anim"), r4.expanded, |ui| {
                        ui.tree_node(id!("org_content"), org_tree, "Content", false);
                        ui.tree_node(id!("org_growth"), org_tree, "Growth", false);
                    });

                    ui.tree_node(id!("org_sales"), org_tree, "Sales", false);
                });
            });
        });
    }

    #[allow(clippy::too_many_arguments)] // demo function passing individual widget states
    fn tab_settings(
        ui: &mut esox_ui::Ui<'_>,
        profile_name: &mut InputState,
        profile_email: &mut InputState,
        profile_bio: &mut InputState,
        pref_role: &mut SelectState,
        pref_country: &mut Option<usize>,
        font_size_value: &mut f64,
        notif_toggle: &mut InputState,
        checkboxes: &mut HashMap<u64, InputState>,
        update_radio: &mut InputState,
        delete_confirm_open: &mut bool,
    ) {
        // Profile.
        ui.card(|ui| {
            ui.section("PROFILE", |ui| {
                ui.form_field("Display Name", FieldStatus::None, "", |ui| {
                    ui.text_input(id!("profile_name"), profile_name, "Your name...")
                });

                let email_status =
                    if profile_email.text.contains('@') && !profile_email.text.contains("@@") {
                        FieldStatus::Success
                    } else if profile_email.text.is_empty() {
                        FieldStatus::None
                    } else {
                        FieldStatus::Error
                    };
                let email_helper = match email_status {
                    FieldStatus::Error => "Please enter a valid email address",
                    FieldStatus::Success => "Looks good!",
                    _ => "",
                };
                ui.form_field("Email", email_status, email_helper, |ui| {
                    ui.text_input_validated(
                        id!("profile_email"),
                        profile_email,
                        "you@example.com",
                        email_status,
                    )
                });

                ui.form_field("Bio", FieldStatus::None, "", |ui| {
                    ui.text_area_wrapped(
                        id!("profile_bio"),
                        profile_bio,
                        4,
                        "Tell us about yourself...",
                    )
                });
            });
        });

        // Actions.
        ui.card(|ui| {
            if ui.button(id!("save_settings"), "Save Changes").clicked {
                ui.toast_success("Settings saved successfully!");
            }
            ui.ghost_button(id!("cancel_settings"), "Cancel");
            ui.hyperlink(
                id!("terms_link"),
                "Terms of Service",
                "https://example.com/terms",
            );
        });

        // Preferences.
        ui.card(|ui| {
            ui.section("PREFERENCES", |ui| {
                ui.form_field("Role", FieldStatus::None, "", |ui| {
                    ui.select(
                        id!("pref_role"),
                        pref_role,
                        &["Administrator", "Editor", "Viewer", "Guest"],
                    )
                });

                ui.form_field("Region", FieldStatus::None, "", |ui| {
                    ui.combobox(
                        id!("pref_country"),
                        &[
                            "United States",
                            "Canada",
                            "United Kingdom",
                            "Germany",
                            "France",
                            "Japan",
                            "Australia",
                            "Brazil",
                            "India",
                            "South Korea",
                        ],
                        pref_country,
                    )
                });

                ui.form_field("Font Size", FieldStatus::None, "", |ui| {
                    ui.slider_f64(id!("pref_fontsize"), font_size_value, 10.0, 24.0)
                });
                let font_label = format!("{} px", (*font_size_value).round() as i32);
                ui.muted_label(&font_label);
            });
        });

        // Notifications.
        ui.card(|ui| {
            ui.section("NOTIFICATIONS", |ui| {
                ui.toggle(id!("notif_toggle"), notif_toggle, "Push notifications");

                let cb1 = id!("cb_newsletter");
                let state1 = checkboxes.entry(cb1).or_default();
                ui.checkbox(cb1, state1, "Subscribe to newsletter");

                let cb2 = id!("cb_marketing");
                let state2 = checkboxes.entry(cb2).or_default();
                ui.checkbox(cb2, state2, "Marketing emails");

                ui.section("UPDATE FREQUENCY", |ui| {
                    ui.radio(id!("freq_daily"), update_radio, 0, "Daily digest");
                    ui.radio(id!("freq_weekly"), update_radio, 1, "Weekly summary");
                    ui.radio(id!("freq_none"), update_radio, 2, "None");
                });
            });
        });

        // Danger zone.
        ui.card(|ui| {
            ui.section("DANGER ZONE", |ui| {
                ui.label_wrapped(
                    "Permanently delete your account and all associated data. \
                     This action cannot be undone.",
                );
                if ui
                    .danger_button(id!("delete_account"), "Delete Account")
                    .clicked
                {
                    *delete_confirm_open = true;
                }
            });
        });
    }

    fn tab_messages(
        ui: &mut esox_ui::Ui<'_>,
        msg_scroll: &mut VirtualScrollState,
        selected_msg: &mut Option<usize>,
        compose_open: &mut bool,
    ) {
        let current_sel = *selected_msg;
        let open_compose = Cell::new(false);

        // Split pane: message list | detail.
        ui.split_pane_h(
            id!("msg_split"),
            0.35,
            |ui| {
                // Inbox header + Compose button.
                ui.row(|ui| {
                    let accent = ui.theme().accent;
                    ui.rich_label(&RichText::new().colored_bold("Inbox", accent));
                    ui.fill_space(80.0);
                    if ui.small_button(id!("compose_btn"), "New", accent).clicked {
                        open_compose.set(true);
                    }
                });

                msg_scroll.item_count = 1_000;
                ui.virtual_scroll(id!("msg_list"), msg_scroll, 48.0, 500.0, |ui, i| {
                    let sender = NAMES[i % NAMES.len()];
                    let subject = SUBJECTS[i % SUBJECTS.len()];
                    let is_sel = current_sel == Some(i);
                    let is_unread = i % 3 == 0;
                    let muted = ui.theme().fg_muted;
                    let hours = (i % 48) + 1;
                    let ts = format!(" \u{00B7} {hours}h");

                    if is_sel {
                        let accent = ui.theme().accent;
                        ui.rich_label(
                            &RichText::new()
                                .colored_bold(sender, accent)
                                .colored(&ts, muted),
                        );
                    } else if is_unread {
                        ui.rich_label(&RichText::new().bold(sender).colored(&ts, muted));
                    } else {
                        ui.rich_label(&RichText::new().span(sender).colored(&ts, muted));
                    }
                    ui.muted_label(subject);
                    ui.separator();
                });
            },
            |ui| match current_sel {
                Some(i) => {
                    let sender = NAMES[i % NAMES.len()];
                    let subject = SUBJECTS[i % SUBJECTS.len()];
                    let hours = (i % 72) + 1;

                    ui.heading(subject);
                    let muted_text = format!("From: {}  ·  {}h ago", sender, hours);
                    ui.muted_label(&muted_text);
                    ui.separator();
                    let body = format!(
                        "Hi team,\n\n\
                         This is message #{} regarding {}.\n\n\
                         Please review the attached materials and provide \
                         your feedback by end of week. Let me know if you \
                         have any questions.\n\n\
                         Best regards,\n{}",
                        i,
                        subject.to_lowercase(),
                        sender,
                    );
                    ui.label_wrapped(&body);

                    ui.section_break();
                    ui.row(|ui| {
                        if ui.button(id!("msg_reply"), "Reply").clicked {
                            ui.toast_info("Reply started");
                        }
                        if ui.ghost_button(id!("msg_fwd"), "Forward").clicked {
                            ui.toast_info("Forward started");
                        }
                    });
                }
                None => {
                    ui.section_break();
                    if ui
                        .empty_state_with_action(
                            id!("compose_from_empty"),
                            "Select a message to read",
                            "Compose Message",
                        )
                        .clicked
                    {
                        open_compose.set(true);
                    }
                }
            },
        );
        if open_compose.get() {
            *compose_open = true;
        }
    }
}

impl AppDelegate for GoogleShowcase {
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

        // Animate deployment progress.
        self.upload_progress += 1.0 / 60.0 * 0.15;
        if self.upload_progress > 1.0 {
            self.upload_progress = 0.0;
        }

        let upload_progress = self.upload_progress;
        let selected_tab = self.tab_state.selected;

        let text = self.text.as_mut().unwrap();
        let vp = Rect::new(0.0, 0.0, self.viewport.0 as f32, self.viewport.1 as f32);

        // Fill viewport with theme background color.
        let bg = self.theme.bg_base;
        frame.push(QuadInstance {
            rect: [0.0, 0.0, vp.w, vp.h],
            uv: [0.0; 4],
            color: [bg.r, bg.g, bg.b, bg.a],
            border_radius: [0.0; 4],
            sdf_params: [0.0; 4],
            flags: [0.0; 4],
            clip_rect: [0.0; 4],
            color2: [0.0; 4],
            extra: [0.0; 4],
        });

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
            // ── Top bar ──
            ui.surface(|ui| {
                ui.row(|ui| {
                    let accent = ui.theme().accent;
                    ui.rich_label(&RichText::new().colored_bold("Workspace Console", accent));
                    ui.fill_space(200.0);
                    ui.text_input(
                        id!("top_search"),
                        &mut self.search_input,
                        "Search... (Ctrl+K)",
                    );
                    let btn_bg = ui.theme().secondary_button_bg;
                    let toggle_label = if self.is_dark { "Light" } else { "Dark" };
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
                        let bg = target.bg_base;
                        self.pending_clear = Some([bg.r, bg.g, bg.b, bg.a]);
                        self.transition =
                            Some(ThemeTransition::new(self.theme.clone(), target, 300.0));
                    }
                });
            });

            // ── Tab bar ──
            ui.tabs(
                id!("main_tabs"),
                &mut self.tab_state,
                &["Dashboard", "Users", "Settings", "Messages"],
                |_ui, _sel| {},
            );

            ui.padding(16.0, |ui| {
                ui.max_width(960.0, |ui| match selected_tab {
                    0 => Self::tab_dashboard(ui, upload_progress, &mut self.activity_scroll),
                    1 => Self::tab_users(
                        ui,
                        &mut self.user_search,
                        &mut self.add_user_open,
                        &mut self.user_table,
                        &mut self.org_tree,
                    ),
                    2 => Self::tab_settings(
                        ui,
                        &mut self.profile_name,
                        &mut self.profile_email,
                        &mut self.profile_bio,
                        &mut self.pref_role,
                        &mut self.pref_country,
                        &mut self.font_size_value,
                        &mut self.notif_toggle,
                        &mut self.checkboxes,
                        &mut self.update_radio,
                        &mut self.delete_confirm_open,
                    ),
                    3 => Self::tab_messages(
                        ui,
                        &mut self.msg_scroll,
                        &mut self.selected_msg,
                        &mut self.compose_open,
                    ),
                    _ => {}
                });
            });
        }); // page_scroll

        // ── Modals (drawn on top of everything) ──

        // Add User modal.
        ui.modal(
            id!("add_user_modal"),
            &mut self.add_user_open,
            "Add New User",
            450.0,
            |ui| {
                ui.form_field("Full Name", FieldStatus::None, "", |ui| {
                    ui.text_input(id!("new_name"), &mut self.new_user_name, "John Doe")
                });
                ui.form_field("Email", FieldStatus::None, "", |ui| {
                    ui.text_input(
                        id!("new_email"),
                        &mut self.new_user_email,
                        "user@company.com",
                    )
                });
                ui.form_field("Department", FieldStatus::None, "", |ui| {
                    ui.select(
                        id!("new_dept"),
                        &mut self.new_user_dept,
                        &[
                            "Engineering",
                            "Design",
                            "Marketing",
                            "Sales",
                            "Support",
                            "Finance",
                        ],
                    )
                });
                ui.form_field("Role", FieldStatus::None, "", |ui| {
                    ui.select(
                        id!("new_role"),
                        &mut self.new_user_role,
                        &["Member", "Admin", "Owner"],
                    )
                });
                if ui.button(id!("create_user_btn"), "Create User").clicked {
                    ui.toast_success("User created successfully!");
                }
            },
        );

        // Compose modal.
        ui.modal(
            id!("compose_modal"),
            &mut self.compose_open,
            "Compose Message",
            500.0,
            |ui| {
                ui.form_field("To", FieldStatus::None, "", |ui| {
                    ui.text_input(
                        id!("compose_to"),
                        &mut self.compose_to,
                        "recipient@example.com",
                    )
                });
                ui.form_field("Subject", FieldStatus::None, "", |ui| {
                    ui.text_input(
                        id!("compose_subject"),
                        &mut self.compose_subject,
                        "Subject...",
                    )
                });
                ui.form_field("Message", FieldStatus::None, "", |ui| {
                    ui.text_area_wrapped(
                        id!("compose_body"),
                        &mut self.compose_body,
                        6,
                        "Type your message...",
                    )
                });
                if ui.button(id!("send_btn"), "Send").clicked {
                    ui.toast_success("Message sent!");
                }
            },
        );

        // Delete confirm dialog.
        let action = ui.modal_confirm(
            id!("delete_confirm"),
            &mut self.delete_confirm_open,
            "Delete Account",
            "Are you sure? This action cannot be undone.",
        );
        if action == ModalAction::Confirm {
            ui.toast_error("Account has been deleted.");
        } else if action == ModalAction::Cancel {
            ui.toast_info("Deletion cancelled.");
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

    fn take_clear_color(&mut self) -> Option<[f32; 4]> {
        self.pending_clear.take()
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
        self.base_light = google_light().scaled(factor);
        self.base_dark = google_dark().scaled(factor);
        self.theme = if self.is_dark {
            self.base_dark.clone()
        } else {
            self.base_light.clone()
        };
        let bg = self.theme.bg_base;
        self.pending_clear = Some([bg.r, bg.g, bg.b, bg.a]);
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = PlatformConfig {
        window: WindowConfig {
            title: "Workspace Console".into(),
            width: Some(1100),
            height: Some(750),
            ..Default::default()
        },
        ..Default::default()
    };

    esox_platform::run(config, Box::new(GoogleShowcase::new())).unwrap();
}
