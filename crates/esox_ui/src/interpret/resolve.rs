//! Property resolution — converts AST values to esox_ui types.

use esox_gfx::Color;
use esox_markup::{Node, Value};

use crate::layout::{Align, FlexWrap, GridTrack, Justify, Spacing};
use crate::theme::{
    Elevation, Gradient, TextAlign, TextDecoration, TextSize, TextTransform, Theme, WidgetStyle,
};
use crate::widgets::form::FieldStatus;

// ── Colors ──────────────────────────────────────────────────────────────

/// Convert a `Value::Color(0xRRGGBB)` to a `Color`.
pub(crate) fn color_from_u32(c: u32) -> Color {
    if c > 0xFF_FF_FF {
        // 8-digit hex: RRGGBBAA
        let r = ((c >> 24) & 0xFF) as f32 / 255.0;
        let g = ((c >> 16) & 0xFF) as f32 / 255.0;
        let b = ((c >> 8) & 0xFF) as f32 / 255.0;
        let a = (c & 0xFF) as f32 / 255.0;
        Color::new(r, g, b, a)
    } else {
        // 6-digit hex: RRGGBB
        let r = ((c >> 16) & 0xFF) as f32 / 255.0;
        let g = ((c >> 8) & 0xFF) as f32 / 255.0;
        let b = (c & 0xFF) as f32 / 255.0;
        Color::new(r, g, b, 1.0)
    }
}

/// Resolve a color property — supports hex values and named theme colors.
pub(crate) fn color_prop(node: &Node, key: &str, theme: &Theme) -> Option<Color> {
    match node.props.get(key)? {
        Value::Color(c) => Some(color_from_u32(*c)),
        Value::Ident(name) | Value::String(name) => named_color(name, theme),
        _ => None,
    }
}

fn named_color(name: &str, theme: &Theme) -> Option<Color> {
    Some(match name {
        "accent" => theme.accent,
        "red" | "error" => theme.red,
        "green" | "success" => theme.green,
        "amber" | "warning" => theme.amber,
        "muted" => theme.fg_muted,
        "dim" => theme.fg_dim,
        "fg" => theme.fg,
        "bg" => theme.bg_base,
        "surface" => theme.bg_surface,
        "raised" => theme.bg_raised,
        "border" => theme.border,
        _ => return None,
    })
}

// ── Text ────────────────────────────────────────────────────────────────

/// Resolve `size=xl` or `size=18` to a `TextSize`.
pub(crate) fn text_size(node: &Node) -> TextSize {
    match node.prop_str("size") {
        Some("xs") => TextSize::Xs,
        Some("sm") => TextSize::Sm,
        Some("base") => TextSize::Base,
        Some("lg") => TextSize::Lg,
        Some("xl") => TextSize::Xl,
        Some("xxl" | "2xl") => TextSize::Xxl,
        _ => node
            .prop_f32("size")
            .map(TextSize::Custom)
            .unwrap_or(TextSize::Base),
    }
}

/// Resolve `text-align=center`.
pub(crate) fn text_align(node: &Node) -> Option<TextAlign> {
    Some(match node.prop_str("text-align")? {
        "left" => TextAlign::Left,
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        _ => return None,
    })
}

/// Resolve `text-decoration=underline`.
pub(crate) fn text_decoration(node: &Node) -> Option<TextDecoration> {
    Some(match node.prop_str("text-decoration")? {
        "none" => TextDecoration::None,
        "underline" => TextDecoration::Underline,
        "strikethrough" | "line-through" => TextDecoration::Strikethrough,
        "both" => TextDecoration::Both,
        _ => return None,
    })
}

/// Resolve `text-transform=uppercase`.
pub(crate) fn text_transform(node: &Node) -> Option<TextTransform> {
    Some(match node.prop_str("text-transform")? {
        "none" => TextTransform::None,
        "uppercase" | "upper" => TextTransform::Uppercase,
        "lowercase" | "lower" => TextTransform::Lowercase,
        "capitalize" | "title" => TextTransform::Capitalize,
        _ => return None,
    })
}

// ── Layout ──────────────────────────────────────────────────────────────

/// Resolve `align=center`.
pub(crate) fn align(node: &Node, key: &str) -> Option<Align> {
    Some(match node.prop_str(key)? {
        "start" => Align::Start,
        "center" => Align::Center,
        "end" => Align::End,
        "stretch" => Align::Stretch,
        _ => return None,
    })
}

/// Resolve `justify=between`.
pub(crate) fn justify(node: &Node, key: &str) -> Option<Justify> {
    Some(match node.prop_str(key)? {
        "start" => Justify::Start,
        "center" => Justify::Center,
        "end" => Justify::End,
        "between" | "space-between" => Justify::SpaceBetween,
        _ => return None,
    })
}

/// Resolve `wrap=wrap`.
pub(crate) fn flex_wrap(node: &Node) -> Option<FlexWrap> {
    Some(match node.prop_str("wrap")? {
        "nowrap" | "none" | "false" => FlexWrap::NoWrap,
        "wrap" | "true" => FlexWrap::Wrap,
        "wrap-reverse" | "reverse" => FlexWrap::WrapReverse,
        _ => return None,
    })
}

/// Resolve `status=error` to `FieldStatus`.
pub(crate) fn field_status(node: &Node) -> FieldStatus {
    match node.prop_str("status") {
        Some("error") => FieldStatus::Error,
        Some("ok" | "success") => FieldStatus::Success,
        Some("warning") => FieldStatus::Warning,
        _ => FieldStatus::None,
    }
}

// ── Spacing ─────────────────────────────────────────────────────────────

/// Resolve padding/margin from a single number or a 2/4-element array.
pub(crate) fn spacing_prop(node: &Node, key: &str) -> Option<Spacing> {
    if let Some(n) = node.prop_f32(key) {
        return Some(Spacing::all(n));
    }
    if let Some(arr) = node.prop_number_array(key) {
        return match arr.len() {
            1 => Some(Spacing::all(arr[0] as f32)),
            // [vertical, horizontal] — matches CSS shorthand order
            2 => Some(Spacing::symmetric(arr[1] as f32, arr[0] as f32)),
            4 => Some(Spacing {
                top: arr[0] as f32,
                right: arr[1] as f32,
                bottom: arr[2] as f32,
                left: arr[3] as f32,
            }),
            _ => None,
        };
    }
    None
}

// ── Grid tracks ─────────────────────────────────────────────────────────

/// Parse grid track definitions from a string like `"1fr 200 auto"`.
pub(crate) fn grid_tracks(value: &Value) -> Vec<GridTrack> {
    match value {
        Value::String(s) | Value::Ident(s) => parse_track_string(s),
        Value::Array(arr) => arr.iter().filter_map(track_from_value).collect(),
        Value::Number(n) => vec![GridTrack::Fixed(*n as f32)],
        _ => vec![],
    }
}

fn parse_track_string(s: &str) -> Vec<GridTrack> {
    s.split_whitespace()
        .filter_map(parse_single_track)
        .collect()
}

fn parse_single_track(s: &str) -> Option<GridTrack> {
    if s.eq_ignore_ascii_case("auto") {
        return Some(GridTrack::Auto);
    }
    if let Some(fr) = s.strip_suffix("fr") {
        return fr.parse::<f32>().ok().map(GridTrack::Fr);
    }
    if let Some(rest) = s.strip_prefix("minmax(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = rest.splitn(2, ',').collect();
        if parts.len() == 2 {
            let min = parts[0].trim().parse::<f32>().ok()?;
            let max_s = parts[1].trim();
            let max = if let Some(fr) = max_s.strip_suffix("fr") {
                fr.parse::<f32>().ok()?
            } else {
                max_s.parse::<f32>().ok()?
            };
            return Some(GridTrack::MinMax(min, max));
        }
    }
    s.parse::<f32>().ok().map(GridTrack::Fixed)
}

fn track_from_value(v: &Value) -> Option<GridTrack> {
    match v {
        Value::Number(n) => Some(GridTrack::Fixed(*n as f32)),
        Value::Ident(s) | Value::String(s) => parse_single_track(s),
        _ => None,
    }
}

// ── Elevation ───────────────────────────────────────────────────────────

/// Resolve elevation from a named preset or custom shadow properties.
pub(crate) fn elevation(node: &Node, theme: &Theme) -> Option<Elevation> {
    // Named presets
    if let Some(name) = node.prop_str("elevation") {
        return Some(match name {
            "low" => theme.elevation_low,
            "medium" | "med" => theme.elevation_medium,
            "high" => theme.elevation_high,
            "none" => Elevation {
                blur: 0.0,
                dx: 0.0,
                dy: 0.0,
                color: Color::TRANSPARENT,
            },
            _ => return None,
        });
    }
    // Custom shadow properties
    let blur = node.prop_f32("shadow-blur")?;
    let dx = node.prop_f32("shadow-dx").unwrap_or(0.0);
    let dy = node.prop_f32("shadow-dy").unwrap_or(0.0);
    let color = color_prop(node, "shadow-color", theme).unwrap_or(theme.shadow);
    Some(Elevation {
        blur,
        dx,
        dy,
        color,
    })
}

// ── Gradient ────────────────────────────────────────────────────────────

/// Resolve gradient from flat properties.
pub(crate) fn gradient(node: &Node, theme: &Theme) -> Option<Gradient> {
    let kind = node.prop_str("gradient")?;
    let end_color = color_prop(node, "gradient-to", theme)?;
    Some(match kind {
        "linear" => Gradient::Linear {
            end_color,
            angle: node.prop_f32("gradient-angle").unwrap_or(0.0),
        },
        "radial" => Gradient::Radial { end_color },
        "conic" => Gradient::Conic {
            end_color,
            start_angle: node.prop_f32("gradient-angle").unwrap_or(0.0),
        },
        _ => return None,
    })
}

// ── WidgetStyle builder ─────────────────────────────────────────────────

/// Build a `WidgetStyle` from a node's inline style properties.
/// Returns `None` if no style properties are present (the common case).
pub(crate) fn build_style(node: &Node, theme: &Theme) -> Option<WidgetStyle> {
    // Quick check: skip building if no style-related keys are present.
    const STYLE_KEYS: &[&str] = &[
        "bg",
        "fg",
        "border-color",
        "text-color",
        "font-size",
        "radius",
        "height",
        "width",
        "opacity",
        "border-width",
        "min-width",
        "max-width",
        "min-height",
        "max-height",
        "text-align",
        "text-decoration",
        "text-transform",
        "elevation",
        "shadow-blur",
        "gradient",
        "spacing",
        "padding",
        "margin",
    ];

    if !STYLE_KEYS.iter().any(|k| node.props.contains_key(*k)) {
        return None;
    }

    // Corner radius: uniform or per-corner array
    let (corner_radius, per_corner_radius) = if let Some(arr) = node.prop_number_array("radius") {
        if arr.len() == 4 {
            (
                None,
                Some([arr[0] as f32, arr[1] as f32, arr[2] as f32, arr[3] as f32]),
            )
        } else if arr.len() == 1 {
            (Some(arr[0] as f32), None)
        } else {
            (None, None)
        }
    } else {
        (node.prop_f32("radius"), None)
    };

    Some(WidgetStyle {
        bg: color_prop(node, "bg", theme),
        fg: color_prop(node, "fg", theme),
        border_color: color_prop(node, "border-color", theme),
        text_color: color_prop(node, "text-color", theme),
        font_size: node.prop_f32("font-size"),
        border_width: node.prop_f32("border-width"),
        opacity: node.prop_f32("opacity"),
        height: node.prop_f32("height"),
        width: node.prop_f32("width"),
        min_width: node.prop_f32("min-width"),
        max_width: node.prop_f32("max-width"),
        min_height: node.prop_f32("min-height"),
        max_height: node.prop_f32("max-height"),
        spacing: node.prop_f32("spacing"),
        text_align: text_align(node),
        text_decoration: text_decoration(node),
        text_transform: text_transform(node),
        elevation: elevation(node, theme),
        gradient: gradient(node, theme),
        padding: spacing_prop(node, "padding"),
        margin: spacing_prop(node, "margin"),
        corner_radius,
        per_corner_radius,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use esox_markup::parse;

    fn node(markup: &str) -> Node {
        parse(markup).unwrap().into_iter().next().unwrap()
    }

    #[test]
    fn test_color_from_u32_6digit() {
        let c = color_from_u32(0xFF0000);
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g.abs() < 0.01);
        assert!(c.b.abs() < 0.01);
        assert!((c.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_from_u32_8digit() {
        let c = color_from_u32(0xFF000080);
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.a - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_text_size_named() {
        let n = node(r#"label "x" size=xl"#);
        assert_eq!(text_size(&n), TextSize::Xl);
    }

    #[test]
    fn test_text_size_numeric() {
        let n = node(r#"label "x" size=18"#);
        assert_eq!(text_size(&n), TextSize::Custom(18.0));
    }

    #[test]
    fn test_text_size_default() {
        let n = node(r#"label "x""#);
        assert_eq!(text_size(&n), TextSize::Base);
    }

    #[test]
    fn test_text_align_resolve() {
        let n = node(r#"label "x" text-align=center"#);
        assert_eq!(text_align(&n), Some(TextAlign::Center));
    }

    #[test]
    fn test_align_resolve() {
        let n = node(r#"flex align=stretch"#);
        assert_eq!(align(&n, "align"), Some(Align::Stretch));
    }

    #[test]
    fn test_justify_resolve() {
        let n = node(r#"row justify=between"#);
        assert_eq!(justify(&n, "justify"), Some(Justify::SpaceBetween));
    }

    #[test]
    fn test_flex_wrap_resolve() {
        let n = node(r#"flex wrap=wrap"#);
        assert_eq!(flex_wrap(&n), Some(FlexWrap::Wrap));
    }

    #[test]
    fn test_field_status_resolve() {
        let n = node(r#"field "Name" status=error"#);
        assert_eq!(field_status(&n), FieldStatus::Error);
    }

    #[test]
    fn test_field_status_none() {
        let n = node(r#"field "Name""#);
        assert_eq!(field_status(&n), FieldStatus::None);
    }

    #[test]
    fn test_spacing_uniform() {
        let n = node(r#"style padding=16"#);
        let s = spacing_prop(&n, "padding").unwrap();
        assert_eq!(s.top, 16.0);
        assert_eq!(s.right, 16.0);
    }

    #[test]
    fn test_spacing_symmetric() {
        let n = node(r#"style padding=[8, 16]"#);
        let s = spacing_prop(&n, "padding").unwrap();
        assert_eq!(s.top, 8.0);
        assert_eq!(s.right, 16.0);
        assert_eq!(s.bottom, 8.0);
        assert_eq!(s.left, 16.0);
    }

    #[test]
    fn test_spacing_four_sides() {
        let n = node(r#"style padding=[4, 8, 12, 16]"#);
        let s = spacing_prop(&n, "padding").unwrap();
        assert_eq!(s.top, 4.0);
        assert_eq!(s.right, 8.0);
        assert_eq!(s.bottom, 12.0);
        assert_eq!(s.left, 16.0);
    }

    #[test]
    fn test_grid_tracks_string() {
        let v = Value::String("1fr 200 auto".to_string());
        let tracks = grid_tracks(&v);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0], GridTrack::Fr(1.0));
        assert_eq!(tracks[1], GridTrack::Fixed(200.0));
        assert_eq!(tracks[2], GridTrack::Auto);
    }

    #[test]
    fn test_grid_tracks_minmax() {
        let v = Value::String("minmax(100,2fr)".to_string());
        let tracks = grid_tracks(&v);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0], GridTrack::MinMax(100.0, 2.0));
    }

    #[test]
    fn test_grid_tracks_array() {
        let v = Value::Array(vec![
            Value::Number(200.0),
            Value::Ident("1fr".to_string()),
            Value::Ident("auto".to_string()),
        ]);
        let tracks = grid_tracks(&v);
        assert_eq!(tracks.len(), 3);
    }

    #[test]
    fn test_build_style_none_when_no_props() {
        let n = node(r#"label "hello""#);
        let theme = Theme::dark();
        assert!(build_style(&n, &theme).is_none());
    }

    #[test]
    fn test_build_style_with_props() {
        let n = node(r#"card bg=#1a1a2e radius=8 opacity=0.9"#);
        let theme = Theme::dark();
        let s = build_style(&n, &theme).unwrap();
        assert!(s.bg.is_some());
        assert_eq!(s.corner_radius, Some(8.0));
        assert_eq!(s.opacity, Some(0.9));
    }

    #[test]
    fn test_build_style_per_corner_radius() {
        let n = node(r#"container radius=[8, 8, 0, 0]"#);
        let theme = Theme::dark();
        let s = build_style(&n, &theme).unwrap();
        assert_eq!(s.per_corner_radius, Some([8.0, 8.0, 0.0, 0.0]));
        assert!(s.corner_radius.is_none());
    }

    #[test]
    fn test_named_color_prop() {
        let n = node(r#"label "x" color=accent"#);
        let theme = Theme::dark();
        let c = color_prop(&n, "color", &theme);
        assert!(c.is_some());
        assert_eq!(c.unwrap().r, theme.accent.r);
    }

    #[test]
    fn test_elevation_preset() {
        let n = node(r#"card elevation=high"#);
        let theme = Theme::dark();
        let e = elevation(&n, &theme).unwrap();
        assert_eq!(e.blur, theme.elevation_high.blur);
    }

    #[test]
    fn test_elevation_custom() {
        let n = node(r#"card shadow-blur=10 shadow-dx=2 shadow-dy=4"#);
        let theme = Theme::dark();
        let e = elevation(&n, &theme).unwrap();
        assert_eq!(e.blur, 10.0);
        assert_eq!(e.dx, 2.0);
        assert_eq!(e.dy, 4.0);
    }

    #[test]
    fn test_gradient_linear() {
        let n = node(r#"card gradient=linear gradient-to=#ff0000 gradient-angle=45"#);
        let theme = Theme::dark();
        let g = gradient(&n, &theme).unwrap();
        match g {
            Gradient::Linear { angle, .. } => assert_eq!(angle, 45.0),
            _ => panic!("expected linear gradient"),
        }
    }
}
