//! Recursive node renderer — walks the AST and calls `Ui` methods.

use esox_markup::{Node, WidgetKind};

use super::id_gen::IdCtx;
use super::state::MarkupState;
use super::{Action, ActionKind};
use crate::rich_text::{FontWeight, RichText};
use crate::text::TruncationMode;
use crate::widgets::table::{ColumnWidth, TableColumn};
use crate::Ui;

use super::resolve;

// ── Public entry ────────────────────────────────────────────────────────

pub(crate) fn render_nodes(
    ui: &mut Ui<'_>,
    nodes: &[Node],
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    for (i, node) in nodes.iter().enumerate() {
        render_node(ui, node, i, state, id_ctx, actions);
    }
}

// ── Dispatch ────────────────────────────────────────────────────────────

fn render_node(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    // Check for transition properties — if present, build an animated style.
    let transitions = resolve::transition_props(node);

    let style = match &transitions {
        Some(props) if !props.is_empty() => {
            let base_id = id_ctx.widget_id(node.prop_str("bind"), child_index);
            let theme = ui.theme().clone();
            let ctx = resolve::TransitionCtx {
                transitions: props,
                base_id,
                duration: node.prop_f32("duration").unwrap_or(200.0),
                easing: resolve::easing(node),
                spring: resolve::spring_config(node),
            };
            resolve::build_animated_style(node, &theme, ui, state, &ctx)
        }
        _ => resolve::build_style(node, ui.theme()),
    };

    // Wrap in with_style if needed, then dispatch to the widget renderer.
    match style {
        Some(s) => ui.with_style(s, |ui| {
            dispatch(ui, node, child_index, state, id_ctx, actions);
        }),
        None => dispatch(ui, node, child_index, state, id_ctx, actions),
    }
}

fn dispatch(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    match &node.kind {
        // ── Text (leaf, no state) ───────────────────────────────
        WidgetKind::Label => render_label(ui, node),
        WidgetKind::Heading => {
            ui.heading(node.text.as_deref().unwrap_or(""));
        }
        WidgetKind::Paragraph => {
            let id = id_ctx.widget_id(node.prop_str("bind"), child_index);
            ui.paragraph(id, node.text.as_deref().unwrap_or(""));
        }
        WidgetKind::Separator => ui.separator(),
        WidgetKind::Space => {
            let amount = node.prop_f32("amount").unwrap_or(8.0);
            ui.add_space(amount);
        }
        WidgetKind::RichText => render_rich_text(ui, node),

        // ── Display (leaf, no state) ────────────────────────────
        WidgetKind::Progress => render_progress(ui, node, child_index, id_ctx),
        WidgetKind::Spinner => {
            if let Some(size) = node.prop_f32("size") {
                ui.spinner_sized(size);
            } else {
                ui.spinner();
            }
        }
        WidgetKind::Badge => render_badge(ui, node),
        WidgetKind::Avatar => render_avatar(ui, node),
        WidgetKind::CodeBlock => render_code_block(ui, node, child_index, id_ctx),
        WidgetKind::Skeleton => render_skeleton(ui, node),
        WidgetKind::StatusPill => render_pill(ui, node),
        WidgetKind::Alert => render_alert(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::StatusBar => {
            let left = node.prop_str("left").unwrap_or("");
            let right = node.prop_str("right").unwrap_or("");
            ui.status_bar(left, right);
        }
        WidgetKind::EmptyState => {
            ui.empty_state(node.text.as_deref().unwrap_or("No items"));
        }

        // ── Buttons (leaf with ID) ──────────────────────────────
        WidgetKind::Button => render_button(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Link => {
            let bind = node.prop_str("bind");
            let id = id_ctx.widget_id(bind, child_index);
            let text = node.text.as_deref().unwrap_or("");
            let href = node.prop_str("href").unwrap_or("");
            let resp = ui.hyperlink(id, text, href);
            check_click(resp.clicked, node, bind, child_index, id_ctx, actions);
        }
        WidgetKind::Chip => {
            let bind = node.prop_str("bind");
            let id = id_ctx.widget_id(bind, child_index);
            let text = node.text.as_deref().unwrap_or("");
            let resp = if let Some(color) = resolve::color_prop(node, "color", ui.theme()) {
                ui.chip_colored(id, text, color)
            } else {
                ui.chip(id, text)
            };
            check_click(resp.clicked, node, bind, child_index, id_ctx, actions);
        }

        // ── Form inputs (leaf with state) ───────────────────────
        WidgetKind::Input => render_input(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Textarea => render_textarea(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Checkbox => render_checkbox(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Toggle => render_toggle(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Radio => render_radio(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Slider => render_slider(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Select => render_select(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Combobox => render_combobox(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::NumberInput => {
            render_number_input(ui, node, child_index, state, id_ctx, actions);
        }
        WidgetKind::Rating => render_rating(ui, node, child_index, state, id_ctx, actions),

        // ── Simple containers (children, no widget state) ───────
        WidgetKind::Row => render_row(ui, node, state, id_ctx, actions),
        WidgetKind::Column => render_column(ui, node, state, id_ctx, actions),
        WidgetKind::Padding => {
            let amount = node.prop_f32("amount").unwrap_or(ui.theme().padding);
            ui.padding(amount, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions)
            });
        }
        WidgetKind::Card => {
            if let Some(bg) = resolve::color_prop(node, "bg", ui.theme()) {
                ui.card_colored(bg, |ui| {
                    render_nodes(ui, &node.children, state, id_ctx, actions);
                });
            } else {
                ui.card(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
            }
        }
        WidgetKind::Surface => {
            ui.surface(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
        }
        WidgetKind::Section => {
            let title = node.text.as_deref().unwrap_or("");
            ui.section(title, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        WidgetKind::MaxWidth => {
            let w = node
                .prop_f32("width")
                .or_else(|| node.prop_f32("value"))
                .unwrap_or(600.0);
            ui.max_width(w, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        WidgetKind::CenterH => {
            let w = node.prop_f32("width").unwrap_or(400.0);
            ui.center_horizontal(w, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        WidgetKind::Clip => {
            ui.clip_children(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
        }
        WidgetKind::Labeled => {
            let label = node.text.as_deref().unwrap_or("");
            ui.labeled(label, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        WidgetKind::Blockquote => {
            let accent =
                resolve::color_prop(node, "accent", ui.theme()).unwrap_or(ui.theme().accent);
            ui.blockquote_colored(accent, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        WidgetKind::Spoiler => render_spoiler(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Style => {
            // Style node's props were already handled by build_style + with_style
            // in render_node. Just render children.
            render_nodes(ui, &node.children, state, id_ctx, actions);
        }
        WidgetKind::Disabled => {
            let val = node.prop_bool("value").unwrap_or(true);
            ui.disabled(val, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        WidgetKind::Field => render_field(ui, node, state, id_ctx, actions),
        WidgetKind::Container => render_container(ui, node, state, id_ctx, actions),

        // ── Layout builders ─────────────────────────────────────
        WidgetKind::Columns => render_columns(ui, node, state, id_ctx, actions),
        WidgetKind::Flex => render_flex(ui, node, state, id_ctx, actions),
        WidgetKind::Grid => render_grid(ui, node, state, id_ctx, actions),

        // ── Stateful containers ─────────────────────────────────
        WidgetKind::Page => render_page(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Scrollable => {
            render_scrollable(ui, node, child_index, state, id_ctx, actions);
        }
        WidgetKind::Tabs => render_tabs(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Modal => render_modal(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Drawer => render_drawer(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Collapsing => {
            render_collapsing(ui, node, child_index, state, id_ctx, actions);
        }
        WidgetKind::Accordion => render_accordion(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::SplitPane => render_split(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::Table => render_table(ui, node, child_index, state, id_ctx, actions),
        WidgetKind::VirtualScroll => {
            render_virtual_scroll(ui, node, child_index, state, id_ctx, actions);
        }
        WidgetKind::Pagination => {
            render_pagination(ui, node, child_index, state, id_ctx, actions);
        }

        // ── Navigation (leaf with action) ───────────────────────
        WidgetKind::Breadcrumb => render_breadcrumb(ui, node, child_index, id_ctx, actions),
        WidgetKind::Stepper => render_stepper(ui, node, child_index, id_ctx, actions),

        // ── Menus ───────────────────────────────────────────────
        WidgetKind::MenuBar => render_menu_bar(ui, node, child_index, id_ctx, actions),

        // ── Structural children (rendered by parent) ────────────
        WidgetKind::Span
        | WidgetKind::TableColumn
        | WidgetKind::TreeNode
        | WidgetKind::Menu
        | WidgetKind::MenuItem => {} // handled by parent widget

        // ── Overlays needing anchor ─────────────────────────────
        WidgetKind::Popover => {}  // TODO: needs anchor rect design
        WidgetKind::Tree => {}     // TODO: tree rendering with tree_node children
        WidgetKind::DropZone => {} // TODO: needs file path state

        // ── Escape hatch ────────────────────────────────────────
        WidgetKind::Image | WidgetKind::Custom(_) => {}
    }
}

// ── Action helper ───────────────────────────────────────────────────────

fn check_click(
    clicked: bool,
    node: &Node,
    bind: Option<&str>,
    child_index: usize,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    if clicked {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: id_ctx.state_key(bind, child_index),
                kind: ActionKind::Click,
            });
        }
    }
}

fn check_change(changed: bool, node: &Node, key: &str, actions: &mut Vec<Action>) {
    if changed {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: key.to_string(),
                kind: ActionKind::Change,
            });
        }
    }
}

// ── Leaf widgets ────────────────────────────────────────────────────────

fn render_label(ui: &mut Ui<'_>, node: &Node) {
    let text = node.text.as_deref().unwrap_or("");
    match node.variant.as_deref() {
        Some("muted") => ui.muted_label(text),
        Some("header") => ui.header_label(text),
        Some("wrapped") => ui.label_wrapped(text),
        Some("truncated") => match node.prop_str("truncation") {
            Some("start") => ui.label_truncated_mode(text, TruncationMode::Start),
            Some("middle") => ui.label_truncated_mode(text, TruncationMode::Middle),
            _ => ui.label_truncated(text),
        },
        Some("sized") => ui.label_sized(text, resolve::text_size(node)),
        Some("colored") => {
            let color = resolve::color_prop(node, "color", ui.theme()).unwrap_or(ui.theme().fg);
            ui.label_colored(text, color);
        }
        _ => {
            // Check inline shorthand props
            if node.props.contains_key("size") {
                ui.label_sized(text, resolve::text_size(node));
            } else if let Some(color) = resolve::color_prop(node, "color", ui.theme()) {
                ui.label_colored(text, color);
            } else {
                ui.label(text);
            }
        }
    }
}

fn render_rich_text(ui: &mut Ui<'_>, node: &Node) {
    let spans: Vec<_> = node
        .children
        .iter()
        .filter(|c| c.kind == WidgetKind::Span)
        .collect();

    if spans.is_empty() {
        return;
    }

    // Collect owned text so we can borrow it for RichText.
    let texts: Vec<String> = spans
        .iter()
        .map(|s| s.text.clone().unwrap_or_default())
        .collect();

    let theme = ui.theme().clone();
    let mut builder = RichText::new();
    for (i, span_node) in spans.iter().enumerate() {
        let text = texts[i].as_str();
        let color = resolve::color_prop(span_node, "color", &theme);
        let bold = span_node.prop_bool("bold").unwrap_or(false)
            || span_node.variant.as_deref() == Some("bold");
        let weight = span_node.prop_str("weight").and_then(|w| match w {
            "light" => Some(FontWeight::Light),
            "medium" => Some(FontWeight::Medium),
            "semibold" => Some(FontWeight::SemiBold),
            "bold" => Some(FontWeight::Bold),
            "extrabold" => Some(FontWeight::ExtraBold),
            _ => None,
        });

        if let (true, Some(c)) = (bold, color) {
            builder = builder.colored_bold(text, c);
        } else if bold {
            builder = builder.bold(text);
        } else if let Some(c) = color {
            builder = builder.colored(text, c);
        } else if let Some(w) = weight {
            builder = match w {
                FontWeight::Light => builder.light(text),
                FontWeight::Medium => builder.medium(text),
                FontWeight::SemiBold => builder.semibold(text),
                FontWeight::Bold => builder.bold(text),
                FontWeight::ExtraBold => builder.extrabold(text),
                FontWeight::Regular => builder.span(text),
            };
        } else {
            builder = builder.span(text);
        }
    }

    let wrapped = node.variant.as_deref() == Some("wrapped");
    if wrapped {
        ui.rich_label_wrapped(&builder);
    } else {
        ui.rich_label(&builder);
    }
}

fn render_progress(ui: &mut Ui<'_>, node: &Node, child_index: usize, id_ctx: &IdCtx) {
    let raw_value = node.prop_f32("value").unwrap_or(0.0);
    let value = if resolve::transition_props(node)
        .as_ref()
        .is_some_and(|t| resolve::should_animate(t, "value"))
    {
        let aid = crate::id::fnv1a_mix(
            id_ctx.widget_id(node.prop_str("bind"), child_index),
            crate::id::fnv1a_runtime("value"),
        );
        let duration = node.prop_f32("duration").unwrap_or(200.0);
        match resolve::spring_config(node) {
            Some(cfg) => ui.animate_spring(aid, raw_value, cfg),
            None => ui.animate(aid, raw_value, duration, resolve::easing(node)),
        }
    } else {
        raw_value
    };
    if let Some(color) = resolve::color_prop(node, "color", ui.theme()) {
        ui.progress_bar_colored(value, color);
    } else {
        ui.progress_bar(value);
    }
}

fn render_badge(ui: &mut Ui<'_>, node: &Node) {
    match node.variant.as_deref() {
        Some("dot") => ui.badge_dot(),
        Some("colored") => {
            let count = node.prop_f64("count").unwrap_or(0.0) as u32;
            let bg = resolve::color_prop(node, "bg", ui.theme()).unwrap_or(ui.theme().red);
            let fg = resolve::color_prop(node, "fg", ui.theme()).unwrap_or(ui.theme().fg);
            ui.badge_colored(count, bg, fg);
        }
        _ => {
            let count = node.prop_f64("count").unwrap_or(0.0) as u32;
            ui.badge(count);
        }
    }
}

fn render_avatar(ui: &mut Ui<'_>, node: &Node) {
    let initials = node.text.as_deref().unwrap_or("?");
    let size = node.prop_f32("size").unwrap_or(32.0);
    let bg = resolve::color_prop(node, "bg", ui.theme());
    let status = node.prop_str("status").and_then(|s| match s {
        "online" => Some(crate::widgets::avatar::Status::Online),
        "idle" => Some(crate::widgets::avatar::Status::Idle),
        "dnd" | "do-not-disturb" => Some(crate::widgets::avatar::Status::DoNotDisturb),
        "offline" => Some(crate::widgets::avatar::Status::Offline),
        _ => None,
    });

    match (bg, status) {
        (Some(bg), Some(st)) => ui.avatar_colored_with_status(initials, size, bg, st),
        (None, Some(st)) => ui.avatar_with_status(initials, size, st),
        (Some(bg), None) => ui.avatar_colored(initials, size, bg),
        (None, None) => ui.avatar(initials, size),
    }
}

fn render_skeleton(ui: &mut Ui<'_>, node: &Node) {
    match node.variant.as_deref() {
        Some("text") => ui.skeleton_text(),
        Some("circle") => {
            let d = node.prop_f32("diameter").unwrap_or(40.0);
            ui.skeleton_circle(d);
        }
        _ => {
            let w = node.prop_f32("width").unwrap_or(200.0);
            let h = node.prop_f32("height").unwrap_or(20.0);
            ui.skeleton(w, h);
        }
    }
}

fn render_pill(ui: &mut Ui<'_>, node: &Node) {
    let text = node.text.as_deref().unwrap_or("");
    match node.variant.as_deref() {
        Some("success") => ui.status_pill_success(text),
        Some("warning") => ui.status_pill_warning(text),
        Some("error") => ui.status_pill_error(text),
        _ => {
            let bg = resolve::color_prop(node, "bg", ui.theme()).unwrap_or(ui.theme().accent);
            let fg = resolve::color_prop(node, "fg", ui.theme()).unwrap_or(ui.theme().fg);
            ui.status_pill(text, bg, fg);
        }
    }
}

fn render_alert(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let msg = node.text.as_deref().unwrap_or("");
    match node.variant.as_deref() {
        Some("info") => ui.alert_info(msg),
        Some("success") => ui.alert_success(msg),
        Some("warning") => ui.alert_warning(msg),
        Some("error") => ui.alert_error(msg),
        Some("dismissable") => {
            let bind = node.prop_str("bind");
            let key = id_ctx.state_key(bind, child_index);
            let id = id_ctx.widget_id(bind, child_index);
            let visible = state.bools.entry(key.clone()).or_insert(true);
            let bg = resolve::color_prop(node, "bg", ui.theme()).unwrap_or(ui.theme().accent);
            let accent =
                resolve::color_prop(node, "accent", ui.theme()).unwrap_or(ui.theme().accent);
            let resp = ui.alert_dismissable(id, msg, visible, bg, accent);
            if resp.clicked {
                if let Some(action_name) = node.prop_str("action") {
                    actions.push(Action {
                        name: action_name.to_string(),
                        source: key,
                        kind: ActionKind::Dismiss,
                    });
                }
            }
        }
        _ => ui.alert_info(msg),
    }
}

// ── Buttons ─────────────────────────────────────────────────────────────

fn render_button(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let text = node.text.as_deref().unwrap_or("");

    let resp = match node.variant.as_deref() {
        Some("secondary") => ui.secondary_button(id, text),
        Some("danger") => ui.danger_button(id, text),
        Some("ghost") => ui.ghost_button(id, text),
        Some("outlined") => ui.outlined_button(id, text),
        Some("text") => ui.text_button(id, text),
        Some("small") => {
            let bg = resolve::color_prop(node, "bg", ui.theme()).unwrap_or(ui.theme().accent);
            ui.small_button(id, text, bg)
        }
        _ => {
            if let Some(max_w) = node.prop_f32("max-width") {
                ui.button_max_width(id, text, max_w)
            } else {
                ui.button(id, text)
            }
        }
    };

    // Tooltip
    if let Some(tooltip) = node.prop_str("tooltip") {
        ui.tooltip(id, tooltip);
    }

    check_click(resp.clicked, node, bind, child_index, id_ctx, actions);

    // Render children inside button context (rarely used, but possible)
    if !node.children.is_empty() {
        let _ = state; // suppress unused warning
    }
}

// ── Form inputs ─────────────────────────────────────────────────────────

fn render_input(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let placeholder = node.prop_str("placeholder").unwrap_or("");
    let input = state.inputs.entry(key.clone()).or_default();

    let resp = match node.variant.as_deref() {
        Some("validated") => {
            let status = resolve::field_status(node);
            ui.text_input_validated(id, input, placeholder, status)
        }
        _ => ui.text_input(id, input, placeholder),
    };
    check_change(resp.changed, node, &key, actions);
}

fn render_textarea(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let placeholder = node.prop_str("placeholder").unwrap_or("");
    let rows = node.prop_f64("rows").unwrap_or(4.0) as usize;
    let input = state.inputs.entry(key.clone()).or_default();

    let resp = match node.variant.as_deref() {
        Some("wrapped") => ui.text_area_wrapped(id, input, rows, placeholder),
        _ => ui.text_area(id, input, rows, placeholder),
    };
    check_change(resp.changed, node, &key, actions);
}

fn render_checkbox(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let label = node.text.as_deref().unwrap_or("");
    let input = state.inputs.entry(key.clone()).or_default();
    let resp = ui.checkbox(id, input, label);
    check_change(resp.changed, node, &key, actions);
}

fn render_toggle(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let label = node.text.as_deref().unwrap_or("");
    let input = state.inputs.entry(key.clone()).or_default();
    let resp = ui.toggle(id, input, label);
    check_change(resp.changed, node, &key, actions);
}

fn render_radio(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let label = node.text.as_deref().unwrap_or("");
    let option_index = node.prop_f64("value").unwrap_or(0.0) as usize;
    let input = state.inputs.entry(key.clone()).or_default();
    let resp = ui.radio(id, input, option_index, label);
    check_change(resp.changed, node, &key, actions);
}

fn render_slider(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let min = node.prop_f32("min").unwrap_or(0.0);
    let max = node.prop_f32("max").unwrap_or(100.0);
    let input = state.inputs.entry(key.clone()).or_default();
    let resp = ui.slider(id, input, min, max);
    check_change(resp.changed, node, &key, actions);
}

fn render_select(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let options_owned: Vec<String> = node
        .prop_string_array("options")
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    let options: Vec<&str> = options_owned.iter().map(|s| s.as_str()).collect();
    let select = state.selects.entry(key.clone()).or_default();
    let resp = ui.select(id, select, &options);
    check_change(resp.changed, node, &key, actions);
}

fn render_combobox(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let options_owned: Vec<String> = node
        .prop_string_array("options")
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    let options: Vec<&str> = options_owned.iter().map(|s| s.as_str()).collect();
    let selected = state.comboboxes.entry(key.clone()).or_insert(None);
    let resp = ui.combobox(id, &options, selected);
    check_change(resp.changed, node, &key, actions);
}

fn render_number_input(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let step = node.prop_f64("step").unwrap_or(1.0);
    let value = state.floats.entry(key.clone()).or_insert(0.0);

    let resp = if node.props.contains_key("min") || node.props.contains_key("max") {
        let min = node.prop_f64("min").unwrap_or(f64::MIN);
        let max = node.prop_f64("max").unwrap_or(f64::MAX);
        ui.number_input_clamped(id, value, step, min, max)
    } else {
        ui.number_input(id, value, step)
    };
    check_change(resp.changed, node, &key, actions);
}

fn render_rating(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let max = node.prop_f64("max").unwrap_or(5.0) as u8;

    if node.variant.as_deref() == Some("display") {
        let value = node.prop_f32("value").unwrap_or(0.0);
        ui.rating_display(value, max);
        return;
    }

    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let value = state.u8s.entry(key.clone()).or_insert(0);
    let resp = ui.rating(id, value, max);
    check_change(resp.changed, node, &key, actions);
}

// ── Simple containers ───────────────────────────────────────────────────

fn render_row(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    if node.prop_str("justify").is_some() || node.prop_str("align").is_some() {
        // Use flex_row for advanced alignment
        let mut builder = ui.flex_row();
        if let Some(gap) = node.prop_f32("gap") {
            builder = builder.gap(gap);
        }
        if let Some(a) = resolve::align(node, "align") {
            builder = builder.align(a);
        }
        if let Some(j) = resolve::justify(node, "justify") {
            builder = builder.justify(j);
        }
        builder.show(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
    } else if let Some(gap) = node.prop_f32("gap") {
        ui.row_spaced(gap, |ui| {
            render_nodes(ui, &node.children, state, id_ctx, actions);
        });
    } else {
        ui.row(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
    }
}

fn render_column(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    if let Some(gap) = node.prop_f32("gap") {
        ui.with_spacing(gap, |ui| {
            render_nodes(ui, &node.children, state, id_ctx, actions);
        });
    } else {
        render_nodes(ui, &node.children, state, id_ctx, actions);
    }
}

fn render_field(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let label = node.text.as_deref().unwrap_or("");
    let status = resolve::field_status(node);
    let hint = node.prop_str("hint").unwrap_or("");
    ui.form_field(label, status, hint, |ui| {
        render_nodes(ui, &node.children, state, id_ctx, actions);
        crate::Response::default()
    });
}

fn render_container(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    // Resolve props before borrowing ui for the builder.
    let theme = ui.theme().clone();
    let bg = resolve::color_prop(node, "bg", &theme);
    let bc = resolve::color_prop(node, "border-color", &theme);
    let bw = node.prop_f32("border-width").unwrap_or(1.0);
    let radius = node.prop_f32("radius");
    let pad = node.prop_f32("padding");
    let elev = resolve::elevation(node, &theme);

    let mut builder = ui.box_container();
    if let Some(bg) = bg {
        builder = builder.bg(bg);
    }
    if let Some(bc) = bc {
        builder = builder.border(bc, bw);
    }
    if let Some(r) = radius {
        builder = builder.radius(r);
    }
    if let Some(p) = pad {
        builder = builder.padding(p);
    }
    if let Some(e) = elev {
        builder = builder.elevation(e);
    }
    builder.show(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
}

// ── Layout builders ─────────────────────────────────────────────────────

fn render_columns(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let weights: Vec<f32> = node
        .prop_number_array("weights")
        .unwrap_or_default()
        .into_iter()
        .map(|n| n as f32)
        .collect();
    if weights.is_empty() {
        render_nodes(ui, &node.children, state, id_ctx, actions);
        return;
    }
    let gap = node.prop_f32("gap").unwrap_or(0.0);
    let children = &node.children;
    ui.columns_spaced(gap, &weights, |ui, col| {
        if let Some(child) = children.get(col) {
            render_node(ui, child, col, state, id_ctx, actions);
        }
    });
}

fn render_flex(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let is_col = node.variant.as_deref() == Some("col");
    let mut builder = if is_col { ui.flex_col() } else { ui.flex_row() };
    if let Some(gap) = node.prop_f32("gap") {
        builder = builder.gap(gap);
    }
    if let Some(a) = resolve::align(node, "align") {
        builder = builder.align(a);
    }
    if let Some(j) = resolve::justify(node, "justify") {
        builder = builder.justify(j);
    }
    if let Some(w) = resolve::flex_wrap(node) {
        builder = builder.wrap(w);
    }
    builder.show(|ui| render_nodes(ui, &node.children, state, id_ctx, actions));
}

fn render_grid(
    ui: &mut Ui<'_>,
    node: &Node,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let col_tracks = node
        .props
        .get("cols")
        .map(resolve::grid_tracks)
        .unwrap_or_default();
    let row_tracks = node
        .props
        .get("rows")
        .map(resolve::grid_tracks)
        .unwrap_or_default();

    let mut builder = ui.grid(&col_tracks, &row_tracks);
    if let Some(g) = node.prop_f32("gap") {
        builder = builder.gap(g);
    }
    if let Some(g) = node.prop_f32("col-gap") {
        builder = builder.col_gap(g);
    }
    if let Some(g) = node.prop_f32("row-gap") {
        builder = builder.row_gap(g);
    }

    builder.show(|grid| {
        for (i, child) in node.children.iter().enumerate() {
            let placement = crate::layout::GridPlacement {
                column: child.prop_f64("col").unwrap_or(i as f64) as u16,
                row: child.prop_f64("row").unwrap_or(0.0) as u16,
                col_span: child.prop_f64("col-span").unwrap_or(1.0) as u16,
                row_span: child.prop_f64("row-span").unwrap_or(1.0) as u16,
            };
            grid.cell(placement, |ui| {
                render_node(ui, child, i, state, id_ctx, actions);
            });
        }
    });
}

// ── Stateful containers ─────────────────────────────────────────────────

fn render_page(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let max_w = node
        .prop_f32("max-width")
        .unwrap_or_else(|| ui.region_width());
    // scroll_h = viewport height so the page is scrollable within the window
    let scroll_h = node.prop_f32("height").unwrap_or(ui.region.h);

    id_ctx.push(id);
    // Ui::page() already applies theme.padding internally, so we only
    // override if the markup specifies a different padding.
    let custom_pad = node.prop_f32("padding");
    if let Some(pad) = custom_pad {
        // Use scrollable + max_width + custom padding directly instead of
        // page(), which hardcodes theme.padding.
        ui.scrollable(id, scroll_h, |ui| {
            ui.max_width(max_w, |ui| {
                ui.padding(pad, |ui| {
                    render_nodes(ui, &node.children, state, id_ctx, actions);
                });
            });
        });
    } else {
        ui.page(id, scroll_h, max_w, |ui| {
            render_nodes(ui, &node.children, state, id_ctx, actions);
        });
    }
    id_ctx.pop();
}

fn render_scrollable(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let height = node.prop_f32("height").unwrap_or(400.0);

    id_ctx.push(id);
    match node.variant.as_deref() {
        Some("horizontal") => {
            let width = node.prop_f32("width").unwrap_or(400.0);
            ui.scrollable_horizontal(id, width, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        Some("2d") => {
            let width = node.prop_f32("width").unwrap_or(400.0);
            ui.scrollable_2d(id, width, height, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
        _ => {
            ui.scrollable(id, height, |ui| {
                render_nodes(ui, &node.children, state, id_ctx, actions);
            });
        }
    }
    id_ctx.pop();
}

fn render_tabs(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);

    let labels_owned: Vec<String> = node
        .prop_string_array("labels")
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();

    // Remove state to avoid double borrow
    let mut tab_state = state.tabs.remove(&key).unwrap_or_default();

    id_ctx.push(id);
    ui.tabs(id, &mut tab_state, &labels, |ui, selected| {
        // Render the child corresponding to the selected tab
        if let Some(child) = node.children.get(selected) {
            render_node(ui, child, selected, state, id_ctx, actions);
        } else if node.children.len() == 1 {
            render_nodes(ui, &node.children, state, id_ctx, actions);
        }
    });
    id_ctx.pop();

    // Re-insert state
    let selected = tab_state.selected;
    state.tabs.insert(key.clone(), tab_state);

    if let Some(action_name) = node.prop_str("action") {
        // We can't detect "changed" directly without comparing to previous,
        // so fire action always with the current selection.
        // The host can track previous state if needed.
        let _ = (action_name, selected);
    }
}

fn render_modal(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let title = node.text.as_deref().unwrap_or("");
    let width = node.prop_f32("width").unwrap_or(400.0);

    let mut open = state.bools.remove(&key).unwrap_or(false);
    let was_open = open;

    id_ctx.push(id);
    ui.modal(id, &mut open, title, width, |ui| {
        render_nodes(ui, &node.children, state, id_ctx, actions);
    });
    id_ctx.pop();

    state.bools.insert(key.clone(), open);

    if was_open && !open {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: key,
                kind: ActionKind::Dismiss,
            });
        }
    }
}

fn render_drawer(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let width = node.prop_f32("width").unwrap_or(300.0);

    let mut open = state.bools.remove(&key).unwrap_or(false);
    let was_open = open;

    id_ctx.push(id);
    if node.variant.as_deref() == Some("right") {
        ui.drawer_right(id, &mut open, width, |ui| {
            render_nodes(ui, &node.children, state, id_ctx, actions);
        });
    } else {
        ui.drawer(id, &mut open, width, |ui| {
            render_nodes(ui, &node.children, state, id_ctx, actions);
        });
    }
    id_ctx.pop();

    state.bools.insert(key.clone(), open);
    if was_open && !open {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: key,
                kind: ActionKind::Dismiss,
            });
        }
    }
}

fn render_collapsing(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let label = node.text.as_deref().unwrap_or("");
    let default_open = node.prop_bool("open").unwrap_or(false);

    id_ctx.push(id);
    ui.collapsing_header(id, label, default_open, |ui| {
        render_nodes(ui, &node.children, state, id_ctx, actions);
    });
    id_ctx.pop();
}

fn render_accordion(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);

    let sections_owned: Vec<String> = node
        .prop_string_array("sections")
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    let sections: Vec<&str> = sections_owned.iter().map(|s| s.as_str()).collect();

    let mut open_idx = state.accordion_open.remove(&key).unwrap_or(None);

    id_ctx.push(id);
    ui.accordion(id, &sections, &mut open_idx, |ui, section_idx| {
        if let Some(child) = node.children.get(section_idx) {
            render_node(ui, child, section_idx, state, id_ctx, actions);
        }
    });
    id_ctx.pop();

    state.accordion_open.insert(key, open_idx);
}

fn render_split(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let ratio = node.prop_f32("ratio").unwrap_or(0.5);
    let is_vertical = node.variant.as_deref() == Some("v");

    // Split pane takes two FnOnce closures. We use a shared &mut context
    // that gets passed to whichever closure runs. Since split_pane calls
    // the left closure first, then the right, we pack our mutable context
    // into an Option that each closure takes from.
    struct Ctx<'a, 'b> {
        state: &'a mut MarkupState,
        id_ctx: &'a mut IdCtx,
        actions: &'a mut Vec<Action>,
        children: &'b [Node],
    }

    // We wrap in a Cell-like pattern: each closure checks if context is available.
    let ctx = std::cell::RefCell::new(Ctx {
        state,
        id_ctx,
        actions,
        children: &node.children,
    });

    let render_pane = |ui: &mut Ui<'_>, idx: usize| {
        let c = &mut *ctx.borrow_mut();
        if let Some(child) = c.children.get(idx) {
            render_node(ui, child, idx, c.state, c.id_ctx, c.actions);
        }
    };

    if is_vertical {
        ui.split_pane_v(id, ratio, |ui| render_pane(ui, 0), |ui| render_pane(ui, 1));
    } else {
        ui.split_pane_h(id, ratio, |ui| render_pane(ui, 0), |ui| render_pane(ui, 1));
    }
}

fn render_table(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    _actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);

    // Extract column definitions from TableColumn children
    let col_defs: Vec<_> = node
        .children
        .iter()
        .filter(|c| c.kind == WidgetKind::TableColumn)
        .collect();

    let headers: Vec<String> = col_defs
        .iter()
        .map(|c| c.text.clone().unwrap_or_default())
        .collect();

    let columns: Vec<TableColumn<'_>> = col_defs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let width = match c.prop_str("width") {
                Some("auto") => ColumnWidth::Auto,
                Some(s) if s.ends_with("fr") => {
                    let n: f32 = s.trim_end_matches("fr").parse().unwrap_or(1.0);
                    ColumnWidth::Weight(n)
                }
                _ => c
                    .prop_f32("width")
                    .map(ColumnWidth::Fixed)
                    .unwrap_or(ColumnWidth::Weight(1.0)),
            };
            let sortable = c.prop_bool("sortable").unwrap_or(true);
            TableColumn {
                header: headers[i].as_str(),
                width,
                sortable,
            }
        })
        .collect();

    let row_count = node.prop_f64("rows").unwrap_or(0.0) as usize;
    let visible = node.prop_f64("visible").unwrap_or(10.0) as usize;

    let mut table_state = state.tables.remove(&key).unwrap_or_default();

    ui.table(
        id,
        &mut table_state,
        &columns,
        row_count,
        visible,
        |ui, _row, _col| {
            // Static markup can't provide dynamic cell content.
            // The host app should use get_selected_row() to know selection,
            // or provide cell content via a callback mechanism.
            ui.label("");
        },
    );

    state.tables.insert(key, table_state);
}

fn render_virtual_scroll(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    _actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let item_height = node.prop_f32("item-height").unwrap_or(32.0);
    let height = node.prop_f32("height").unwrap_or(400.0);
    let count = node.prop_f64("count").unwrap_or(0.0) as usize;

    let mut vs = state
        .vscrolls
        .remove(&key)
        .unwrap_or(crate::state::VirtualScrollState {
            item_count: 0,
            scroll_to: None,
        });
    vs.item_count = count;

    id_ctx.push(id);
    ui.virtual_scroll(id, &mut vs, item_height, height, |ui, _item_idx| {
        // Static markup can't provide per-item content.
        ui.label("");
    });
    id_ctx.pop();

    state.vscrolls.insert(key, vs);
}

fn render_pagination(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let key = id_ctx.state_key(bind, child_index);
    let id = id_ctx.widget_id(bind, child_index);
    let total = node.prop_f64("total-pages").unwrap_or(1.0) as usize;

    let mut ps = state.paginations.remove(&key).unwrap_or_default();
    let resp = ui.pagination(id, &mut ps, total);
    let page = ps.current_page;
    state.paginations.insert(key.clone(), ps);

    if resp.changed {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: key,
                kind: ActionKind::Select(page),
            });
        }
    }
}

// ── Navigation ──────────────────────────────────────────────────────────

fn render_breadcrumb(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let segments_owned: Vec<String> = node
        .prop_string_array("segments")
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    let segments: Vec<&str> = segments_owned.iter().map(|s| s.as_str()).collect();

    if let Some(clicked_idx) = ui.breadcrumb(id, &segments) {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: id_ctx.state_key(bind, child_index),
                kind: ActionKind::Select(clicked_idx),
            });
        }
    }
}

fn render_stepper(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);
    let labels_owned: Vec<String> = node
        .prop_string_array("labels")
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();
    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
    let current = node.prop_f64("current").unwrap_or(0.0) as usize;

    if let Some(clicked_idx) = ui.stepper(id, &labels, current) {
        if let Some(action_name) = node.prop_str("action") {
            actions.push(Action {
                name: action_name.to_string(),
                source: id_ctx.state_key(bind, child_index),
                kind: ActionKind::Select(clicked_idx),
            });
        }
    }
}

// ── Menus ───────────────────────────────────────────────────────────────

fn render_menu_bar(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    id_ctx: &IdCtx,
    actions: &mut Vec<Action>,
) {
    use crate::widgets::menu_bar::{Menu, MenuEntry, MenuItem};

    let menus: Vec<Menu> = node
        .children
        .iter()
        .filter(|c| c.kind == WidgetKind::Menu)
        .map(|menu_node| {
            let items: Vec<MenuEntry> = menu_node
                .children
                .iter()
                .filter(|c| c.kind == WidgetKind::MenuItem)
                .map(|item_node| {
                    let label = item_node.text.clone().unwrap_or_default();
                    let action = item_node.prop_str("action").unwrap_or(label.as_str());
                    let id = crate::id::fnv1a_runtime(action);
                    MenuEntry::Item(MenuItem::new(label, id))
                })
                .collect();
            Menu {
                label: menu_node.text.clone().unwrap_or_default(),
                items,
            }
        })
        .collect();

    if let Some(selected_id) = ui.menu_bar(&menus) {
        // Find the action name for the selected menu item
        for menu_node in node.children.iter().filter(|c| c.kind == WidgetKind::Menu) {
            for item_node in menu_node
                .children
                .iter()
                .filter(|c| c.kind == WidgetKind::MenuItem)
            {
                if let Some(action_name) = item_node.prop_str("action") {
                    let item_id = crate::id::fnv1a_runtime(action_name);
                    if item_id == selected_id {
                        actions.push(Action {
                            name: action_name.to_string(),
                            source: id_ctx.state_key(None, child_index),
                            kind: ActionKind::Click,
                        });
                    }
                }
            }
        }
    }
}

// ── Veil P0 widgets ──────────────────────────────────────────────────────

fn render_code_block(ui: &mut Ui<'_>, node: &Node, child_index: usize, id_ctx: &IdCtx) {
    let code = node.text.as_deref().unwrap_or("");
    let language = node.prop_str("language");
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);

    if let Some(lang) = language {
        ui.code_block_lang(id, lang, code);
    } else {
        ui.code_block(id, code);
    }
}

fn render_spoiler(
    ui: &mut Ui<'_>,
    node: &Node,
    child_index: usize,
    state: &mut MarkupState,
    id_ctx: &mut IdCtx,
    actions: &mut Vec<Action>,
) {
    let bind = node.prop_str("bind");
    let id = id_ctx.widget_id(bind, child_index);

    id_ctx.push(id);
    ui.spoiler(id, |ui| {
        render_nodes(ui, &node.children, state, id_ctx, actions);
    });
    id_ctx.pop();
}
