use std::collections::HashMap;

use taffy::prelude::*;
use taffy::style_helpers::{auto, fr, length};

#[derive(Clone, Copy, Debug)]
struct TextMeasure {
    characters: usize,
    glyph_width: f32,
    line_height: f32,
}

#[derive(Clone, Debug)]
struct Props {
    key: &'static str,
    style: Style,
    text: Option<TextMeasure>,
    paint: bool,
    interactive: bool,
    semantic: bool,
    clips_children: bool,
    scroll_y: f32,
}

#[derive(Clone, Debug)]
struct Element {
    props: Props,
    children: Vec<Element>,
}

impl Element {
    fn new(key: &'static str, style: Style) -> Self {
        Self {
            props: Props {
                key,
                style,
                text: None,
                paint: true,
                interactive: false,
                semantic: false,
                clips_children: false,
                scroll_y: 0.0,
            },
            children: Vec::new(),
        }
    }

    fn children(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }

    fn text(mut self, characters: usize) -> Self {
        self.props.text = Some(TextMeasure {
            characters,
            glyph_width: 8.0,
            line_height: 18.0,
        });
        self.props.semantic = true;
        self
    }

    fn interactive(mut self) -> Self {
        self.props.interactive = true;
        self.props.semantic = true;
        self
    }

    fn clip_and_scroll(mut self, scroll_y: f32) -> Self {
        self.props.clips_children = true;
        self.props.scroll_y = scroll_y;
        self
    }

    fn hidden_from_paint(mut self) -> Self {
        self.props.paint = false;
        self
    }

    fn hidden(mut self) -> Self {
        self.props.style.display = Display::None;
        self.props.paint = false;
        self.props.interactive = false;
        self.props.semantic = false;
        self
    }
}

fn fixed(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: length(width),
            height: length(height),
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}

fn column(height: Option<f32>) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: Dimension::auto(),
            height: height.map_or_else(Dimension::auto, length),
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
struct SceneState {
    viewport_width: f32,
    changed: bool,
    scroll_y: f32,
}

fn declare_scene(calls: &mut usize, state: SceneState) -> Element {
    *calls += 1;

    let text_characters = if state.changed { 26 } else { 18 };
    let text = Element::new("constrained-text", Style::default()).text(text_characters);
    let action = Element::new("grid-action", fixed(60.0, 28.0)).interactive();
    let action = if state.changed {
        action.hidden()
    } else {
        action
    };
    let grid_children = vec![text, action];
    let grid = Element::new(
        "grid",
        Style {
            display: Display::Grid,
            size: Size {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            grid_template_columns: vec![length(72.0), fr(1.0)],
            grid_template_rows: vec![auto()],
            flex_shrink: 0.0,
            ..Default::default()
        },
    )
    .children(grid_children);

    let row_a = Element::new("row-a", fixed(210.0, 46.0)).interactive();
    let row_b = Element::new("row-b", fixed(210.0, 46.0)).interactive();
    let row_c = Element::new("row-c", fixed(210.0, 46.0)).interactive();
    let rows = if state.changed {
        vec![
            row_c,
            row_b,
            Element::new("row-d", fixed(210.0, 46.0)).interactive(),
        ]
    } else {
        vec![row_a, row_b, row_c]
    };
    let scroll_content = Element::new("scroll-content", column(None)).children(rows);
    let scroll = Element::new("scroll", column(Some(82.0)))
        .clip_and_scroll(state.scroll_y)
        .children(vec![scroll_content]);

    let header = Element::new("header", fixed(240.0, 38.0)).interactive();
    let mut main_style = column(None);
    main_style.flex_grow = 1.0;
    main_style.min_size.width = length(0.0);
    let main = Element::new("main", main_style)
        .children(vec![header, grid, scroll])
        .hidden_from_paint();
    let navigation = Element::new("navigation", fixed(80.0, 230.0)).interactive();

    let overlay = Element::new(
        "blocking-overlay",
        Style {
            position: Position::Absolute,
            inset: Rect {
                left: length(108.0),
                top: length(52.0),
                right: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::auto(),
            },
            size: Size {
                width: length(150.0),
                height: length(68.0),
            },
            ..Default::default()
        },
    )
    .interactive();

    Element::new(
        "root",
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::FLEX_START),
            size: Size {
                width: length(state.viewport_width),
                height: length(230.0),
            },
            ..Default::default()
        },
    )
    .children(vec![navigation, main, overlay])
    .hidden_from_paint()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LogicalRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl LogicalRect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedItem {
    key: &'static str,
    paint_rect: LogicalRect,
    hit_rect: Option<LogicalRect>,
    clip_rect: Option<LogicalRect>,
    semantic_rect: Option<LogicalRect>,
    damage_rect: LogicalRect,
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    items: Vec<ResolvedItem>,
}

impl Snapshot {
    fn item(&self, key: &str) -> Option<&ResolvedItem> {
        self.items.iter().find(|item| item.key == key)
    }

    fn hit_test(&self, x: f32, y: f32) -> Option<&'static str> {
        self.items.iter().rev().find_map(|item| {
            let hit = item.hit_rect?;
            if !hit.contains(x, y) {
                return None;
            }
            if item.clip_rect.is_some_and(|clip| !clip.contains(x, y)) {
                return None;
            }
            Some(item.key)
        })
    }
}

fn solve(root: &Element) -> Snapshot {
    fn build(
        element: &Element,
        tree: &mut TaffyTree<TextMeasure>,
        nodes: &mut HashMap<&'static str, NodeId>,
    ) -> NodeId {
        let children: Vec<_> = element
            .children
            .iter()
            .map(|child| build(child, tree, nodes))
            .collect();
        let node = if children.is_empty() {
            if let Some(text) = element.props.text {
                tree.new_leaf_with_context(element.props.style.clone(), text)
                    .unwrap()
            } else {
                tree.new_leaf(element.props.style.clone()).unwrap()
            }
        } else {
            tree.new_with_children(element.props.style.clone(), &children)
                .unwrap()
        };
        assert!(nodes.insert(element.props.key, node).is_none());
        node
    }

    let mut tree = TaffyTree::new();
    let mut nodes = HashMap::new();
    let root_node = build(root, &mut tree, &mut nodes);
    tree.compute_layout_with_measure(
        root_node,
        Size::MAX_CONTENT,
        |known, available, _, context, _| {
            let Some(text) = context else {
                return Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                };
            };
            let natural_width = text.characters as f32 * text.glyph_width;
            let available_width = match available.width {
                AvailableSpace::Definite(width) => width,
                _ => natural_width,
            };
            let width = known.width.unwrap_or(natural_width.min(available_width));
            let lines = (natural_width / width.max(1.0)).ceil();
            Size {
                width,
                height: known.height.unwrap_or(lines * text.line_height),
            }
        },
    )
    .unwrap();

    fn collect(
        element: &Element,
        tree: &TaffyTree<TextMeasure>,
        nodes: &HashMap<&'static str, NodeId>,
        origin: (f32, f32),
        inherited_clip: Option<LogicalRect>,
        output: &mut Vec<ResolvedItem>,
    ) {
        let layout = tree.unrounded_layout(nodes[&element.props.key]);
        let rect = LogicalRect {
            x: origin.0 + layout.location.x,
            y: origin.1 + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };
        let own_clip = if element.props.clips_children {
            Some(inherited_clip.map_or(rect, |clip| clip.intersection(rect).unwrap_or(rect)))
        } else {
            inherited_clip
        };
        if element.props.paint {
            output.push(ResolvedItem {
                key: element.props.key,
                paint_rect: rect,
                hit_rect: element.props.interactive.then_some(rect),
                clip_rect: inherited_clip,
                semantic_rect: element.props.semantic.then_some(rect),
                damage_rect: rect,
            });
        }
        let child_origin = (rect.x, rect.y - element.props.scroll_y);
        for child in &element.children {
            collect(child, tree, nodes, child_origin, own_clip, output);
        }
    }

    let mut items = Vec::new();
    collect(root, &tree, &nodes, (0.0, 0.0), None, &mut items);
    Snapshot { items }
}

trait Candidate {
    fn name(&self) -> &'static str;
    fn render(&mut self, declaration: Element) -> Snapshot;
}

#[derive(Clone)]
enum RecordedCommand {
    Begin(Box<Props>),
    End,
}

#[derive(Default)]
struct RecordedCommands;

impl RecordedCommands {
    fn record(element: Element, commands: &mut Vec<RecordedCommand>) {
        commands.push(RecordedCommand::Begin(Box::new(element.props)));
        for child in element.children {
            Self::record(child, commands);
        }
        commands.push(RecordedCommand::End);
    }

    fn materialize(commands: &[RecordedCommand], cursor: &mut usize) -> Element {
        let RecordedCommand::Begin(props) = &commands[*cursor] else {
            panic!("expected begin command");
        };
        *cursor += 1;
        let mut children = Vec::new();
        while !matches!(commands[*cursor], RecordedCommand::End) {
            children.push(Self::materialize(commands, cursor));
        }
        *cursor += 1;
        Element {
            props: props.as_ref().clone(),
            children,
        }
    }
}

impl Candidate for RecordedCommands {
    fn name(&self) -> &'static str {
        "recorded widget/paint commands"
    }

    fn render(&mut self, declaration: Element) -> Snapshot {
        let mut commands = Vec::new();
        Self::record(declaration, &mut commands);
        let root = Self::materialize(&commands, &mut 0);
        solve(&root)
    }
}

#[derive(Default)]
struct LightweightElementTree;

impl Candidate for LightweightElementTree {
    fn name(&self) -> &'static str {
        "lightweight element tree"
    }

    fn render(&mut self, declaration: Element) -> Snapshot {
        solve(&declaration)
    }
}

#[derive(Clone)]
struct RetainedNode {
    props: Props,
    children: Vec<RetainedNode>,
}

impl RetainedNode {
    fn as_element(&self) -> Element {
        Element {
            props: self.props.clone(),
            children: self.children.iter().map(Self::as_element).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ReconcileStats {
    inserted: usize,
    reused: usize,
    removed: usize,
    moved: usize,
}

#[derive(Default)]
struct ReconciledTree {
    root: Option<RetainedNode>,
    last_stats: ReconcileStats,
}

impl ReconciledTree {
    fn node_count(node: &RetainedNode) -> usize {
        1 + node.children.iter().map(Self::node_count).sum::<usize>()
    }

    fn inserted(element: Element, stats: &mut ReconcileStats) -> RetainedNode {
        stats.inserted += 1;
        RetainedNode {
            props: element.props,
            children: element
                .children
                .into_iter()
                .map(|child| Self::inserted(child, stats))
                .collect(),
        }
    }

    fn reconcile(
        old: Option<RetainedNode>,
        incoming: Element,
        stats: &mut ReconcileStats,
    ) -> RetainedNode {
        let Some(old) = old.filter(|node| node.props.key == incoming.props.key) else {
            return Self::inserted(incoming, stats);
        };
        stats.reused += 1;
        let mut old_children: HashMap<_, _> = old
            .children
            .into_iter()
            .enumerate()
            .map(|(index, node)| (node.props.key, (index, node)))
            .collect();
        let children = incoming
            .children
            .into_iter()
            .enumerate()
            .map(|(new_index, child)| {
                let prior = old_children.remove(child.props.key);
                if prior
                    .as_ref()
                    .is_some_and(|(old_index, _)| *old_index != new_index)
                {
                    stats.moved += 1;
                }
                Self::reconcile(prior.map(|(_, node)| node), child, stats)
            })
            .collect();
        stats.removed += old_children
            .into_values()
            .map(|(_, node)| Self::node_count(&node))
            .sum::<usize>();
        RetainedNode {
            props: incoming.props,
            children,
        }
    }
}

impl Candidate for ReconciledTree {
    fn name(&self) -> &'static str {
        "reconciled tree behind immediate facade"
    }

    fn render(&mut self, declaration: Element) -> Snapshot {
        let mut stats = ReconcileStats::default();
        self.root = Some(Self::reconcile(self.root.take(), declaration, &mut stats));
        self.last_stats = stats;
        solve(&self.root.as_ref().unwrap().as_element())
    }
}

fn verify_candidate(candidate: &mut dyn Candidate) -> (Snapshot, Snapshot) {
    let initial = SceneState {
        viewport_width: 320.0,
        changed: false,
        scroll_y: 24.0,
    };
    let changed = SceneState {
        viewport_width: 360.0,
        changed: true,
        scroll_y: 40.0,
    };
    let mut declaration_calls = 0;
    let first = candidate.render(declare_scene(&mut declaration_calls, initial));
    assert_eq!(
        declaration_calls, 1,
        "declaration closure replayed in frame 1"
    );
    let stable = candidate.render(declare_scene(&mut declaration_calls, initial));
    assert_eq!(
        declaration_calls, 2,
        "declaration closure replayed in frame 2"
    );
    assert_eq!(first, stable, "{} has a warm-up frame", candidate.name());

    let changed_snapshot = candidate.render(declare_scene(&mut declaration_calls, changed));
    assert_eq!(
        declaration_calls, 3,
        "declaration replayed on structural change"
    );
    assert!(
        changed_snapshot.item("row-a").is_none(),
        "removed row was painted"
    );
    assert!(
        changed_snapshot.item("grid-action").is_none(),
        "hidden child was painted"
    );
    assert!(
        changed_snapshot.item("row-d").is_some(),
        "inserted row was not painted"
    );
    let row_c = changed_snapshot.item("row-c").unwrap().paint_rect;
    let row_b = changed_snapshot.item("row-b").unwrap().paint_rect;
    assert!(row_c.y < row_b.y, "reordered rows retained stale positions");

    let text = first.item("constrained-text").unwrap();
    assert!(text.paint_rect.width <= 72.0 && text.paint_rect.height >= 36.0);
    assert_ne!(
        text.paint_rect,
        changed_snapshot
            .item("constrained-text")
            .unwrap()
            .paint_rect,
        "text metric change retained stale geometry"
    );
    assert_ne!(
        first.item("grid").unwrap().paint_rect.width,
        changed_snapshot.item("grid").unwrap().paint_rect.width,
        "viewport change retained stale geometry"
    );
    let overlay = first.item("blocking-overlay").unwrap();
    assert_eq!(
        first.hit_test(overlay.paint_rect.x + 2.0, overlay.paint_rect.y + 2.0),
        Some("blocking-overlay")
    );
    let scrolled = first.item("row-a").unwrap();
    assert!(
        scrolled.clip_rect.is_some(),
        "scroll child has no current clip"
    );
    assert_eq!(scrolled.paint_rect, scrolled.damage_rect);
    assert_eq!(scrolled.hit_rect, scrolled.semantic_rect);
    let clipped_row = first.item("row-c").unwrap();
    assert_eq!(
        first.hit_test(
            clipped_row.paint_rect.x + 2.0,
            clipped_row.paint_rect.y + clipped_row.paint_rect.height - 2.0
        ),
        None,
        "hit testing ignored the current scroll clip"
    );
    (first, changed_snapshot)
}

fn run_spike() {
    let mut recorded = RecordedCommands;
    let mut elements = LightweightElementTree;
    let mut reconciled = ReconciledTree::default();

    let (recorded_first, recorded_changed) = verify_candidate(&mut recorded);
    println!(
        "PASS {}: first frame and structural change",
        recorded.name()
    );
    let (element_first, element_changed) = verify_candidate(&mut elements);
    println!(
        "PASS {}: first frame and structural change",
        elements.name()
    );
    let (reconciled_first, reconciled_changed) = verify_candidate(&mut reconciled);
    println!(
        "PASS {}: first frame and structural change",
        reconciled.name()
    );

    assert_eq!(recorded_first, element_first);
    assert_eq!(element_first, reconciled_first);
    assert_eq!(recorded_changed, element_changed);
    assert_eq!(element_changed, reconciled_changed);
    assert!(reconciled.last_stats.inserted > 0);
    assert!(reconciled.last_stats.removed > 0);
    assert!(reconciled.last_stats.moved > 0);
    println!("PASS all candidates: identical paint, clip, hit, semantic, and damage geometry");
    println!("PASS closure safety: exactly one application declaration per rendered frame");
    println!(
        "reconciled structural-change stats: {:?}",
        reconciled.last_stats
    );
}

fn main() {
    run_spike();
}

#[cfg(test)]
mod tests {
    #[test]
    fn current_frame_candidates_meet_the_spike_contract() {
        super::run_spike();
    }
}
