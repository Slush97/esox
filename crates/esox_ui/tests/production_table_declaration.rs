use std::cell::{Cell, RefCell};

use esox_input::{Key, KeyCode, KeyEvent, Modifiers, NamedKey};
use esox_ui::declaration::{
    run_declaration_frame, table_cell_id, table_header_cell_id, ButtonStyle, DeclarationStyle,
    TableColumn, TableIds, TableResponse, TableSpec, TableStyle,
};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, PointerEventKind, SemanticRole, TableDeclarationError, WidgetId,
};

const TABLE: WidgetId = WidgetId(300_000);
const FIRST_COLUMN: WidgetId = WidgetId(300_001);
const SECOND_COLUMN: WidgetId = WidgetId(300_002);
const DUPLICATE: WidgetId = WidgetId(399_999);

fn row_id(index: usize) -> WidgetId {
    // Deliberately non-slot-shaped application identities.
    WidgetId(310_000 + (index as u64).wrapping_mul(37))
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn named_key(key: NamedKey, physical_key: KeyCode) -> KeyEvent {
    KeyEvent {
        key: Key::Named(key),
        physical_key,
        pressed: true,
        repeat: false,
        text: None,
    }
}

fn spec() -> TableSpec {
    TableSpec::new(
        TABLE,
        20,
        20.0,
        45.0,
        vec![
            TableColumn::new(FIRST_COLUMN, "Name", 100.0)
                .sortable()
                .resizable(60.0, 180.0),
            TableColumn::new(SECOND_COLUMN, "Value", 80.0),
        ],
    )
    .selectable()
}

fn table_style() -> TableStyle {
    TableStyle {
        layout: DeclarationStyle::new().width(180.0),
        header_height: 30.0,
        ..TableStyle::default()
    }
}

fn declare_table(
    ui: &mut esox_ui::declaration::DeclarationUi<'_>,
    table_spec: TableSpec,
    calls: &RefCell<Vec<usize>>,
    duplicate: bool,
) -> TableResponse {
    declare_table_with_style(ui, table_spec, table_style(), calls, duplicate)
}

fn declare_table_with_style(
    ui: &mut esox_ui::declaration::DeclarationUi<'_>,
    table_spec: TableSpec,
    style: TableStyle,
    calls: &RefCell<Vec<usize>>,
    duplicate: bool,
) -> TableResponse {
    let (_, response) = ui
        .table(table_spec, style, |ui, index| {
            calls.borrow_mut().push(index);
            let row = row_id(index);
            ui.solid_rect(
                if duplicate && index == 0 {
                    DUPLICATE
                } else {
                    table_cell_id(row, FIRST_COLUMN)
                },
                DeclarationStyle::new().height(20.0),
                Color::rgba(0.1, 0.2, 0.3, 1.0),
            );
            ui.solid_rect(
                if duplicate && index == 0 {
                    DUPLICATE
                } else {
                    table_cell_id(row, SECOND_COLUMN)
                },
                DeclarationStyle::new().height(20.0),
                Color::rgba(0.3, 0.2, 0.1, 1.0),
            );
            row
        })
        .unwrap();
    response
}

#[test]
fn exact_visible_rows_are_declared_once_with_shared_current_tracks() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let (window, _) = ui
            .table(spec(), table_style(), |ui, index| {
                calls.borrow_mut().push(index);
                let row = row_id(index);
                ui.solid_rect(
                    table_cell_id(row, FIRST_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                ui.solid_rect(
                    table_cell_id(row, SECOND_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                row
            })
            .unwrap();
        assert_eq!(window.visible_range, 0..3);
    })
    .unwrap();

    assert_eq!(*calls.borrow(), vec![0, 1, 2]);
    let ids = TableIds::new(TABLE);
    assert_eq!(
        scene.node(ids.header).unwrap().bounds,
        rect(0.0, 0.0, 180.0, 30.0)
    );
    assert_eq!(
        scene.node(ids.body).unwrap().bounds,
        rect(0.0, 30.0, 180.0, 45.0)
    );
    assert_eq!(
        scene.node(row_id(0)).unwrap().bounds,
        rect(0.0, 30.0, 180.0, 20.0)
    );
    assert_eq!(
        scene
            .node(table_cell_id(row_id(0), SECOND_COLUMN))
            .unwrap()
            .bounds,
        rect(100.0, 30.0, 80.0, 20.0)
    );
    assert_eq!(
        scene.node(row_id(2)).unwrap().effective_clip,
        Some(rect(0.0, 30.0, 180.0, 45.0))
    );
    assert_eq!(
        scene.semantics.node(ids.body).unwrap().properties.role,
        SemanticRole::ScrollView
    );
    assert_eq!(
        scene.semantics.node(row_id(0)).unwrap().properties.role,
        SemanticRole::Generic
    );

    assert!(core.resize(LogicalSize::new(260.0, 75.0)));
    calls.borrow_mut().clear();
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    assert_eq!(*calls.borrow(), vec![0, 1, 2]);
    assert_eq!(scene.node(ids.body).unwrap().bounds.width, 180.0);
}

#[test]
fn logical_row_and_cell_ids_survive_scroll_slot_changes() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    calls.borrow_mut().clear();

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec().with_offset(40.0), &calls, false);
    })
    .unwrap();
    assert_eq!(*calls.borrow(), vec![2, 3, 4]);
    assert!(scene.node(row_id(0)).is_none());
    assert!(scene.node(row_id(2)).is_some());
    assert!(scene.node(table_cell_id(row_id(2), FIRST_COLUMN)).is_some());
}

#[test]
fn invalid_columns_fail_before_any_row_callback() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = Cell::new(0);
    let invalid = TableSpec::new(
        TABLE,
        10,
        20.0,
        40.0,
        vec![
            TableColumn::new(FIRST_COLUMN, "A", 100.0),
            TableColumn::new(FIRST_COLUMN, "B", 80.0),
        ],
    );
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert!(ui
            .table(invalid, table_style(), |_, _| {
                calls.set(calls.get() + 1);
                row_id(0)
            })
            .is_err());
    })
    .unwrap_err();
    assert_eq!(calls.get(), 0);
    assert_eq!(
        error,
        FrameError::InvalidTable {
            id: TABLE,
            error: TableDeclarationError::DuplicateColumnId(FIRST_COLUMN),
        }
    );
}

#[test]
fn retained_column_widths_are_isolated_per_window_owner() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut first_consumer = NullSceneConsumer::default();
    let mut second_consumer = NullSceneConsumer::default();
    let mut first = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let mut second = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    for (core, consumer) in [
        (&mut first, &mut first_consumer),
        (&mut second, &mut second_consumer),
    ] {
        run_declaration_frame(core, &measurer, consumer, |ui| {
            declare_table(ui, spec(), &calls, false);
        })
        .unwrap();
    }

    first.queue_pointer_event(PointerEventKind::Press, 4, LogicalPoint::new(98.0, 10.0));
    first.queue_pointer_event(PointerEventKind::Move, 4, LogicalPoint::new(128.0, 10.0));
    first.queue_pointer_event(PointerEventKind::Release, 4, LogicalPoint::new(128.0, 10.0));
    let first_scene = run_declaration_frame(&mut first, &measurer, &mut first_consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    let second_scene = run_declaration_frame(&mut second, &measurer, &mut second_consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();

    assert_eq!(
        first_scene
            .node(table_header_cell_id(TABLE, SECOND_COLUMN))
            .unwrap()
            .bounds
            .x,
        130.0
    );
    assert_eq!(
        second_scene
            .node(table_header_cell_id(TABLE, SECOND_COLUMN))
            .unwrap()
            .bounds
            .x,
        100.0
    );
}

#[test]
fn changed_declared_widths_replace_stale_base_widths_in_the_same_generation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(220.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 5, LogicalPoint::new(98.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Move, 5, LogicalPoint::new(128.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Release, 5, LogicalPoint::new(128.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();

    let mut changed = spec();
    changed.columns[0].width = 110.0;
    changed.columns[1].width = 60.0;
    let failed = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, changed.clone(), &calls, true);
    });
    assert!(matches!(
        failed,
        Err(FrameError::DuplicateWidgetId(DUPLICATE))
    ));
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, changed, &calls, false);
    })
    .unwrap();
    assert_eq!(
        scene
            .node(table_header_cell_id(TABLE, SECOND_COLUMN))
            .unwrap()
            .bounds
            .x,
        110.0
    );
    assert_eq!(
        scene
            .node(table_cell_id(row_id(0), SECOND_COLUMN))
            .unwrap()
            .bounds,
        rect(110.0, 30.0, 60.0, 20.0)
    );
}

#[test]
fn aggregate_width_and_zero_handle_are_typed_before_row_callbacks() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    for (columns, style, expected) in [
        (
            vec![
                TableColumn::new(FIRST_COLUMN, "A", f32::MAX),
                TableColumn::new(SECOND_COLUMN, "B", f32::MAX),
            ],
            table_style(),
            TableDeclarationError::UnrepresentableAggregateWidth,
        ),
        (
            spec().columns,
            TableStyle {
                resize_handle_width: 0.0,
                ..table_style()
            },
            TableDeclarationError::InvalidResizeHandleWidth(0.0),
        ),
    ] {
        let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
        let calls = Cell::new(0);
        let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            let _ = ui.table(
                TableSpec::new(TABLE, 10, 20.0, 40.0, columns),
                style,
                |_, _| {
                    calls.set(calls.get() + 1);
                    row_id(0)
                },
            );
        })
        .unwrap_err();
        assert_eq!(calls.get(), 0);
        assert_eq!(
            error,
            FrameError::InvalidTable {
                id: TABLE,
                error: expected
            }
        );
    }
}

#[test]
fn chronological_multi_header_clicks_survive_failure_replay() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    let mut ordered_spec = spec();
    ordered_spec.columns[1].sortable = true;
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, ordered_spec.clone(), &calls, false);
    })
    .unwrap();
    for (pointer, x) in [(10, 20.0), (11, 130.0), (12, 20.0)] {
        core.queue_pointer_event(PointerEventKind::Press, pointer, LogicalPoint::new(x, 10.0));
        core.queue_pointer_event(
            PointerEventKind::Release,
            pointer,
            LogicalPoint::new(x, 10.0),
        );
    }
    let observed = RefCell::new(Vec::new());
    assert!(
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            observed
                .borrow_mut()
                .push(declare_table(ui, ordered_spec.clone(), &calls, true));
        })
        .is_err()
    );
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        observed
            .borrow_mut()
            .push(declare_table(ui, ordered_spec, &calls, false));
    })
    .unwrap();
    assert_eq!(observed.borrow()[0], observed.borrow()[1]);
    assert_eq!(
        observed.borrow()[1].sort_requested,
        vec![FIRST_COLUMN, SECOND_COLUMN, FIRST_COLUMN]
    );
}

#[test]
fn click_then_wheel_selects_the_committed_row_without_offscreen_declaration() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    core.set_wheel_scroll_speed(40.0);
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 20, LogicalPoint::new(20.0, 40.0));
    core.queue_pointer_event(PointerEventKind::Release, 20, LogicalPoint::new(20.0, 40.0));
    assert!(core.queue_wheel(LogicalPoint::new(20.0, 40.0), LogicalPoint::new(0.0, 1.0)));
    calls.borrow_mut().clear();
    let observed = RefCell::new(Vec::new());
    let failed = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let (_, table_response) = ui
            .table(spec(), table_style(), |ui, index| {
                calls.borrow_mut().push(index);
                let row = row_id(index);
                let first = if index == 2 {
                    DUPLICATE
                } else {
                    table_cell_id(row, FIRST_COLUMN)
                };
                let second = if index == 2 {
                    DUPLICATE
                } else {
                    table_cell_id(row, SECOND_COLUMN)
                };
                ui.solid_rect(first, DeclarationStyle::new().height(20.0), Color::BLACK);
                ui.solid_rect(second, DeclarationStyle::new().height(20.0), Color::BLACK);
                row
            })
            .unwrap();
        observed.borrow_mut().push(table_response);
    });
    assert!(failed.is_err());
    calls.borrow_mut().clear();
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        observed
            .borrow_mut()
            .push(declare_table(ui, spec(), &calls, false));
    })
    .unwrap();
    assert_eq!(*calls.borrow(), vec![2, 3, 4]);
    assert_eq!(observed.borrow()[0], observed.borrow()[1]);
    assert_eq!(observed.borrow()[1].selected_row, Some(row_id(0)));
    let next = RefCell::new(TableResponse::default());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        *next.borrow_mut() = declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    assert_eq!(next.borrow().selected_row, None);
}

#[test]
fn nested_cell_control_click_is_not_promoted_to_row_selection() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let button_id = WidgetId(380_000);
    let clicked = Cell::new(false);
    let render = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        let (_, response) = ui
            .table(spec(), table_style(), |ui, index| {
                let row = row_id(index);
                let button = ui.button(
                    if index == 0 {
                        button_id
                    } else {
                        table_cell_id(row, FIRST_COLUMN)
                    },
                    "Action",
                    ButtonStyle::default().layout(DeclarationStyle::new().size(100.0, 20.0)),
                );
                clicked.set(clicked.get() || button.clicked);
                ui.solid_rect(
                    table_cell_id(row, SECOND_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                row
            })
            .unwrap();
        assert_eq!(response.selected_row, None);
    };
    run_declaration_frame(&mut core, &measurer, &mut consumer, render).unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 30, LogicalPoint::new(20.0, 40.0));
    core.queue_pointer_event(PointerEventKind::Release, 30, LogicalPoint::new(20.0, 40.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, render).unwrap();
    assert!(clicked.get());
}

#[test]
fn sort_and_selection_require_release_inside_the_pressed_target() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Press, 40, LogicalPoint::new(20.0, 10.0));
    core.queue_pointer_event(
        PointerEventKind::Release,
        40,
        LogicalPoint::new(250.0, 10.0),
    );
    core.queue_pointer_event(PointerEventKind::Press, 41, LogicalPoint::new(20.0, 40.0));
    core.queue_pointer_event(
        PointerEventKind::Release,
        41,
        LogicalPoint::new(250.0, 40.0),
    );
    let observed = RefCell::new(TableResponse::default());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        *observed.borrow_mut() = declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    assert!(observed.borrow().sort_requested.is_empty());
    assert_eq!(observed.borrow().selected_row, None);
}

#[test]
fn row_click_focuses_the_body_and_arrow_navigation_returns_a_stable_row_id() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 90, LogicalPoint::new(20.0, 40.0));
    core.queue_pointer_event(PointerEventKind::Release, 90, LogicalPoint::new(20.0, 40.0));
    let clicked = RefCell::new(TableResponse::default());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        *clicked.borrow_mut() = declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    assert_eq!(clicked.borrow().selected_row, Some(row_id(0)));
    assert_eq!(core.keyboard_focus(), Some(TableIds::new(TABLE).body));

    core.queue_keyboard_input(
        named_key(NamedKey::ArrowDown, KeyCode::ArrowDown),
        Modifiers::empty(),
    );
    let navigated = RefCell::new(TableResponse::default());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        *navigated.borrow_mut() = declare_table(ui, spec().selected_row(0), &calls, false);
    })
    .unwrap();
    assert_eq!(navigated.borrow().selected_row, Some(row_id(1)));
}

#[test]
fn end_key_selects_and_scrolls_to_an_offscreen_row_in_the_same_generation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.request_keyboard_focus(TableIds::new(TABLE).body);
        declare_table(ui, spec().selected_row(0), &calls, false);
    })
    .unwrap();

    core.queue_keyboard_input(named_key(NamedKey::End, KeyCode::End), Modifiers::empty());
    calls.borrow_mut().clear();
    let observed = RefCell::new(None);
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let (window, response) = ui
            .table(spec().selected_row(0), table_style(), |ui, index| {
                calls.borrow_mut().push(index);
                let row = row_id(index);
                ui.solid_rect(
                    table_cell_id(row, FIRST_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                ui.solid_rect(
                    table_cell_id(row, SECOND_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                row
            })
            .unwrap();
        *observed.borrow_mut() = Some((window, response));
    })
    .unwrap();
    let (window, response) = observed.borrow().clone().unwrap();
    assert!(window.visible_range.contains(&19));
    assert_eq!(response.selected_row, Some(row_id(19)));
    assert!(calls.borrow().contains(&19));
    assert!(scene.node(row_id(19)).is_some());
}

#[test]
fn ordered_table_keyboard_navigation_replays_after_a_failed_generation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.request_keyboard_focus(TableIds::new(TABLE).body);
        declare_table(ui, spec().selected_row(0), &calls, false);
    })
    .unwrap();
    core.queue_keyboard_input(
        named_key(NamedKey::ArrowDown, KeyCode::ArrowDown),
        Modifiers::empty(),
    );
    core.queue_keyboard_input(
        named_key(NamedKey::ArrowDown, KeyCode::ArrowDown),
        Modifiers::empty(),
    );

    let observed = RefCell::new(Vec::new());
    assert!(
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            observed
                .borrow_mut()
                .push(declare_table(ui, spec().selected_row(0), &calls, true));
        })
        .is_err()
    );
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        observed
            .borrow_mut()
            .push(declare_table(ui, spec().selected_row(0), &calls, false));
    })
    .unwrap();
    assert_eq!(observed.borrow()[0], observed.borrow()[1]);
    assert_eq!(observed.borrow()[1].selected_row, Some(row_id(2)));

    let next = RefCell::new(TableResponse::default());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        *next.borrow_mut() = declare_table(ui, spec().selected_row(2), &calls, false);
    })
    .unwrap();
    assert_eq!(next.borrow().selected_row, None);
}

#[test]
fn split_frame_clicks_require_the_same_live_capture_and_clear_after_outside_release() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    let header = table_header_cell_id(TABLE, FIRST_COLUMN);

    core.queue_pointer_event(PointerEventKind::Press, 50, LogicalPoint::new(20.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert!(declare_table(ui, spec(), &calls, false)
            .sort_requested
            .is_empty());
    })
    .unwrap();
    assert_eq!(core.pointer_capture(50), Some(header));
    core.queue_pointer_event(PointerEventKind::Release, 50, LogicalPoint::new(20.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert_eq!(
            declare_table(ui, spec(), &calls, false).sort_requested,
            vec![FIRST_COLUMN]
        );
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 51, LogicalPoint::new(20.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    core.queue_pointer_event(
        PointerEventKind::Release,
        51,
        LogicalPoint::new(250.0, 10.0),
    );
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert!(declare_table(ui, spec(), &calls, false)
            .sort_requested
            .is_empty());
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Release, 51, LogicalPoint::new(20.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert!(declare_table(ui, spec(), &calls, false)
            .sort_requested
            .is_empty());
    })
    .unwrap();
}

#[test]
fn split_frame_row_press_is_cleared_by_feature_toggle_and_offscreen_removal() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 60, LogicalPoint::new(20.0, 40.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    let mut not_selectable = spec();
    not_selectable.selectable = false;
    core.queue_pointer_event(PointerEventKind::Release, 60, LogicalPoint::new(20.0, 40.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert_eq!(
            declare_table(ui, not_selectable.clone(), &calls, false).selected_row,
            None
        );
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Release, 60, LogicalPoint::new(20.0, 40.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert_eq!(declare_table(ui, spec(), &calls, false).selected_row, None);
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 62, LogicalPoint::new(20.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Release, 62, LogicalPoint::new(20.0, 10.0));
    let mut hidden_style = table_style();
    hidden_style.layout = hidden_style.layout.hidden();
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let response = declare_table_with_style(ui, spec(), hidden_style, &calls, false);
        assert!(response.sort_requested.is_empty());
    })
    .unwrap();
    core.queue_pointer_event(PointerEventKind::Release, 62, LogicalPoint::new(20.0, 10.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert!(declare_table(ui, spec(), &calls, false)
            .sort_requested
            .is_empty());
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 61, LogicalPoint::new(20.0, 40.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec().with_offset(40.0), &calls, false);
    })
    .unwrap();
    assert_eq!(core.pointer_capture(61), None);
    core.queue_pointer_event(PointerEventKind::Release, 61, LogicalPoint::new(20.0, 40.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        assert_eq!(declare_table(ui, spec(), &calls, false).selected_row, None);
    })
    .unwrap();
}

#[test]
fn narrow_resizable_track_clamps_its_handle_and_routes_drag() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(83.0, 75.0));
    let mut narrow = spec();
    narrow.columns[0] = TableColumn::new(FIRST_COLUMN, "N", 3.0).resizable(1.0, 20.0);
    let calls = RefCell::new(Vec::new());
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, narrow.clone(), &calls, false);
    })
    .unwrap();
    let handle = TableIds::new(TABLE).resize_handle(FIRST_COLUMN);
    assert_eq!(scene.node(handle).unwrap().bounds.width, 3.0);
    core.queue_pointer_event(PointerEventKind::Press, 31, LogicalPoint::new(1.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Move, 31, LogicalPoint::new(6.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Release, 31, LogicalPoint::new(6.0, 10.0));
    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, narrow, &calls, false);
    })
    .unwrap();
    assert_eq!(
        scene
            .node(table_header_cell_id(TABLE, SECOND_COLUMN))
            .unwrap()
            .bounds
            .x,
        8.0
    );
}

#[test]
fn every_reserved_table_part_collision_is_atomic() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let ids = TableIds::new(TABLE);
    let reserved = [
        ids.root,
        ids.header,
        ids.body,
        ids.header_cell(FIRST_COLUMN),
        ids.header_label(FIRST_COLUMN),
        ids.resize_handle(FIRST_COLUMN),
        ids.virtual_wrapper(0),
    ];
    for collision in reserved {
        let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
        let mut consumer = NullSceneConsumer::default();
        let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.table(spec(), table_style(), |ui, index| {
                let logical_row = row_id(index);
                ui.solid_rect(
                    table_cell_id(logical_row, FIRST_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                ui.solid_rect(
                    table_cell_id(logical_row, SECOND_COLUMN),
                    DeclarationStyle::new().height(20.0),
                    Color::BLACK,
                );
                collision
            })
            .unwrap();
        })
        .unwrap_err();
        assert_eq!(error, FrameError::DuplicateWidgetId(collision));
        assert!(core.committed_scene().is_none());
    }
}

#[test]
fn row_callback_must_declare_one_direct_child_per_column() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let _ = ui.table(spec(), table_style(), |ui, index| {
            ui.solid_rect(
                table_cell_id(row_id(index), FIRST_COLUMN),
                DeclarationStyle::new().height(20.0),
                Color::BLACK,
            );
            row_id(index)
        });
    })
    .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidTable {
            id: TABLE,
            error: TableDeclarationError::InvalidRowCellCount {
                row: 0,
                expected: 2,
                actual: 1,
            },
        }
    );
}

#[test]
fn sort_selection_and_resize_replay_after_atomic_failure() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(180.0, 75.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare_table(ui, spec(), &calls, false);
    })
    .unwrap();

    core.queue_pointer_event(PointerEventKind::Press, 1, LogicalPoint::new(20.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Release, 1, LogicalPoint::new(20.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Press, 2, LogicalPoint::new(20.0, 40.0));
    core.queue_pointer_event(PointerEventKind::Release, 2, LogicalPoint::new(20.0, 40.0));
    core.queue_pointer_event(PointerEventKind::Press, 3, LogicalPoint::new(98.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Move, 3, LogicalPoint::new(118.0, 10.0));
    core.queue_pointer_event(PointerEventKind::Release, 3, LogicalPoint::new(118.0, 10.0));

    let attempts = RefCell::new(Vec::new());
    let failed = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        attempts
            .borrow_mut()
            .push(declare_table(ui, spec(), &calls, true));
    });
    assert!(matches!(
        failed,
        Err(FrameError::DuplicateWidgetId(DUPLICATE))
    ));
    assert_eq!(core.pointer_capture(3), None);

    let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        attempts
            .borrow_mut()
            .push(declare_table(ui, spec(), &calls, false));
    })
    .unwrap();
    let attempts = attempts.into_inner();
    assert_eq!(attempts[0], attempts[1]);
    assert_eq!(attempts[1].sort_requested, vec![FIRST_COLUMN]);
    assert_eq!(attempts[1].selected_row, Some(row_id(0)));
    assert_eq!(attempts[1].resized.len(), 1);
    assert_eq!(attempts[1].resized[0].width, 120.0);
    assert_eq!(
        scene
            .node(table_header_cell_id(TABLE, SECOND_COLUMN))
            .unwrap()
            .bounds
            .x,
        120.0
    );
    assert_eq!(
        scene
            .node(table_cell_id(row_id(0), SECOND_COLUMN))
            .unwrap()
            .bounds
            .x,
        120.0
    );
}
