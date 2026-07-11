use std::cell::{Cell, RefCell};
use std::convert::Infallible;

use esox_gfx::{Color as GfxColor, Frame, ShapeBuilder};
use esox_ui::declaration::{
    run_declaration_frame, ButtonIds, ButtonStyle, DeclarationStyle, GridStyle, GridTrack,
};
use esox_ui::frame_core::{
    Axis, Color, CommittedScene, DeterministicMeasurer, Element, FrameCore, LogicalPoint,
    LogicalRect, LogicalSize, NullSceneConsumer, PaintPrimitive, PointerEventKind,
    SemanticProperties, SemanticRole, WidgetId,
};
use esox_ui::frame_scene_consumer::submit_display_list;
use esox_ui::scene_submission::{TextPaintBoundary, TextPaintRequest};

const ROOT: WidgetId = WidgetId(1_000);
const STATE_COLUMN: WidgetId = WidgetId(1_001);
const INNER_ROW: WidgetId = WidgetId(1_002);
const GRID: WidgetId = WidgetId(1_003);
const ACTION: WidgetId = WidgetId(1_004);
const SIBLING: WidgetId = WidgetId(1_005);
const DIRECT_DISABLED: WidgetId = WidgetId(1_006);

const ACTION_COLOR: Color = Color::rgba(0.15, 0.35, 0.75, 1.0);
const SIBLING_COLOR: Color = Color::rgba(0.20, 0.65, 0.35, 1.0);
const DISABLED_COLOR: Color = Color::rgba(0.45, 0.45, 0.48, 1.0);
const BORDER_COLOR: Color = Color::rgba(0.08, 0.12, 0.20, 1.0);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Clone, Copy)]
struct Participation {
    hidden: bool,
    disabled: bool,
}

impl Participation {
    const ENABLED: Self = Self {
        hidden: false,
        disabled: false,
    };
    const HIDDEN: Self = Self {
        hidden: true,
        disabled: false,
    };
    const DISABLED: Self = Self {
        hidden: false,
        disabled: true,
    };
}

#[derive(Default)]
struct ClosureCounts {
    applications: Cell<usize>,
    rows: Cell<usize>,
    columns: Cell<usize>,
    nested_rows: Cell<usize>,
    grids: Cell<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservedResponse {
    clicked: bool,
    hovered: bool,
    pressed: bool,
    disabled: bool,
}

#[derive(Default)]
struct HeadlessText {
    requests: Vec<(WidgetId, LogicalRect, Option<LogicalRect>)>,
}

impl TextPaintBoundary<Frame> for HeadlessText {
    type Error = Infallible;

    fn paint_text(
        &mut self,
        frame: &mut Frame,
        request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error> {
        self.requests
            .push((request.id, request.bounds, request.effective_clip));
        frame.push(
            ShapeBuilder::rect(
                request.bounds.x,
                request.bounds.y,
                request.bounds.width,
                request.bounds.height,
            )
            .color(GfxColor::new(
                request.color.r,
                request.color.g,
                request.color.b,
                request.color.a,
            ))
            .build(),
        );
        Ok(())
    }
}

fn declare_production_tree(
    ui: &mut esox_ui::declaration::DeclarationUi<'_>,
    participation: Participation,
    counts: &ClosureCounts,
) -> ObservedResponse {
    counts.applications.set(counts.applications.get() + 1);
    let response = RefCell::new(None);
    ui.row(
        ROOT,
        DeclarationStyle::new().gap(10.0).clip_children(),
        |ui| {
            counts.rows.set(counts.rows.get() + 1);
            ui.column(
                STATE_COLUMN,
                DeclarationStyle::new()
                    .size(100.0, 60.0)
                    .with_hidden(participation.hidden)
                    .with_disabled(participation.disabled),
                |ui| {
                    counts.columns.set(counts.columns.get() + 1);
                    ui.row(INNER_ROW, DeclarationStyle::new().size(100.0, 60.0), |ui| {
                        counts.nested_rows.set(counts.nested_rows.get() + 1);
                        ui.grid(
                            GRID,
                            GridStyle::new(vec![GridTrack::Fixed(100.0)])
                                .layout(DeclarationStyle::new().size(100.0, 60.0).clip_children()),
                            |ui| {
                                counts.grids.set(counts.grids.get() + 1);
                                let current = ui.button(
                                    ACTION,
                                    "Action",
                                    ButtonStyle::default()
                                        .layout(DeclarationStyle::new().size(100.0, 40.0))
                                        .fill(ACTION_COLOR)
                                        .border(BORDER_COLOR, 2.0),
                                );
                                response.replace(Some(ObservedResponse {
                                    clicked: current.clicked,
                                    hovered: current.hovered,
                                    pressed: current.pressed,
                                    disabled: current.disabled,
                                }));
                            },
                        );
                    });
                },
            );
            let sibling = ui.button(
                SIBLING,
                "Sibling",
                ButtonStyle::default()
                    .layout(DeclarationStyle::new().size(80.0, 40.0))
                    .fill(SIBLING_COLOR),
            );
            assert!(!sibling.disabled);
            let direct = ui.button(
                DIRECT_DISABLED,
                "Direct",
                ButtonStyle::default()
                    .layout(DeclarationStyle::new().size(70.0, 40.0))
                    .fill(DISABLED_COLOR)
                    .disabled(),
            );
            assert!(direct.disabled);
            assert!(!direct.clicked);
            assert!(!direct.hovered);
            assert!(!direct.pressed);
        },
    );
    response
        .into_inner()
        .expect("the nested grid declares exactly one action")
}

fn interaction_bridge_tree() -> Element {
    let action = Element::fixed(ACTION, 100.0, 40.0)
        .with_paint(PaintPrimitive::Border {
            color: BORDER_COLOR,
            width: 2.0,
        })
        .with_semantics(SemanticProperties::new(SemanticRole::Button).with_label("Action"))
        .interactive();
    let state_column = Element::flex(STATE_COLUMN, Axis::Column, 0.0)
        .without_paint()
        .with_size(Some(100.0), Some(60.0))
        .with_children(vec![Element::flex(INNER_ROW, Axis::Row, 0.0)
            .without_paint()
            .with_size(Some(100.0), Some(60.0))
            .with_children(vec![Element::grid(
                GRID,
                vec![GridTrack::Fixed(100.0)],
                0.0,
            )
            .without_paint()
            .with_size(Some(100.0), Some(60.0))
            .clip_children()
            .with_children(vec![action])])]);
    Element::flex(ROOT, Axis::Row, 10.0)
        .without_paint()
        .clip_children()
        .with_children(vec![
            state_column,
            Element::fixed(SIBLING, 80.0, 40.0)
                .with_semantics(SemanticProperties::new(SemanticRole::Button).with_label("Sibling"))
                .interactive(),
            Element::fixed(DIRECT_DISABLED, 70.0, 40.0).disabled(),
        ])
}

fn assert_all_products_agree(scene: &CommittedScene) {
    assert_eq!(
        scene.focus_order,
        scene
            .hit_index
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>()
    );
    for node in &scene.nodes {
        let paint = scene
            .display_list
            .iter()
            .find(|record| record.id == node.id);
        let hit = scene.hit_index.iter().find(|record| record.id == node.id);
        let semantic = scene.semantics.node(node.id);
        let damage = scene.damage.iter().find(|record| record.id == node.id);

        assert_eq!(paint.map(|record| record.bounds), node.paint_bounds);
        assert_eq!(hit.map(|record| record.bounds), node.hit_bounds);
        assert_eq!(semantic.map(|record| record.bounds), node.semantic_bounds);
        assert_eq!(
            damage.map(|record| record.current_bounds),
            node.current_damage_bounds
        );
        if let Some(record) = paint {
            assert_eq!(record.effective_clip, node.effective_clip);
        }
        if let Some(record) = hit {
            assert_eq!(record.effective_clip, node.effective_clip);
        }
        if let Some(record) = semantic {
            assert_eq!(record.effective_clip, node.effective_clip);
            assert_eq!(record.properties.disabled, node.effective_disabled);
        }
        if let Some(record) = damage {
            assert_eq!(record.effective_clip, node.effective_clip);
        }
        if node.effective_hidden {
            assert_eq!(node.effective_clip, None);
            assert!(paint.is_none());
            assert!(hit.is_none());
            assert!(semantic.is_none());
            assert!(damage.is_none());
        }
        if node.effective_disabled {
            assert!(hit.is_none());
            assert!(!scene.focus_order.contains(&node.id));
        }
    }
}

fn run_declared_state(
    core: &mut FrameCore,
    measurer: &DeterministicMeasurer,
    consumer: &mut NullSceneConsumer,
    participation: Participation,
    counts: &ClosureCounts,
) -> (CommittedScene, ObservedResponse) {
    let observed = RefCell::new(None);
    let scene = run_declaration_frame(core, measurer, consumer, |ui| {
        observed.replace(Some(declare_production_tree(ui, participation, counts)));
    })
    .unwrap()
    .clone();
    let observed = observed
        .into_inner()
        .expect("the application declaration executes exactly once");
    (scene, observed)
}

fn establish_focus_and_capture(
    core: &mut FrameCore,
    measurer: &DeterministicMeasurer,
    consumer: &mut NullSceneConsumer,
    pointer: u64,
) {
    core.run_frame(measurer, consumer, |state| {
        state.request_keyboard_focus(ACTION);
        state.request_pointer_capture(pointer, ACTION);
        interaction_bridge_tree()
    })
    .unwrap();
    assert_eq!(core.keyboard_focus(), Some(ACTION));
    assert_eq!(core.pointer_capture(pointer), Some(ACTION));
}

#[test]
fn production_hidden_and_disabled_transitions_are_current_inherited_and_renderer_ready() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(300.0, 80.0));
    let counts = ClosureCounts::default();
    let action_ids = ButtonIds::new(ACTION);
    let sibling_ids = ButtonIds::new(SIBLING);
    let direct_ids = ButtonIds::new(DIRECT_DISABLED);

    let (enabled, enabled_response) = run_declared_state(
        &mut core,
        &measurer,
        &mut consumer,
        Participation::ENABLED,
        &counts,
    );
    assert_eq!(enabled.generation, 1);
    assert_eq!(
        enabled_response,
        ObservedResponse {
            clicked: false,
            hovered: false,
            pressed: false,
            disabled: false,
        }
    );
    assert_eq!(
        enabled.node(ROOT).unwrap().bounds,
        rect(0.0, 0.0, 300.0, 80.0)
    );
    assert_eq!(
        enabled.node(STATE_COLUMN).unwrap().bounds,
        rect(0.0, 0.0, 100.0, 60.0)
    );
    assert_eq!(
        enabled.node(INNER_ROW).unwrap().bounds,
        rect(0.0, 0.0, 100.0, 60.0)
    );
    assert_eq!(
        enabled.node(GRID).unwrap().bounds,
        rect(0.0, 0.0, 100.0, 60.0)
    );
    assert_eq!(
        enabled.node(ACTION).unwrap().bounds,
        rect(0.0, 0.0, 100.0, 40.0)
    );
    assert_eq!(
        enabled.node(SIBLING).unwrap().bounds,
        rect(110.0, 0.0, 80.0, 40.0)
    );
    assert_eq!(
        enabled.node(DIRECT_DISABLED).unwrap().bounds,
        rect(200.0, 0.0, 70.0, 40.0)
    );
    assert_eq!(enabled.focus_order, vec![ACTION, SIBLING]);
    assert!(
        enabled
            .semantics
            .node(DIRECT_DISABLED)
            .unwrap()
            .properties
            .disabled
    );
    for id in [DIRECT_DISABLED, direct_ids.fill, direct_ids.label] {
        assert!(enabled.node(id).unwrap().effective_disabled);
        assert!(enabled.node(id).unwrap().hit_bounds.is_none());
    }
    assert_all_products_agree(&enabled);

    establish_focus_and_capture(&mut core, &measurer, &mut consumer, 9);
    core.queue_pointer_event(PointerEventKind::Press, 0, LogicalPoint::new(20.0, 20.0));
    core.queue_pointer_event(PointerEventKind::Release, 0, LogicalPoint::new(20.0, 20.0));
    let (disabled, disabled_response) = run_declared_state(
        &mut core,
        &measurer,
        &mut consumer,
        Participation::DISABLED,
        &counts,
    );
    assert_eq!(
        disabled_response,
        ObservedResponse {
            clicked: false,
            hovered: false,
            pressed: false,
            disabled: true,
        }
    );
    for id in [
        STATE_COLUMN,
        INNER_ROW,
        GRID,
        ACTION,
        action_ids.fill,
        action_ids.label,
    ] {
        let node = disabled.node(id).unwrap();
        assert!(node.effective_disabled);
        assert!(!node.effective_hidden);
        assert_eq!(node.bounds, enabled.node(id).unwrap().bounds);
    }
    assert_eq!(disabled.display_list, enabled.display_list);
    assert_eq!(disabled.damage, enabled.damage);
    assert_eq!(disabled.focus_order, vec![SIBLING]);
    assert_eq!(core.keyboard_focus(), Some(SIBLING));
    assert_eq!(core.pointer_capture(9), None);
    let disabled_cancel = core.take_cancellation().unwrap();
    assert_eq!(disabled_cancel.kind, PointerEventKind::Cancel);
    assert_eq!(disabled_cancel.pointer, 9);
    assert_eq!(disabled_cancel.target, ACTION);
    assert_eq!(disabled_cancel.committed_generation, 2);
    assert_eq!(core.take_cancellation(), None);
    for id in [ACTION, action_ids.label] {
        assert!(disabled.semantics.node(id).unwrap().properties.disabled);
    }
    assert_all_products_agree(&disabled);

    let disabled_before_submission = disabled.clone();
    let mut frame = Frame::new();
    let mut text = HeadlessText::default();
    submit_display_list(&disabled.display_list, &mut frame, &mut text).unwrap();
    assert_eq!(disabled, disabled_before_submission);
    assert_eq!(frame.instance_data().len(), disabled.display_list.len());
    assert_eq!(
        frame
            .instance_data()
            .iter()
            .map(|instance| instance.rect)
            .collect::<Vec<_>>(),
        disabled
            .display_list
            .iter()
            .map(|record| [
                record.bounds.x,
                record.bounds.y,
                record.bounds.width,
                record.bounds.height,
            ])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        frame
            .instance_data()
            .iter()
            .map(|instance| instance.clip_rect)
            .collect::<Vec<_>>(),
        disabled
            .display_list
            .iter()
            .map(|record| record.effective_clip.map_or([0.0; 4], |clip| [
                clip.x,
                clip.y,
                clip.width,
                clip.height,
            ]))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        text.requests
            .iter()
            .map(|request| request.0)
            .collect::<Vec<_>>(),
        vec![action_ids.label, sibling_ids.label, direct_ids.label]
    );

    let (restored_from_disabled, restored_response) = run_declared_state(
        &mut core,
        &measurer,
        &mut consumer,
        Participation::ENABLED,
        &counts,
    );
    assert!(!restored_response.disabled);
    assert!(!restored_response.clicked);
    assert_eq!(
        restored_from_disabled.node(ACTION).unwrap().bounds,
        enabled.node(ACTION).unwrap().bounds
    );
    assert_eq!(restored_from_disabled.focus_order, vec![ACTION, SIBLING]);
    assert!(restored_from_disabled
        .node(ACTION)
        .unwrap()
        .hit_bounds
        .is_some());
    assert!(
        !restored_from_disabled
            .semantics
            .node(ACTION)
            .unwrap()
            .properties
            .disabled
    );
    assert_all_products_agree(&restored_from_disabled);

    establish_focus_and_capture(&mut core, &measurer, &mut consumer, 11);
    core.queue_pointer_event(PointerEventKind::Press, 0, LogicalPoint::new(20.0, 20.0));
    core.queue_pointer_event(PointerEventKind::Release, 0, LogicalPoint::new(20.0, 20.0));
    let (hidden, hidden_response) = run_declared_state(
        &mut core,
        &measurer,
        &mut consumer,
        Participation::HIDDEN,
        &counts,
    );
    assert_eq!(
        hidden_response,
        ObservedResponse {
            clicked: false,
            hovered: false,
            pressed: false,
            disabled: false,
        }
    );
    for id in [
        STATE_COLUMN,
        INNER_ROW,
        GRID,
        ACTION,
        action_ids.fill,
        action_ids.label,
    ] {
        let node = hidden.node(id).unwrap();
        assert!(node.effective_hidden);
        assert!(!node.effective_disabled);
        assert_eq!(node.paint_bounds, None);
        assert_eq!(node.hit_bounds, None);
        assert_eq!(node.semantic_bounds, None);
        assert_eq!(node.current_damage_bounds, None);
        assert_eq!(node.effective_clip, None);
    }
    assert_eq!(
        hidden.node(STATE_COLUMN).unwrap().bounds,
        rect(0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        hidden.node(SIBLING).unwrap().bounds,
        rect(0.0, 0.0, 80.0, 40.0)
    );
    assert_eq!(
        hidden.node(DIRECT_DISABLED).unwrap().bounds,
        rect(90.0, 0.0, 70.0, 40.0)
    );
    assert_eq!(hidden.focus_order, vec![SIBLING]);
    assert_eq!(core.keyboard_focus(), Some(SIBLING));
    assert_eq!(core.pointer_capture(11), None);
    let hidden_cancel = core.take_cancellation().unwrap();
    assert_eq!(hidden_cancel.kind, PointerEventKind::Cancel);
    assert_eq!(hidden_cancel.pointer, 11);
    assert_eq!(hidden_cancel.target, ACTION);
    assert_eq!(hidden_cancel.committed_generation, 5);
    assert_eq!(core.take_cancellation(), None);
    assert_all_products_agree(&hidden);

    let (restored_from_hidden, final_response) = run_declared_state(
        &mut core,
        &measurer,
        &mut consumer,
        Participation::ENABLED,
        &counts,
    );
    assert_eq!(
        final_response,
        ObservedResponse {
            clicked: false,
            hovered: false,
            pressed: false,
            disabled: false,
        }
    );
    for id in [
        STATE_COLUMN,
        INNER_ROW,
        GRID,
        ACTION,
        action_ids.fill,
        action_ids.label,
    ] {
        let node = restored_from_hidden.node(id).unwrap();
        assert!(!node.effective_hidden);
        assert!(!node.effective_disabled);
        assert_eq!(node.bounds, enabled.node(id).unwrap().bounds);
    }
    assert_eq!(restored_from_hidden.display_list, enabled.display_list);
    assert_eq!(restored_from_hidden.damage, enabled.damage);
    assert_eq!(restored_from_hidden.focus_order, vec![ACTION, SIBLING]);
    assert_all_products_agree(&restored_from_hidden);

    assert_eq!(counts.applications.get(), 5);
    assert_eq!(counts.rows.get(), 5);
    assert_eq!(counts.columns.get(), 5);
    assert_eq!(counts.nested_rows.get(), 5);
    assert_eq!(counts.grids.get(), 5);
}
