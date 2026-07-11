use esox_ui::layout::{
    Direction as EsoxDirection, FlexWrap as EsoxFlexWrap, GridPlacement, GridTrack, Rect,
};
use esox_ui::layout_tree::LayoutTree;
use taffy::prelude::*;
use taffy::style_helpers::{auto, fr, length, line};

const EPSILON: f32 = 0.05;

fn approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn fixed_leaf(taffy: &mut TaffyTree<()>, width: f32, height: f32) -> NodeId {
    taffy
        .new_leaf(Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .unwrap()
}

fn esox_flex_grow() -> Vec<[f32; 4]> {
    let mut tree = LayoutTree::new();
    let root = tree.add_node(0);
    tree.node_mut(root).is_leaf = false;
    tree.node_mut(root).style.direction = EsoxDirection::Horizontal;

    for (key, grow) in [(1, 1.0), (2, 3.0)] {
        let child = tree.add_node(key);
        tree.node_mut(child).intrinsic = (50.0, 30.0);
        tree.node_mut(child).style.flex_grow = grow;
        tree.append_child(root, child);
    }

    tree.solve(Rect::new(0.0, 0.0, 200.0, 100.0));
    [1, 2]
        .into_iter()
        .map(|key| {
            let rect = tree.lookup(key).unwrap();
            [rect.x, rect.y, rect.w, rect.h]
        })
        .collect()
}

fn taffy_flex_grow() -> Vec<[f32; 4]> {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let children: Vec<_> = [1.0, 3.0]
        .into_iter()
        .map(|grow| {
            tree.new_leaf(Style {
                size: Size {
                    width: length(50.0),
                    height: length(30.0),
                },
                flex_grow: grow,
                flex_shrink: 1.0,
                ..Default::default()
            })
            .unwrap()
        })
        .collect();
    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::FLEX_START),
                size: Size {
                    width: length(200.0),
                    height: length(100.0),
                },
                ..Default::default()
            },
            &children,
        )
        .unwrap();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();
    children
        .into_iter()
        .map(|node| {
            let layout = tree.unrounded_layout(node);
            [
                layout.location.x,
                layout.location.y,
                layout.size.width,
                layout.size.height,
            ]
        })
        .collect()
}

fn compare_flex_grow() {
    let esox = esox_flex_grow();
    let taffy = taffy_flex_grow();
    for (actual, expected) in taffy.into_iter().zip(esox) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            approx(actual, expected);
        }
    }
    println!("PASS flex-grow: Esox and Taffy agree on 1:3 distribution");
}

fn compare_flex_shrink() {
    let mut esox = LayoutTree::new();
    let root = esox.add_node(0);
    esox.node_mut(root).is_leaf = false;
    esox.node_mut(root).style.direction = EsoxDirection::Horizontal;
    for (key, width) in [(1, 200.0), (2, 100.0)] {
        let child = esox.add_node(key);
        esox.node_mut(child).intrinsic = (width, 30.0);
        esox.node_mut(child).style.flex_shrink = 1.0;
        esox.append_child(root, child);
    }
    esox.solve(Rect::new(0.0, 0.0, 200.0, 100.0));

    let mut taffy: TaffyTree<()> = TaffyTree::new();
    let children: Vec<_> = [200.0, 100.0]
        .into_iter()
        .map(|width| {
            taffy
                .new_leaf(Style {
                    size: Size {
                        width: length(width),
                        height: length(30.0),
                    },
                    flex_shrink: 1.0,
                    ..Default::default()
                })
                .unwrap()
        })
        .collect();
    let root = taffy
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::FLEX_START),
                size: Size {
                    width: length(200.0),
                    height: length(100.0),
                },
                ..Default::default()
            },
            &children,
        )
        .unwrap();
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    for (index, child) in children.into_iter().enumerate() {
        let esox_rect = esox.lookup(index as u64 + 1).unwrap();
        let taffy_rect = taffy.unrounded_layout(child);
        approx(taffy_rect.size.width, esox_rect.w);
        approx(taffy_rect.location.x, esox_rect.x);
    }
    println!("PASS flex-shrink: both use weighted shrink (133.33px and 66.67px)");
}

fn compare_flex_wrap() {
    let mut esox = LayoutTree::new();
    let root = esox.add_node(0);
    esox.node_mut(root).is_leaf = false;
    esox.node_mut(root).style.direction = EsoxDirection::Horizontal;
    esox.node_mut(root).style.flex_wrap = EsoxFlexWrap::Wrap;
    for key in 1..=3 {
        let child = esox.add_node(key);
        esox.node_mut(child).intrinsic = (80.0, 30.0);
        esox.node_mut(child).style.flex_shrink = 0.0;
        esox.append_child(root, child);
    }
    esox.solve(Rect::new(0.0, 0.0, 200.0, 200.0));

    let mut taffy: TaffyTree<()> = TaffyTree::new();
    let children: Vec<_> = (0..3).map(|_| fixed_leaf(&mut taffy, 80.0, 30.0)).collect();
    let root = taffy
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: Some(AlignItems::FLEX_START),
                align_content: Some(AlignContent::FLEX_START),
                size: Size {
                    width: length(200.0),
                    height: length(200.0),
                },
                ..Default::default()
            },
            &children,
        )
        .unwrap();
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    for (index, child) in children.into_iter().enumerate() {
        let esox_rect = esox.lookup(index as u64 + 1).unwrap();
        let taffy_rect = taffy.unrounded_layout(child);
        approx(taffy_rect.location.x, esox_rect.x);
        approx(taffy_rect.location.y, esox_rect.y);
    }
    println!("PASS flex-wrap: both place 80px items at (0,0), (80,0), (0,30)");
}

fn compare_grid() {
    let mut esox = LayoutTree::new();
    let root = esox.add_node(0);
    esox.node_mut(root).is_leaf = false;
    esox.node_mut(root).style.grid_columns = Some(vec![
        GridTrack::Fixed(50.0),
        GridTrack::Fr(1.0),
        GridTrack::Auto,
    ]);
    esox.node_mut(root).style.grid_rows = Some(vec![GridTrack::Auto]);
    for (key, column, intrinsic_width) in [(1, 0, 20.0), (2, 1, 30.0), (3, 2, 70.0)] {
        let child = esox.add_node(key);
        esox.node_mut(child).intrinsic = (intrinsic_width, 25.0);
        esox.node_mut(child).style.grid_placement = Some(GridPlacement::at(column, 0));
        esox.append_child(root, child);
    }
    esox.solve(Rect::new(0.0, 0.0, 300.0, 100.0));

    let mut taffy: TaffyTree<()> = TaffyTree::new();
    let children: Vec<_> = [(1, 20.0), (2, 30.0), (3, 70.0)]
        .into_iter()
        .map(|(column, width)| {
            taffy
                .new_leaf(Style {
                    size: Size {
                        width: length(width),
                        height: length(25.0),
                    },
                    grid_column: line(column),
                    grid_row: line(1),
                    ..Default::default()
                })
                .unwrap()
        })
        .collect();
    let root = taffy
        .new_with_children(
            Style {
                display: Display::Grid,
                size: Size {
                    width: length(300.0),
                    height: length(100.0),
                },
                grid_template_columns: vec![length(50.0), fr(1.0), auto()],
                grid_template_rows: vec![auto()],
                ..Default::default()
            },
            &children,
        )
        .unwrap();
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    for (index, child) in children.into_iter().enumerate() {
        let esox_rect = esox.lookup(index as u64 + 1).unwrap();
        let taffy_rect = taffy.unrounded_layout(child);
        approx(taffy_rect.location.x, esox_rect.x);
    }
    approx(esox.lookup(3).unwrap().w, 70.0);
    println!("PASS grid: fixed/fr/auto track starts agree; auto track resolves to 70px");
}

#[derive(Debug)]
struct TextContext {
    characters: usize,
    glyph_width: f32,
    line_height: f32,
}

fn demonstrate_intrinsic_text() {
    let mut tree: TaffyTree<TextContext> = TaffyTree::new();
    let text = tree
        .new_leaf_with_context(
            Style::default(),
            TextContext {
                characters: 20,
                glyph_width: 10.0,
                line_height: 20.0,
            },
        )
        .unwrap();
    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(80.0),
                    height: auto(),
                },
                ..Default::default()
            },
            &[text],
        )
        .unwrap();
    tree.compute_layout_with_measure(
        root,
        Size::MAX_CONTENT,
        |known, available, _, context, _| {
            let context = context.unwrap();
            let natural_width = context.characters as f32 * context.glyph_width;
            let available_width = match available.width {
                AvailableSpace::Definite(width) => width,
                _ => natural_width,
            };
            let width = known.width.unwrap_or(natural_width.min(available_width));
            let lines = (natural_width / width.max(1.0)).ceil();
            Size {
                width,
                height: known.height.unwrap_or(lines * context.line_height),
            }
        },
    )
    .unwrap();
    let layout = tree.unrounded_layout(text);
    approx(layout.size.width, 80.0);
    approx(layout.size.height, 60.0);

    let mut esox = LayoutTree::new();
    let root = esox.add_node(0);
    esox.node_mut(root).is_leaf = false;
    let text = esox.add_node(1);
    esox.node_mut(text).intrinsic = (200.0, 20.0);
    esox.append_child(root, text);
    esox.solve(Rect::new(0.0, 0.0, 80.0, 200.0));
    let fixed_intrinsic = esox.lookup(1).unwrap();
    approx(fixed_intrinsic.h, 20.0);
    println!(
        "DIFF intrinsic text: Taffy remeasures to 80x60; current Esox retains fixed 200x20 intrinsic size"
    );
}

fn demonstrate_scroll_content_size() {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let child = fixed_leaf(&mut tree, 100.0, 300.0);
    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                overflow: taffy::geometry::Point {
                    x: taffy::style::Overflow::Hidden,
                    y: taffy::style::Overflow::Scroll,
                },
                size: Size {
                    width: length(100.0),
                    height: length(100.0),
                },
                ..Default::default()
            },
            &[child],
        )
        .unwrap();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();
    let layout = tree.unrounded_layout(root);
    approx(layout.size.height, 100.0);
    assert!(layout.content_size.height >= 300.0);
    println!(
        "PASS scroll sizing: viewport is 100px and Taffy reports at least 300px content height; scrolling state remains Esox-owned"
    );
}

fn main() {
    compare_flex_grow();
    compare_flex_shrink();
    compare_flex_wrap();
    compare_grid();
    demonstrate_intrinsic_text();
    demonstrate_scroll_content_size();
}
