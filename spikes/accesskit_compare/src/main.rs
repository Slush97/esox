use std::collections::{BTreeMap, BTreeSet};

use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Rect as AccessRect, Role as AccessRole,
    TextPosition, TextSelection, Tree, TreeId, TreeUpdate,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Role {
    Window,
    Button,
    TextInput,
    TextRun,
    Slider,
    Table,
    Row,
    Cell,
    Dialog,
    ScrollView,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
enum SemanticAction {
    Click,
    Focus,
    ReplaceSelectedText,
    SetTextSelection,
    SetValue,
    Increment,
    Decrement,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Selection {
    text_run: u64,
    anchor: usize,
    focus: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SemanticNode {
    id: u64,
    role: Role,
    label: Option<String>,
    value: Option<String>,
    bounds: Rect,
    visible_bounds: Option<Rect>,
    children: Vec<u64>,
    actions: BTreeSet<SemanticAction>,
    selection: Option<Selection>,
    character_lengths: Vec<u8>,
    numeric_range: Option<(f64, f64, f64, f64)>,
    modal: bool,
    clips_children: bool,
    row_index: Option<usize>,
    column_index: Option<usize>,
}

impl SemanticNode {
    fn new(id: u64, role: Role, bounds: Rect) -> Self {
        Self {
            id,
            role,
            label: None,
            value: None,
            bounds,
            visible_bounds: Some(bounds),
            children: Vec::new(),
            actions: BTreeSet::new(),
            selection: None,
            character_lengths: Vec::new(),
            numeric_range: None,
            modal: false,
            clips_children: false,
            row_index: None,
            column_index: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SemanticSnapshot {
    window_id: u64,
    root: u64,
    focus: u64,
    nodes: BTreeMap<u64, SemanticNode>,
}

impl SemanticSnapshot {
    fn node(&self, id: u64) -> &SemanticNode {
        self.nodes.get(&id).expect("representative node must exist")
    }

    fn to_accesskit(&self) -> TreeUpdate {
        let nodes = self
            .nodes
            .values()
            .map(|source| (NodeId(source.id), adapt_node(source)))
            .collect();
        let mut tree = Tree::new(NodeId(self.root));
        tree.toolkit_name = Some("Esox".into());
        TreeUpdate {
            nodes,
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus: NodeId(self.focus),
        }
    }

    fn route_action(&self, request: &ActionRequest) -> Option<RoutedAction> {
        if request.target_tree != TreeId::ROOT {
            return None;
        }
        let target = self.nodes.get(&request.target_node.0)?;
        let action = match request.action {
            Action::Click => SemanticAction::Click,
            Action::Focus => SemanticAction::Focus,
            Action::ReplaceSelectedText => SemanticAction::ReplaceSelectedText,
            Action::SetTextSelection => SemanticAction::SetTextSelection,
            Action::SetValue => SemanticAction::SetValue,
            Action::Increment => SemanticAction::Increment,
            Action::Decrement => SemanticAction::Decrement,
            _ => return None,
        };
        target.actions.contains(&action).then(|| RoutedAction {
            window_id: self.window_id,
            node_id: target.id,
            action,
            data: request.data.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RoutedAction {
    window_id: u64,
    node_id: u64,
    action: SemanticAction,
    data: Option<ActionData>,
}

fn access_role(role: Role) -> AccessRole {
    match role {
        Role::Window => AccessRole::Window,
        Role::Button => AccessRole::Button,
        Role::TextInput => AccessRole::TextInput,
        Role::TextRun => AccessRole::TextRun,
        Role::Slider => AccessRole::Slider,
        Role::Table => AccessRole::Table,
        Role::Row => AccessRole::Row,
        Role::Cell => AccessRole::Cell,
        Role::Dialog => AccessRole::Dialog,
        Role::ScrollView => AccessRole::ScrollView,
    }
}

fn adapt_node(source: &SemanticNode) -> Node {
    let mut node = Node::new(access_role(source.role));
    if let Some(label) = &source.label {
        node.set_label(label.as_str());
    }
    if let Some(value) = &source.value {
        node.set_value(value.as_str());
    }
    let bounds = source.visible_bounds.unwrap_or(source.bounds);
    node.set_bounds(AccessRect {
        x0: bounds.x,
        y0: bounds.y,
        x1: bounds.x + bounds.width,
        y1: bounds.y + bounds.height,
    });
    node.set_children(
        source
            .children
            .iter()
            .copied()
            .map(NodeId)
            .collect::<Vec<_>>(),
    );
    for action in &source.actions {
        node.add_action(match action {
            SemanticAction::Click => Action::Click,
            SemanticAction::Focus => Action::Focus,
            SemanticAction::ReplaceSelectedText => Action::ReplaceSelectedText,
            SemanticAction::SetTextSelection => Action::SetTextSelection,
            SemanticAction::SetValue => Action::SetValue,
            SemanticAction::Increment => Action::Increment,
            SemanticAction::Decrement => Action::Decrement,
        });
    }
    if let Some(selection) = &source.selection {
        node.set_text_selection(TextSelection {
            anchor: TextPosition {
                node: NodeId(selection.text_run),
                character_index: selection.anchor,
            },
            focus: TextPosition {
                node: NodeId(selection.text_run),
                character_index: selection.focus,
            },
        });
    }
    if !source.character_lengths.is_empty() {
        node.set_character_lengths(source.character_lengths.clone());
    }
    if let Some((minimum, maximum, value, step)) = source.numeric_range {
        node.set_min_numeric_value(minimum);
        node.set_max_numeric_value(maximum);
        node.set_numeric_value(value);
        node.set_numeric_value_step(step);
    }
    if source.modal {
        node.set_modal();
    }
    if source.clips_children {
        node.set_clips_children();
    }
    if let Some(index) = source.row_index {
        node.set_row_index(index);
    }
    if let Some(index) = source.column_index {
        node.set_column_index(index);
    }
    node
}

fn representative_snapshot(window_id: u64) -> SemanticSnapshot {
    let window = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let viewport = Rect {
        x: 20.0,
        y: 300.0,
        width: 300.0,
        height: 100.0,
    };
    let clipped_row = Rect {
        x: 20.0,
        y: 370.0,
        width: 300.0,
        height: 80.0,
    };
    let mut nodes = BTreeMap::new();

    let mut root = SemanticNode::new(1, Role::Window, window);
    root.label = Some(format!("Esox window {window_id}"));
    root.children = vec![10, 20, 30, 40, 50];
    nodes.insert(root.id, root);

    let mut button = SemanticNode::new(
        10,
        Role::Button,
        Rect {
            x: 20.0,
            y: 20.0,
            width: 100.0,
            height: 32.0,
        },
    );
    button.label = Some("Save".into());
    button
        .actions
        .extend([SemanticAction::Click, SemanticAction::Focus]);
    nodes.insert(button.id, button);

    let mut input = SemanticNode::new(
        20,
        Role::TextInput,
        Rect {
            x: 20.0,
            y: 70.0,
            width: 260.0,
            height: 36.0,
        },
    );
    input.label = Some("Message".into());
    input.children = vec![21];
    input.selection = Some(Selection {
        text_run: 21,
        anchor: 1,
        focus: 2,
    });
    input.actions.extend([
        SemanticAction::Focus,
        SemanticAction::ReplaceSelectedText,
        SemanticAction::SetTextSelection,
    ]);
    nodes.insert(input.id, input);

    let mut text = SemanticNode::new(
        21,
        Role::TextRun,
        Rect {
            x: 26.0,
            y: 76.0,
            width: 130.0,
            height: 20.0,
        },
    );
    text.value = Some("Ae\u{301} 👨‍👩‍👧‍👦".into());
    text.character_lengths = vec![1, 3, 1, 25];
    nodes.insert(text.id, text);

    let mut slider = SemanticNode::new(
        30,
        Role::Slider,
        Rect {
            x: 20.0,
            y: 130.0,
            width: 200.0,
            height: 28.0,
        },
    );
    slider.label = Some("Volume".into());
    slider.numeric_range = Some((0.0, 100.0, 40.0, 5.0));
    slider.actions.extend([
        SemanticAction::Focus,
        SemanticAction::SetValue,
        SemanticAction::Increment,
        SemanticAction::Decrement,
    ]);
    nodes.insert(slider.id, slider);

    let mut table = SemanticNode::new(
        40,
        Role::Table,
        Rect {
            x: 20.0,
            y: 180.0,
            width: 300.0,
            height: 90.0,
        },
    );
    table.label = Some("Builds".into());
    table.children = vec![41];
    nodes.insert(table.id, table);
    let mut row = SemanticNode::new(
        41,
        Role::Row,
        Rect {
            x: 20.0,
            y: 210.0,
            width: 300.0,
            height: 30.0,
        },
    );
    row.row_index = Some(0);
    row.children = vec![42];
    nodes.insert(row.id, row);
    let mut cell = SemanticNode::new(
        42,
        Role::Cell,
        Rect {
            x: 20.0,
            y: 210.0,
            width: 150.0,
            height: 30.0,
        },
    );
    cell.column_index = Some(0);
    cell.label = Some("Passing".into());
    nodes.insert(cell.id, cell);

    let mut scroll = SemanticNode::new(50, Role::ScrollView, viewport);
    scroll.clips_children = true;
    scroll.children = vec![51];
    nodes.insert(scroll.id, scroll);
    let mut clipped = SemanticNode::new(51, Role::Button, clipped_row);
    clipped.label = Some("Partly visible action".into());
    clipped.visible_bounds = clipped_row.intersection(viewport);
    clipped.actions.insert(SemanticAction::Click);
    nodes.insert(clipped.id, clipped);

    let mut dialog = SemanticNode::new(
        60,
        Role::Dialog,
        Rect {
            x: 180.0,
            y: 120.0,
            width: 440.0,
            height: 260.0,
        },
    );
    dialog.label = Some("Confirm save".into());
    dialog.modal = true;
    dialog.children = vec![61];
    nodes.insert(dialog.id, dialog);
    let mut confirm = SemanticNode::new(
        61,
        Role::Button,
        Rect {
            x: 500.0,
            y: 330.0,
            width: 90.0,
            height: 32.0,
        },
    );
    confirm.label = Some("Confirm".into());
    confirm
        .actions
        .extend([SemanticAction::Click, SemanticAction::Focus]);
    nodes.insert(confirm.id, confirm);

    SemanticSnapshot {
        window_id,
        root: 1,
        focus: 10,
        nodes,
    }
}

fn main() {
    let snapshot = representative_snapshot(7);
    let serialized = serde_json::to_string_pretty(&snapshot).expect("snapshot must serialize");
    let update = snapshot.to_accesskit();
    let routed = snapshot
        .route_action(&ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: NodeId(snapshot.node(10).id),
            data: None,
        })
        .expect("button action must route");
    println!("AccessKit 0.24.1 adapter spike");
    println!("semantic nodes: {}", snapshot.nodes.len());
    println!("AccessKit nodes: {}", update.nodes.len());
    println!("snapshot bytes: {}", serialized.len());
    println!("focus: {}", update.focus.0);
    println!(
        "routed action: window {} node {}",
        routed.window_id, routed.node_id
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapted(update: &TreeUpdate, id: u64) -> &Node {
        &update
            .nodes
            .iter()
            .find(|(node_id, _)| node_id.0 == id)
            .unwrap()
            .1
    }

    #[test]
    fn adapts_representative_semantics() {
        let snapshot = representative_snapshot(7);
        let update = snapshot.to_accesskit();

        assert!(adapted(&update, 10).supports_action(Action::Click));
        assert_eq!(
            adapted(&update, 20)
                .text_selection()
                .unwrap()
                .focus
                .character_index,
            2
        );
        assert_eq!(adapted(&update, 21).character_lengths(), &[1, 3, 1, 25]);
        assert_eq!(adapted(&update, 30).numeric_value(), Some(40.0));
        assert_eq!(adapted(&update, 41).children(), &[NodeId(42)]);
        assert!(adapted(&update, 60).is_modal());
        assert_eq!(update.focus, NodeId(10));
        assert!(adapted(&update, 50).clips_children());
        assert_eq!(adapted(&update, 51).bounds().unwrap().y1, 400.0);
    }

    #[test]
    fn snapshot_is_headless_stable_and_round_trips() {
        let snapshot = representative_snapshot(7);
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        assert_eq!(
            serde_json::from_str::<SemanticSnapshot>(&json).unwrap(),
            snapshot
        );
        assert_eq!(json, serde_json::to_string_pretty(&snapshot).unwrap());
    }

    #[test]
    fn actions_route_to_owned_identity() {
        let snapshot = representative_snapshot(7);
        let request = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: NodeId(10),
            data: None,
        };
        let routed = snapshot.route_action(&request).unwrap();
        assert_eq!((routed.window_id, routed.node_id), (7, 10));
        assert!(
            snapshot
                .route_action(&ActionRequest {
                    target_node: NodeId(21),
                    ..request
                })
                .is_none()
        );
    }

    #[test]
    fn windows_have_independent_stable_id_spaces() {
        let first = representative_snapshot(7);
        let second = representative_snapshot(8);
        assert_eq!(first.node(10).id, second.node(10).id);
        assert_ne!(first.window_id, second.window_id);
        let keys = [
            (first.window_id, first.node(10).id),
            (second.window_id, second.node(10).id),
        ];
        assert_ne!(keys[0], keys[1]);
    }

    #[test]
    fn modal_focus_is_an_explicit_snapshot_update() {
        let mut snapshot = representative_snapshot(7);
        snapshot.node(60);
        snapshot
            .nodes
            .get_mut(&snapshot.root)
            .unwrap()
            .children
            .push(60);
        snapshot.focus = 61;
        let update = snapshot.to_accesskit();
        assert_eq!(update.focus, NodeId(61));
        assert!(adapted(&update, 60).is_modal());
    }
}
