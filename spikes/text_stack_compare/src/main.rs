use std::collections::BTreeSet;

use cosmic_text::{
    Action as CosmicAction, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Motion, Selection,
    Shaping,
};
use parley::{FontContext, Layout, LayoutContext, StyleProperty};
use unicode_bidi::BidiInfo;
use unicode_linebreak::{BreakOpportunity, linebreaks};
use unicode_segmentation::UnicodeSegmentation;

const EDIT_TEXT: &str = "Ae\u{301} 👨‍👩‍👧‍👦 नमस्ते";
const LAYOUT_TEXT: &str = "English العربية 日本語 👨‍👩‍👧‍👦 wraps across lines";

#[derive(Debug)]
struct LayoutEvidence {
    lines: usize,
    clusters: usize,
    rtl_runs: usize,
    fonts: usize,
    cursor_stops: Vec<usize>,
}

fn print_evidence(name: &str, evidence: &LayoutEvidence) {
    println!(
        "{name}: {} lines, {} clusters, {} RTL runs, {} fonts, {} cursor stops",
        evidence.lines,
        evidence.clusters,
        evidence.rtl_runs,
        evidence.fonts,
        evidence.cursor_stops.len()
    );
}

fn unicode_boundaries(text: &str) -> (Vec<usize>, Vec<usize>, usize) {
    let mut graphemes = text
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    graphemes.push(text.len());
    let breaks = linebreaks(text)
        .filter_map(|(index, opportunity)| {
            matches!(
                opportunity,
                BreakOpportunity::Allowed | BreakOpportunity::Mandatory
            )
            .then_some(index)
        })
        .collect();
    let bidi_paragraphs = BidiInfo::new(text, None).paragraphs.len();
    (graphemes, breaks, bidi_paragraphs)
}

fn parley_layout(text: &str, width: f32) -> LayoutEvidence {
    let mut fonts = FontContext::new();
    let mut layouts = LayoutContext::new();
    let mut builder = layouts.ranged_builder(&mut fonts, text, 1.0, true);
    builder.push_default(StyleProperty::FontSize(18.0));
    let mut layout: Layout<()> = builder.build(text);
    layout.break_all_lines(Some(width));

    let mut clusters = 0;
    let mut rtl_runs = 0;
    let mut font_keys = BTreeSet::new();
    let mut cursor_stops = BTreeSet::from([0, text.len()]);
    for line in layout.lines() {
        for run in line.runs() {
            rtl_runs += usize::from(run.is_rtl());
            font_keys.insert((run.font().data.id(), run.font().index));
            for cluster in run.clusters() {
                clusters += 1;
                cursor_stops.insert(cluster.text_range().start);
                cursor_stops.insert(cluster.text_range().end);
            }
        }
    }

    LayoutEvidence {
        lines: layout.lines().count(),
        clusters,
        rtl_runs,
        fonts: font_keys.len(),
        cursor_stops: cursor_stops.into_iter().collect(),
    }
}

fn cosmic_layout(text: &str, width: f32) -> LayoutEvidence {
    let mut fonts = FontSystem::new();
    let mut buffer = Buffer::new(&mut fonts, Metrics::new(18.0, 24.0));
    buffer.set_size(Some(width), None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut fonts, false);

    let mut lines = 0;
    let mut clusters = 0;
    let mut rtl_runs = 0;
    let mut font_keys = BTreeSet::new();
    let mut cursor_stops = BTreeSet::from([0, text.len()]);
    for run in buffer.layout_runs() {
        lines += 1;
        rtl_runs += usize::from(run.rtl || run.glyphs.iter().any(|glyph| glyph.level.is_rtl()));
        for glyph in run.glyphs {
            clusters += 1;
            font_keys.insert(glyph.font_id);
            cursor_stops.insert(glyph.start);
            cursor_stops.insert(glyph.end);
        }
    }

    LayoutEvidence {
        lines,
        clusters,
        rtl_runs,
        fonts: font_keys.len(),
        cursor_stops: cursor_stops.into_iter().collect(),
    }
}

fn rustybuzz_layout(text: &str) -> LayoutEvidence {
    let bytes = include_bytes!("../../../test-data/JetBrainsMono-Regular.ttf");
    let face = rustybuzz::Face::from_slice(bytes, 0).expect("test font must load");
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let shaped = rustybuzz::shape(&face, &[], buffer);
    let cursor_stops = shaped
        .glyph_infos()
        .iter()
        .map(|glyph| glyph.cluster as usize)
        .chain([text.len()])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    LayoutEvidence {
        lines: 1,
        clusters: shaped.len(),
        rtl_runs: 0,
        fonts: 1,
        cursor_stops,
    }
}

fn parley_editing_evidence() -> (Vec<usize>, std::ops::Range<usize>, bool) {
    let mut fonts = FontContext::new();
    let mut layouts = LayoutContext::new();
    let mut editor = parley::editing::PlainEditor::<()>::new(18.0);
    editor.set_text(EDIT_TEXT);
    editor.set_width(Some(240.0));

    let mut stops = vec![editor.raw_selection().focus().index()];
    {
        let mut driver = editor.driver(&mut fonts, &mut layouts);
        for _ in 0..6 {
            driver.move_right();
            stops.push(driver.editor.raw_selection().focus().index());
        }
        driver.select_right();
        driver.set_compose("かな", Some((0, 6)));
        driver.refresh_layout();
        let _ime_area = driver.editor.ime_cursor_area();
    }
    let compose = editor.raw_compose().clone().expect("IME preedit range");
    (stops, compose, editor.is_composing())
}

fn cosmic_editing_evidence() -> (Vec<usize>, bool) {
    let mut fonts = FontSystem::new();
    let mut buffer = Buffer::new(&mut fonts, Metrics::new(18.0, 24.0));
    buffer.set_size(Some(240.0), None);
    buffer.set_text(EDIT_TEXT, &Attrs::new(), Shaping::Advanced, None);
    let mut editor = Editor::new(buffer);
    editor.shape_as_needed(&mut fonts, false);

    let mut stops = vec![editor.cursor().index];
    for _ in 0..6 {
        editor.action(&mut fonts, CosmicAction::Motion(Motion::Next));
        stops.push(editor.cursor().index);
    }
    let anchor = editor.cursor();
    editor.set_selection(Selection::Normal(anchor));
    editor.action(&mut fonts, CosmicAction::Motion(Motion::Next));
    let has_selection = editor.selection_bounds().is_some();
    (stops, has_selection)
}

fn main() {
    let (graphemes, breaks, bidi_paragraphs) = unicode_boundaries(LAYOUT_TEXT);
    let parley = parley_layout(LAYOUT_TEXT, 180.0);
    let cosmic = cosmic_layout(LAYOUT_TEXT, 180.0);
    let current = rustybuzz_layout(LAYOUT_TEXT);
    let (edit_stops, compose, _) = parley_editing_evidence();
    let (cosmic_stops, cosmic_selection) = cosmic_editing_evidence();

    println!("Unicode text-stack spike");
    println!(
        "Unicode boundaries: {} graphemes, {} line breaks, {bidi_paragraphs} bidi paragraph",
        graphemes.len() - 1,
        breaks.len()
    );
    print_evidence("Parley", &parley);
    print_evidence("cosmic-text", &cosmic);
    print_evidence("rustybuzz-only", &current);
    println!("Parley editor stops: {edit_stops:?}, IME preedit: {compose:?}");
    println!("cosmic-text editor stops: {cosmic_stops:?}, selection: {cosmic_selection}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_algorithms_find_graphemes_breaks_and_bidi() {
        let (graphemes, breaks, bidi_paragraphs) = unicode_boundaries(LAYOUT_TEXT);
        assert!(graphemes.windows(2).all(|range| range[0] < range[1]));
        assert!(breaks.len() > 2);
        assert_eq!(bidi_paragraphs, 1);
        assert!(
            EDIT_TEXT
                .graphemes(true)
                .any(|grapheme| grapheme == "e\u{301}")
        );
        assert!(EDIT_TEXT.graphemes(true).any(|grapheme| grapheme == "👨‍👩‍👧‍👦"));
    }

    #[test]
    fn parley_exercises_wrapping_bidi_fallback_and_clusters() {
        let evidence = parley_layout(LAYOUT_TEXT, 180.0);
        assert!(evidence.lines > 1);
        assert!(evidence.rtl_runs > 0);
        assert!(evidence.fonts > 1);
        assert!(evidence.clusters > 10);
        assert!(evidence.cursor_stops.contains(&LAYOUT_TEXT.len()));
    }

    #[test]
    fn cosmic_text_exercises_wrapping_bidi_fallback_and_clusters() {
        let evidence = cosmic_layout(LAYOUT_TEXT, 180.0);
        assert!(evidence.lines > 1);
        assert!(evidence.rtl_runs > 0);
        assert!(evidence.fonts > 1);
        assert!(evidence.clusters > 10);
        assert!(evidence.cursor_stops.contains(&LAYOUT_TEXT.len()));
    }

    #[test]
    fn parley_editor_exposes_ime_data_but_uses_shaped_cluster_stops() {
        let (stops, compose, composing) = parley_editing_evidence();
        let grapheme_stops = EDIT_TEXT
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .chain([EDIT_TEXT.len()])
            .collect::<BTreeSet<_>>();
        assert!(stops.iter().any(|stop| !grapheme_stops.contains(stop)));
        assert!(compose.start < compose.end);
        assert!(composing);
    }

    #[test]
    fn cosmic_editor_navigates_graphemes_and_tracks_selection() {
        let (stops, has_selection) = cosmic_editing_evidence();
        let grapheme_stops = EDIT_TEXT
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .chain([EDIT_TEXT.len()])
            .collect::<BTreeSet<_>>();
        assert!(
            stops.iter().all(|stop| grapheme_stops.contains(stop)),
            "{stops:?}"
        );
        assert_eq!(&stops[..3], &[0, 1, 4]);
        assert!(has_selection);
    }

    #[test]
    fn rustybuzz_alone_is_shaping_not_a_text_layout_interface() {
        let evidence = rustybuzz_layout(LAYOUT_TEXT);
        assert_eq!(evidence.lines, 1);
        assert_eq!(evidence.fonts, 1);
        assert_eq!(evidence.rtl_runs, 0);
    }
}
