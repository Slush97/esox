//! Declaration logic for the FrameCore Tonight showcase.
//!
//! Everything here is windowless: the tree is a pure function of [`DemoState`],
//! so the regression harness can run the exact declaration the windowed shell
//! uses and prove its once-only and commit contracts headlessly.

use std::cell::Cell;
use std::collections::VecDeque;

use esox_ui::declaration::{
    ButtonStyle, CrossAxisAlignment, DeclarationStyle, DeclarationUi, GridStyle, GridTrack,
    TextStyle,
};
use esox_ui::frame_core::{Color, CommittedScene, LogicalRect, TextProperties, WidgetId};

pub const ROOT: WidgetId = WidgetId(1);
pub const HEADER_ROW: WidgetId = WidgetId(2);
pub const TITLE: WidgetId = WidgetId(3);
pub const HEADER_SPACER: WidgetId = WidgetId(4);
pub const HEADER_TAG: WidgetId = WidgetId(5);

pub const CONTROL_BAR: WidgetId = WidgetId(10);
pub const BTN_VISIBLE: WidgetId = WidgetId(11);
pub const BTN_ENABLED: WidgetId = WidgetId(12);
pub const BTN_STALE: WidgetId = WidgetId(13);
pub const STATE_CHIP: WidgetId = WidgetId(14);
pub const STATE_TEXT: WidgetId = WidgetId(15);
pub const FOCUS_CHIP: WidgetId = WidgetId(16);
pub const FOCUS_TEXT: WidgetId = WidgetId(17);
pub const CAPTURE_CHIP: WidgetId = WidgetId(18);
pub const CAPTURE_TEXT: WidgetId = WidgetId(19);
pub const CONTROL_SPACER: WidgetId = WidgetId(20);
pub const GEN_CHIP: WidgetId = WidgetId(21);
pub const GEN_TEXT: WidgetId = WidgetId(22);

pub const MAIN_ROW: WidgetId = WidgetId(30);
pub const SHOWCASE_COL: WidgetId = WidgetId(31);
pub const SUBJECT_ROW: WidgetId = WidgetId(32);

pub const TARGET_CARD: WidgetId = WidgetId(40);
pub const TARGET_TITLE: WidgetId = WidgetId(41);
pub const TARGET_BUTTON: WidgetId = WidgetId(42);
pub const TARGET_SWATCHES: WidgetId = WidgetId(43);
pub const SWATCH_A: WidgetId = WidgetId(44);
pub const SWATCH_B: WidgetId = WidgetId(45);
pub const SWATCH_C: WidgetId = WidgetId(46);
pub const TARGET_CAPTION: WidgetId = WidgetId(47);

pub const SIBLING_CARD: WidgetId = WidgetId(50);
pub const SIBLING_TITLE: WidgetId = WidgetId(51);
pub const SIBLING_BODY: WidgetId = WidgetId(52);
pub const SIBLING_FILL: WidgetId = WidgetId(53);

pub const GRID_PANEL: WidgetId = WidgetId(60);
pub const GRID_TITLE: WidgetId = WidgetId(61);
pub const SHOWCASE_GRID: WidgetId = WidgetId(62);
pub const CELL_FIXED: WidgetId = WidgetId(63);
pub const CELL_FIXED_TEXT: WidgetId = WidgetId(64);
pub const CELL_CENTER: WidgetId = WidgetId(65);
pub const CELL_CENTER_DOT: WidgetId = WidgetId(66);
pub const CELL_CENTER_TEXT: WidgetId = WidgetId(67);
pub const CELL_AUTO_BUTTON: WidgetId = WidgetId(68);
pub const CELL_MINMAX: WidgetId = WidgetId(69);
pub const CELL_MINMAX_TEXT: WidgetId = WidgetId(70);
pub const CELL_CLIP: WidgetId = WidgetId(71);
pub const CELL_CLIP_TEXT: WidgetId = WidgetId(72);
pub const CELL_BORDER: WidgetId = WidgetId(73);

pub const INSPECTOR_COL: WidgetId = WidgetId(80);
pub const INSPECTOR_TITLE: WidgetId = WidgetId(81);
pub const INSPECTOR_SUBJECT: WidgetId = WidgetId(82);
pub const SELECT_ROW: WidgetId = WidgetId(83);
pub const SEL_TARGET_CARD: WidgetId = WidgetId(90);
pub const SEL_TARGET_BUTTON: WidgetId = WidgetId(91);
pub const SEL_SIBLING: WidgetId = WidgetId(92);
pub const SEL_GRID: WidgetId = WidgetId(93);
pub const SEL_CLIP: WidgetId = WidgetId(94);

pub const LOG_PANEL: WidgetId = WidgetId(100);
pub const LOG_TITLE: WidgetId = WidgetId(101);
const INSPECTOR_ROW_BASE: u64 = 200;
const LOG_LINE_BASE: u64 = 300;

/// Container bodies executed by one [`declare_showcase`] application.
pub const CONTAINER_CLOSURES: u32 = 36;

/// Inspector rows are declared unconditionally so the container count is
/// constant across frames.
pub const INSPECTOR_ROWS: usize = 12;

/// Event-log lines visible in the log panel.
pub const LOG_LINES: usize = 7;

/// Counts container-body executions to prove the once-only contract.
#[derive(Debug, Default)]
pub struct DeclarationProbe {
    closures: Cell<u32>,
}

impl DeclarationProbe {
    pub fn record(&self) {
        self.closures.set(self.closures.get() + 1);
    }

    /// Executions since the last call, resetting the counter.
    pub fn take(&self) -> u32 {
        self.closures.replace(0)
    }
}

/// Application-owned state; the declaration is a pure function of this.
#[derive(Debug)]
pub struct DemoState {
    pub target_hidden: bool,
    pub target_disabled: bool,
    pub target_pressed: bool,
    pub hovered: Option<WidgetId>,
    pub selected: WidgetId,
    pub inspector: Vec<(String, String)>,
    pub generation_label: String,
    pub focus_label: String,
    pub capture_label: String,
    pub activation_label: String,
    pub log: VecDeque<String>,
    pub flash: f32,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            target_hidden: false,
            target_disabled: false,
            target_pressed: false,
            hovered: None,
            selected: TARGET_CARD,
            inspector: placeholder_inspector(),
            generation_label: "gen —".to_owned(),
            focus_label: "focus: —".to_owned(),
            capture_label: "capture: —".to_owned(),
            activation_label: "activations: 0".to_owned(),
            log: VecDeque::new(),
            flash: 0.0,
        }
    }
}

/// Responses observed while the declaration executed.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShowcaseFrame {
    pub toggle_hidden: bool,
    pub toggle_disabled: bool,
    pub stale_requested: bool,
    pub target_clicked: bool,
    pub target_pressed: bool,
    pub select: Option<WidgetId>,
}

fn channel(byte: u8) -> f32 {
    let srgb = f32::from(byte) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert an sRGB byte triple to the linear color space the renderer expects.
pub fn srgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgba(channel(r), channel(g), channel(b), 1.0)
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::rgba(color.r, color.g, color.b, alpha)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let lerp = |x: f32, y: f32| x + (y - x) * t;
    Color::rgba(
        lerp(a.r, b.r),
        lerp(a.g, b.g),
        lerp(a.b, b.b),
        lerp(a.a, b.a),
    )
}

pub fn background() -> Color {
    srgb(11, 14, 20)
}

fn panel() -> Color {
    srgb(20, 25, 34)
}

fn panel_alt() -> Color {
    srgb(28, 34, 46)
}

fn panel_hover() -> Color {
    srgb(37, 45, 60)
}

fn stroke() -> Color {
    srgb(48, 57, 75)
}

fn text_hi() -> Color {
    srgb(214, 221, 232)
}

fn text_lo() -> Color {
    srgb(134, 144, 164)
}

fn accent() -> Color {
    srgb(88, 142, 255)
}

fn accent_deep() -> Color {
    srgb(52, 96, 196)
}

fn good() -> Color {
    srgb(63, 182, 139)
}

fn warn() -> Color {
    srgb(229, 164, 85)
}

fn bad() -> Color {
    srgb(224, 108, 117)
}

fn label(size: f32, weight: u16, color: Color) -> TextStyle {
    TextStyle::default()
        .properties(TextProperties {
            font_size: size,
            font_weight: weight,
            ..TextProperties::default()
        })
        .color(color)
}

fn control_button(state: &DemoState, id: WidgetId, width: f32) -> ButtonStyle {
    let hovered = state.hovered == Some(id);
    ButtonStyle::default()
        .layout(DeclarationStyle::new().size(width, 30.0))
        .fill(if hovered { panel_hover() } else { panel_alt() })
        .border(if hovered { accent() } else { stroke() }, 1.0)
        .text(label(13.0, 400, text_hi()))
}

fn select_button(state: &DemoState, id: WidgetId, target: WidgetId, width: f32) -> ButtonStyle {
    let selected = state.selected == target;
    let hovered = state.hovered == Some(id);
    ButtonStyle::default()
        .layout(DeclarationStyle::new().size(width, 24.0))
        .fill(if selected {
            accent_deep()
        } else if hovered {
            panel_hover()
        } else {
            panel_alt()
        })
        .border(if selected { accent() } else { stroke() }, 1.0)
        .text(label(
            11.0,
            400,
            if selected { text_hi() } else { text_lo() },
        ))
}

/// Human-readable name for the interactive and inspectable ids.
pub fn widget_label(id: WidgetId) -> &'static str {
    match id {
        TARGET_CARD => "target card",
        TARGET_BUTTON => "demo button",
        SIBLING_CARD => "sibling card",
        SHOWCASE_GRID => "showcase grid",
        CELL_CLIP => "clip cell",
        BTN_VISIBLE => "visibility toggle",
        BTN_ENABLED => "enable toggle",
        BTN_STALE => "stale-click trigger",
        CELL_AUTO_BUTTON => "auto-track button",
        SEL_TARGET_CARD | SEL_TARGET_BUTTON | SEL_SIBLING | SEL_GRID | SEL_CLIP => {
            "inspector selector"
        }
        _ => "widget",
    }
}

/// Inspector rows shown before the first scene commits.
pub fn placeholder_inspector() -> Vec<(String, String)> {
    inspector_keys()
        .into_iter()
        .map(|key| (key.to_owned(), "—".to_owned()))
        .collect()
}

fn inspector_keys() -> [&'static str; INSPECTOR_ROWS] {
    [
        "origin",
        "size",
        "eff. hidden",
        "eff. disabled",
        "paint",
        "hit",
        "semantic",
        "focus order",
        "eff. clip",
        "damage",
        "sem. disabled",
        "generation",
    ]
}

/// Snapshot one committed node's scene participation for display next frame.
///
/// This reads only the already-committed scene, never geometry mid-declaration,
/// and formats logical values without rounding.
pub fn inspect(scene: &CommittedScene, id: WidgetId) -> Vec<(String, String)> {
    let Some(node) = scene.node(id) else {
        return placeholder_inspector();
    };
    let flag = |present: bool| {
        if present {
            "yes".to_owned()
        } else {
            "no".to_owned()
        }
    };
    let rect = |r: LogicalRect| format!("{}, {}, {}, {}", r.x, r.y, r.width, r.height);
    let values = [
        format!("{}, {}", node.bounds.x, node.bounds.y),
        format!("{} x {}", node.bounds.width, node.bounds.height),
        node.effective_hidden.to_string(),
        node.effective_disabled.to_string(),
        flag(node.paint_bounds.is_some()),
        flag(node.hit_bounds.is_some()),
        flag(node.semantic_bounds.is_some()),
        flag(scene.focus_order.contains(&id)),
        node.effective_clip.map_or("none".to_owned(), rect),
        flag(node.current_damage_bounds.is_some()),
        scene.semantics.node(id).map_or("—".to_owned(), |semantic| {
            semantic.properties.disabled.to_string()
        }),
        scene.generation.to_string(),
    ];
    inspector_keys()
        .into_iter()
        .map(str::to_owned)
        .zip(values)
        .collect()
}

/// Declare the complete showcase tree exactly once for the current frame.
pub fn declare_showcase(
    ui: &mut DeclarationUi<'_>,
    state: &DemoState,
    probe: &DeclarationProbe,
) -> ShowcaseFrame {
    let mut out = ShowcaseFrame::default();
    ui.column(
        ROOT,
        DeclarationStyle::new()
            .gap(10.0)
            .padding(14.0)
            .background(background()),
        |ui| {
            probe.record();
            header(ui, probe);
            control_bar(ui, state, probe, &mut out);
            ui.row(
                MAIN_ROW,
                DeclarationStyle::new().gap(10.0).flex_grow(1.0),
                |ui| {
                    probe.record();
                    showcase_column(ui, state, probe, &mut out);
                    inspector_column(ui, state, probe, &mut out);
                },
            );
            log_panel(ui, state, probe);
        },
    );
    out
}

fn header(ui: &mut DeclarationUi<'_>, probe: &DeclarationProbe) {
    ui.row(
        HEADER_ROW,
        DeclarationStyle::new()
            .gap(10.0)
            .cross_axis_alignment(CrossAxisAlignment::Center),
        |ui| {
            probe.record();
            ui.text(TITLE, "FrameCore Tonight", label(18.0, 700, text_hi()));
            ui.row(
                HEADER_SPACER,
                DeclarationStyle::new().flex_grow(1.0),
                |ui| {
                    probe.record();
                    let _ = ui;
                },
            );
            ui.text(
                HEADER_TAG,
                "current-generation scene pipeline",
                label(12.0, 400, text_lo()),
            );
        },
    );
}

fn control_bar(
    ui: &mut DeclarationUi<'_>,
    state: &DemoState,
    probe: &DeclarationProbe,
    out: &mut ShowcaseFrame,
) {
    let (state_text, state_color) = if state.target_hidden {
        ("HIDDEN", warn())
    } else if state.target_disabled {
        ("DISABLED", bad())
    } else {
        ("LIVE", good())
    };
    ui.row(
        CONTROL_BAR,
        DeclarationStyle::new()
            .gap(8.0)
            .padding(9.0)
            .background(panel())
            .cross_axis_alignment(CrossAxisAlignment::Center),
        |ui| {
            probe.record();
            let visible_label = if state.target_hidden {
                "Show target"
            } else {
                "Hide target"
            };
            out.toggle_hidden = ui
                .button(
                    BTN_VISIBLE,
                    visible_label,
                    control_button(state, BTN_VISIBLE, 118.0),
                )
                .clicked;
            let enabled_label = if state.target_disabled {
                "Enable target"
            } else {
                "Disable target"
            };
            out.toggle_disabled = ui
                .button(
                    BTN_ENABLED,
                    enabled_label,
                    control_button(state, BTN_ENABLED, 132.0),
                )
                .clicked;
            out.stale_requested = ui
                .button(
                    BTN_STALE,
                    "Queue stale click",
                    control_button(state, BTN_STALE, 150.0),
                )
                .clicked;
            chip(ui, STATE_CHIP, STATE_TEXT, state_text, state_color, probe);
            chip(
                ui,
                FOCUS_CHIP,
                FOCUS_TEXT,
                &state.focus_label,
                text_lo(),
                probe,
            );
            chip(
                ui,
                CAPTURE_CHIP,
                CAPTURE_TEXT,
                &state.capture_label,
                text_lo(),
                probe,
            );
            ui.row(
                CONTROL_SPACER,
                DeclarationStyle::new().flex_grow(1.0),
                |ui| {
                    probe.record();
                    let _ = ui;
                },
            );
            chip(
                ui,
                GEN_CHIP,
                GEN_TEXT,
                &state.generation_label,
                accent(),
                probe,
            );
        },
    );
}

fn chip(
    ui: &mut DeclarationUi<'_>,
    id: WidgetId,
    text_id: WidgetId,
    content: &str,
    color: Color,
    probe: &DeclarationProbe,
) {
    let content = content.to_owned();
    ui.column(
        id,
        DeclarationStyle::new()
            .padding(6.0)
            .background(with_alpha(color, 0.14)),
        move |ui| {
            probe.record();
            ui.text(text_id, content, label(11.0, 700, color));
        },
    );
}

fn showcase_column(
    ui: &mut DeclarationUi<'_>,
    state: &DemoState,
    probe: &DeclarationProbe,
    out: &mut ShowcaseFrame,
) {
    ui.column(
        SHOWCASE_COL,
        DeclarationStyle::new().gap(10.0).flex_grow(1.0),
        |ui| {
            probe.record();
            subject_row(ui, state, probe, out);
            grid_panel(ui, state, probe, out);
        },
    );
}

fn subject_row(
    ui: &mut DeclarationUi<'_>,
    state: &DemoState,
    probe: &DeclarationProbe,
    out: &mut ShowcaseFrame,
) {
    ui.row(SUBJECT_ROW, DeclarationStyle::new().gap(10.0), |ui| {
        probe.record();
        ui.column(
            TARGET_CARD,
            DeclarationStyle::new()
                .gap(8.0)
                .padding(12.0)
                .width(240.0)
                .background(panel())
                .clip_children()
                .with_hidden(state.target_hidden)
                .with_disabled(state.target_disabled),
            |ui| {
                probe.record();
                ui.text(TARGET_TITLE, "State subject", label(13.0, 700, text_hi()));
                let hovered = state.hovered == Some(TARGET_BUTTON);
                let fill = if state.target_pressed {
                    accent_deep()
                } else if hovered {
                    mix(accent(), text_hi(), 0.15)
                } else {
                    accent()
                };
                let response = ui.button(
                    TARGET_BUTTON,
                    "Demo action",
                    ButtonStyle::default()
                        .layout(DeclarationStyle::new().size(120.0, 34.0))
                        .fill(fill)
                        .border(accent_deep(), 1.0)
                        .text(label(13.0, 700, srgb(12, 16, 24))),
                );
                out.target_clicked = response.clicked;
                out.target_pressed = response.pressed;
                ui.row(TARGET_SWATCHES, DeclarationStyle::new().gap(6.0), |ui| {
                    probe.record();
                    ui.solid_rect(SWATCH_A, DeclarationStyle::new().size(26.0, 26.0), accent());
                    ui.solid_rect(SWATCH_B, DeclarationStyle::new().size(26.0, 26.0), good());
                    ui.solid_rect(SWATCH_C, DeclarationStyle::new().size(26.0, 26.0), warn());
                });
                ui.text(
                    TARGET_CAPTION,
                    &state.activation_label,
                    label(11.0, 400, text_lo()),
                );
            },
        );
        let sibling_bg = mix(panel(), accent_deep(), state.flash * 0.45);
        ui.column(
            SIBLING_CARD,
            DeclarationStyle::new()
                .gap(8.0)
                .padding(12.0)
                .flex_grow(1.0)
                .background(sibling_bg)
                .clip_children(),
            |ui| {
                probe.record();
                ui.text(SIBLING_TITLE, "Reflow sibling", label(13.0, 700, text_hi()));
                ui.text(
                    SIBLING_BODY,
                    "Collapses of the state subject reflow this card in the same committed frame.",
                    label(12.0, 400, text_lo()),
                );
                ui.solid_rect(
                    SIBLING_FILL,
                    DeclarationStyle::new()
                        .flex_grow(1.0)
                        .min_size(None, Some(10.0)),
                    with_alpha(stroke(), 0.5),
                );
            },
        );
    });
}

fn grid_panel(
    ui: &mut DeclarationUi<'_>,
    state: &DemoState,
    probe: &DeclarationProbe,
    out: &mut ShowcaseFrame,
) {
    ui.column(
        GRID_PANEL,
        DeclarationStyle::new()
            .gap(8.0)
            .padding(12.0)
            .flex_grow(1.0)
            .background(panel()),
        |ui| {
            probe.record();
            ui.text(
                GRID_TITLE,
                "Grid: fixed 150 / fraction 1 / auto — column gap 10, row gap 16",
                label(13.0, 700, text_hi()),
            );
            ui.grid(
                SHOWCASE_GRID,
                GridStyle::new(vec![
                    GridTrack::Fixed(150.0),
                    GridTrack::Fraction(1.0),
                    GridTrack::Auto,
                ])
                .gaps(10.0, 16.0),
                |ui| {
                    probe.record();
                    ui.column(
                        CELL_FIXED,
                        DeclarationStyle::new().padding(10.0).background(panel_alt()),
                        |ui| {
                            probe.record();
                            ui.text(
                                CELL_FIXED_TEXT,
                                "fixed 150 track",
                                label(12.0, 400, text_hi()),
                            );
                        },
                    );
                    ui.column(
                        CELL_CENTER,
                        DeclarationStyle::new()
                            .gap(5.0)
                            .height(72.0)
                            .background(panel_alt())
                            .main_axis_alignment(
                                esox_ui::declaration::MainAxisAlignment::Center,
                            )
                            .cross_axis_alignment(CrossAxisAlignment::Center),
                        |ui| {
                            probe.record();
                            ui.solid_rect(
                                CELL_CENTER_DOT,
                                DeclarationStyle::new().size(16.0, 16.0),
                                accent(),
                            );
                            ui.text(
                                CELL_CENTER_TEXT,
                                "centered both axes",
                                label(11.0, 400, text_lo()),
                            );
                        },
                    );
                    out.select = out.select.or(ui
                        .button(
                            CELL_AUTO_BUTTON,
                            "auto track",
                            control_button(state, CELL_AUTO_BUTTON, 104.0),
                        )
                        .clicked
                        .then_some(SHOWCASE_GRID));
                    ui.column(
                        CELL_MINMAX,
                        DeclarationStyle::new()
                            .padding(8.0)
                            .background(panel_alt())
                            .min_size(Some(120.0), Some(40.0))
                            .max_size(None, Some(52.0)),
                        |ui| {
                            probe.record();
                            ui.text(
                                CELL_MINMAX_TEXT,
                                "min 120x40, max h 52",
                                label(11.0, 400, text_lo()),
                            );
                        },
                    );
                    ui.column(
                        CELL_CLIP,
                        DeclarationStyle::new()
                            .padding(8.0)
                            .height(52.0)
                            // A zero minimum keeps the unwrapped text's intrinsic
                            // width from inflating the fraction track's floor.
                            .min_size(Some(0.0), None)
                            .background(panel_alt())
                            .clip_children(),
                        |ui| {
                            probe.record();
                            ui.text(
                                CELL_CLIP_TEXT,
                                "clipped: this single unwrapped line keeps going well past the cell bounds to make the effective clip visible",
                                label(12.0, 400, warn()),
                            );
                        },
                    );
                    ui.border(
                        CELL_BORDER,
                        DeclarationStyle::new().size(110.0, 52.0),
                        stroke(),
                        1.5,
                    );
                },
            );
        },
    );
}

fn inspector_column(
    ui: &mut DeclarationUi<'_>,
    state: &DemoState,
    probe: &DeclarationProbe,
    out: &mut ShowcaseFrame,
) {
    ui.column(
        INSPECTOR_COL,
        DeclarationStyle::new()
            .gap(6.0)
            .padding(12.0)
            .width(330.0)
            .min_size(Some(330.0), None)
            .background(panel())
            .clip_children(),
        |ui| {
            probe.record();
            ui.text(INSPECTOR_TITLE, "Inspector", label(13.0, 700, text_hi()));
            ui.text(
                INSPECTOR_SUBJECT,
                format!("subject: {}", widget_label(state.selected)),
                label(11.0, 400, accent()),
            );
            ui.row(SELECT_ROW, DeclarationStyle::new().gap(5.0), |ui| {
                probe.record();
                let choices = [
                    (SEL_TARGET_CARD, TARGET_CARD, "card", 52.0),
                    (SEL_TARGET_BUTTON, TARGET_BUTTON, "button", 60.0),
                    (SEL_SIBLING, SIBLING_CARD, "sibling", 60.0),
                    (SEL_GRID, SHOWCASE_GRID, "grid", 48.0),
                    (SEL_CLIP, CELL_CLIP, "clip", 46.0),
                ];
                for (id, target, name, width) in choices {
                    if ui
                        .button(id, name, select_button(state, id, target, width))
                        .clicked
                    {
                        out.select = Some(target);
                    }
                }
            });
            for (index, (key, value)) in state.inspector.iter().enumerate() {
                let base = INSPECTOR_ROW_BASE + 3 * index as u64;
                let (key, value) = (key.clone(), value.clone());
                ui.row(
                    WidgetId(base),
                    DeclarationStyle::new().gap(8.0),
                    move |ui| {
                        probe.record();
                        ui.text(
                            WidgetId(base + 1),
                            key,
                            label(11.0, 400, text_lo()).layout(DeclarationStyle::new().width(96.0)),
                        );
                        ui.text(WidgetId(base + 2), value, label(11.0, 400, text_hi()));
                    },
                );
            }
        },
    );
}

fn log_panel(ui: &mut DeclarationUi<'_>, state: &DemoState, probe: &DeclarationProbe) {
    ui.column(
        LOG_PANEL,
        DeclarationStyle::new()
            .gap(3.0)
            .padding(10.0)
            .height(148.0)
            .background(panel())
            .clip_children(),
        |ui| {
            probe.record();
            ui.text(LOG_TITLE, "Event log", label(12.0, 700, text_hi()));
            let start = state.log.len().saturating_sub(LOG_LINES);
            for (slot, line) in state.log.iter().skip(start).enumerate() {
                ui.text(
                    WidgetId(LOG_LINE_BASE + slot as u64),
                    line.clone(),
                    label(11.0, 400, text_lo()),
                );
            }
        },
    );
}
