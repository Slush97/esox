use std::cell::{Cell, RefCell};

use esox_ui::declaration::{
    run_declaration_frame, virtual_item_id, ButtonStyle, DeclarationStyle, LogicalTransform,
};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, FrameError, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, VirtualDeclarationError, VirtualListSpec, WidgetId,
};

const ROOT: WidgetId = WidgetId(80_000);
const OUTER: WidgetId = WidgetId(80_001);
const VIRTUAL: WidgetId = WidgetId(80_002);
const FOOTER: WidgetId = WidgetId(80_003);

fn item_id(index: usize) -> WidgetId {
    WidgetId(81_000 + index as u64)
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn item_style() -> DeclarationStyle {
    DeclarationStyle::new()
        .size(100.0, 20.0)
        .min_size(Some(100.0), Some(20.0))
}

fn render_visible_frame(
    core: &mut FrameCore,
    measurer: &DeterministicMeasurer,
    consumer: &mut NullSceneConsumer,
    height: f32,
    calls: &RefCell<Vec<usize>>,
) {
    run_declaration_frame(core, measurer, consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            let window = ui
                .virtual_column(
                    VirtualListSpec::new(VIRTUAL, 10, 20.0, height),
                    DeclarationStyle::new().width(100.0),
                    |ui, index| {
                        calls.borrow_mut().push(index);
                        ui.solid_rect(
                            item_id(index),
                            item_style(),
                            Color::rgba(0.2, 0.4, 0.6, 1.0),
                        );
                    },
                )
                .unwrap();
            assert_eq!(window.visible_range, 0..if height == 45.0 { 3 } else { 2 });
        });
    })
    .unwrap();
}

fn render_growth_frame(
    core: &mut FrameCore,
    measurer: &DeterministicMeasurer,
    consumer: &mut NullSceneConsumer,
    count: usize,
    explicit: Option<f32>,
    ranges: &RefCell<Vec<std::ops::Range<usize>>>,
) {
    run_declaration_frame(core, measurer, consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            let mut spec = VirtualListSpec::new(VIRTUAL, count, 20.0, 40.0);
            if let Some(offset) = explicit {
                spec = spec.with_offset(offset);
            }
            let window = ui
                .virtual_column(spec, DeclarationStyle::new().width(100.0), |ui, index| {
                    ui.solid_rect(item_id(index), item_style(), Color::BLACK);
                })
                .unwrap();
            ranges.borrow_mut().push(window.visible_range);
        });
    })
    .unwrap();
}

fn render_nested_growth_frame(
    core: &mut FrameCore,
    measurer: &DeterministicMeasurer,
    consumer: &mut NullSceneConsumer,
    count: usize,
    initial: bool,
) {
    run_declaration_frame(core, measurer, consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            let outer = DeclarationStyle::new().size(100.0, 40.0).clip_children();
            let outer = if initial {
                outer.scroll_offset(0.0, 0.0)
            } else {
                outer.scrollable()
            };
            ui.column(OUTER, outer, |ui| {
                let mut spec = VirtualListSpec::new(VIRTUAL, count, 20.0, 40.0);
                if initial {
                    spec = spec.with_offset(20.0);
                }
                ui.virtual_column(spec, DeclarationStyle::new().width(100.0), |ui, index| {
                    ui.solid_rect(item_id(index), item_style(), Color::BLACK);
                })
                .unwrap();
                ui.solid_rect(
                    FOOTER,
                    DeclarationStyle::new()
                        .size(100.0, 100.0)
                        .min_size(Some(100.0), Some(100.0)),
                    Color::BLACK,
                );
            });
        });
    })
    .unwrap();
}

#[test]
fn first_frame_and_resize_declare_only_the_exact_visible_range() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 45.0));
    let calls = RefCell::new(Vec::new());

    render_visible_frame(&mut core, &measurer, &mut consumer, 45.0, &calls);
    assert!(core.resize(LogicalSize::new(100.0, 25.0)));
    render_visible_frame(&mut core, &measurer, &mut consumer, 25.0, &calls);

    assert_eq!(*calls.borrow(), [0, 1, 2, 0, 1]);
    let first = &consumer.scenes()[0];
    assert_eq!(
        first.node(virtual_item_id(VIRTUAL, 0)).unwrap().bounds,
        rect(0.0, 0.0, 100.0, 20.0)
    );
    assert_eq!(
        first.node(virtual_item_id(VIRTUAL, 1)).unwrap().bounds,
        rect(0.0, 20.0, 100.0, 20.0)
    );
    assert_eq!(
        first.node(virtual_item_id(VIRTUAL, 2)).unwrap().bounds,
        rect(0.0, 40.0, 100.0, 20.0)
    );
    assert!(first.node(virtual_item_id(VIRTUAL, 3)).is_none());
    let metrics = first.node(VIRTUAL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.content_extent.height, 200.0);
    assert_eq!(metrics.viewport_extent.height, 45.0);
    assert_eq!(metrics.maximum_offset.y, 155.0);

    let second = &consumer.scenes()[1];
    assert_eq!(
        second.node(virtual_item_id(VIRTUAL, 1)).unwrap().bounds.y,
        20.0
    );
    assert!(second.node(virtual_item_id(VIRTUAL, 2)).is_none());
    assert_eq!(
        second
            .node(VIRTUAL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .viewport_extent
            .height,
        25.0
    );
    for index in 0..2 {
        let id = item_id(index);
        let node = second.node(id).unwrap();
        assert_eq!(node.effective_clip, Some(rect(0.0, 0.0, 100.0, 25.0)));
        assert!(second.display_list.iter().any(|record| record.id == id));
        assert!(second.damage.iter().any(|record| record.id == id));
    }
    assert!(second
        .display_list
        .iter()
        .all(|record| record.id != item_id(2)));
}

#[test]
fn repeated_wheel_intent_at_the_previous_max_accumulates_when_content_grows() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.set_wheel_scroll_speed(40.0);
    let ranges = RefCell::new(Vec::new());

    render_growth_frame(&mut core, &measurer, &mut consumer, 3, Some(20.0), &ranges);
    assert!(core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5)));
    assert!(core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5)));
    render_growth_frame(&mut core, &measurer, &mut consumer, 5, None, &ranges);

    assert_eq!(*ranges.borrow(), [1..3, 3..5]);
    let metrics = consumer.scenes()[1]
        .node(VIRTUAL)
        .unwrap()
        .scroll_metrics
        .unwrap();
    assert_eq!(metrics.requested_offset.y, 60.0);
    assert_eq!(metrics.applied_offset.y, 60.0);
    assert_eq!(metrics.maximum_offset.y, 60.0);
    assert_eq!(
        consumer.scenes()[1]
            .node(virtual_item_id(VIRTUAL, 3))
            .unwrap()
            .bounds
            .y,
        0.0
    );
}

#[test]
fn explicit_offset_wins_and_scroll_to_minimally_reveals_items() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 45.0));

    let mut frame = |spec| {
        let mut observed = None;
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                observed = Some(
                    ui.virtual_column(spec, DeclarationStyle::new().width(100.0), |ui, index| {
                        ui.solid_rect(item_id(index), item_style(), Color::BLACK);
                    })
                    .unwrap(),
                );
            });
        })
        .unwrap();
        observed.unwrap()
    };

    let below = frame(VirtualListSpec::new(VIRTUAL, 10, 20.0, 45.0).scroll_to(5));
    assert_eq!(below.applied_offset, 75.0);
    assert_eq!(below.visible_range, 3..6);
    let above = frame(VirtualListSpec::new(VIRTUAL, 10, 20.0, 45.0).scroll_to(1));
    assert_eq!(above.applied_offset, 20.0);
    assert_eq!(above.visible_range, 1..4);
    let explicit = frame(
        VirtualListSpec::new(VIRTUAL, 10, 20.0, 45.0)
            .with_offset(100.0)
            .scroll_to(0),
    );
    assert_eq!(explicit.applied_offset, 100.0);
    assert_eq!(explicit.visible_range, 5..8);
    let current = frame(VirtualListSpec::new(VIRTUAL, 10, 20.0, 45.0).scroll_to(6));
    assert_eq!(current.applied_offset, 100.0);
    assert_eq!(current.visible_range, 5..8);
    let last = frame(VirtualListSpec::new(VIRTUAL, 10, 20.0, 45.0).scroll_to(9));
    assert_eq!(last.applied_offset, 155.0);
    assert_eq!(last.visible_range, 7..10);
    assert_eq!(
        consumer
            .scenes()
            .last()
            .unwrap()
            .node(virtual_item_id(VIRTUAL, 9))
            .unwrap()
            .bounds
            .y,
        25.0
    );
}

#[test]
fn queued_wheel_is_overridden_by_explicit_offset_and_item_ids_survive_range_shifts() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let ranges = RefCell::new(Vec::new());

    render_growth_frame(&mut core, &measurer, &mut consumer, 10, Some(0.0), &ranges);
    let stable = virtual_item_id(VIRTUAL, 1);
    assert!(consumer.scenes()[0].node(stable).is_some());
    assert!(core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 1.0)));
    render_growth_frame(&mut core, &measurer, &mut consumer, 10, Some(20.0), &ranges);

    let metrics = consumer.scenes()[1]
        .node(VIRTUAL)
        .unwrap()
        .scroll_metrics
        .unwrap();
    assert_eq!(metrics.applied_offset.y, 20.0);
    assert_eq!(*ranges.borrow(), [0..2, 1..3]);
    assert_eq!(consumer.scenes()[1].node(stable).unwrap().id, stable);
}

#[test]
fn planned_height_is_authoritative_under_parent_constraints_and_conflicts_are_early() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 20.0));
    let calls = Cell::new(0);
    let mut planned = None;
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new().size(100.0, 20.0), |ui| {
            planned = Some(
                ui.virtual_column(
                    VirtualListSpec::new(VIRTUAL, 10, 20.0, 40.0),
                    DeclarationStyle::new().width(100.0),
                    |ui, index| {
                        calls.set(calls.get() + 1);
                        ui.solid_rect(item_id(index), item_style(), Color::BLACK);
                    },
                )
                .unwrap(),
            );
        });
    })
    .unwrap();
    let planned = planned.unwrap();
    let metrics = consumer.scenes()[0]
        .node(VIRTUAL)
        .unwrap()
        .scroll_metrics
        .unwrap();
    assert_eq!(planned.visible_range, 0..2);
    assert_eq!(calls.get(), 2);
    assert_eq!(metrics.viewport_extent.height, 40.0);
    assert_eq!(metrics.maximum_offset.y, planned.maximum_offset);

    let committed = core.committed_scene().unwrap().clone();
    let calls = Cell::new(0);
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let _ = ui.virtual_column(
            VirtualListSpec::new(VIRTUAL, 10, 20.0, 40.0),
            DeclarationStyle::new().padding(1.0),
            |_, _| calls.set(calls.get() + 1),
        );
    })
    .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidVirtualContent {
            id: VIRTUAL,
            error: VirtualDeclarationError::ConflictingViewportStyle,
        }
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(core.committed_scene(), Some(&committed));
}

#[test]
fn committed_ancestor_consumption_is_not_retroactively_rerouted_to_growing_inner_content() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));

    render_nested_growth_frame(&mut core, &measurer, &mut consumer, 3, true);
    assert!(core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5)));
    render_nested_growth_frame(&mut core, &measurer, &mut consumer, 5, false);

    let scene = &consumer.scenes()[1];
    assert_eq!(
        scene
            .node(OUTER)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset
            .y,
        20.0
    );
    assert_eq!(
        scene
            .node(VIRTUAL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset
            .y,
        20.0
    );
}

#[test]
fn shrink_clamps_in_the_same_generation_and_empty_content_calls_nothing() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 45.0));
    let calls = Cell::new(0);

    for (count, offset) in [(10, Some(155.0)), (2, None), (0, None)] {
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                let mut spec = VirtualListSpec::new(VIRTUAL, count, 20.0, 45.0);
                if let Some(offset) = offset {
                    spec = spec.with_offset(offset);
                }
                ui.virtual_column(spec, DeclarationStyle::new().width(100.0), |ui, index| {
                    calls.set(calls.get() + 1);
                    ui.solid_rect(item_id(index), item_style(), Color::BLACK);
                })
                .unwrap();
            });
        })
        .unwrap();
    }

    let shrunk = &consumer.scenes()[1];
    let metrics = shrunk.node(VIRTUAL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.applied_offset.y, 0.0);
    assert_eq!(metrics.maximum_offset.y, 0.0);
    assert_eq!(
        shrunk.node(virtual_item_id(VIRTUAL, 0)).unwrap().bounds.y,
        0.0
    );
    assert_eq!(
        shrunk.node(virtual_item_id(VIRTUAL, 1)).unwrap().bounds.y,
        20.0
    );
    let before_empty = calls.get();
    let empty = &consumer.scenes()[2];
    assert_eq!(
        empty
            .node(VIRTUAL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset
            .y,
        0.0
    );
    assert_eq!(calls.get(), before_empty);
}

#[test]
fn nested_clip_and_transform_apply_to_every_visible_scene_product() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(200.0, 100.0));

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.column(
                OUTER,
                DeclarationStyle::new()
                    .size(100.0, 40.0)
                    .clip_children()
                    .transform(LogicalTransform::translate(20.0, 10.0)),
                |ui| {
                    ui.virtual_column(
                        VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(10.0),
                        DeclarationStyle::new().width(100.0),
                        |ui, index| {
                            ui.button(
                                item_id(index),
                                format!("Item {index}"),
                                ButtonStyle::default().layout(item_style()),
                            );
                        },
                    )
                    .unwrap();
                },
            );
        });
    })
    .unwrap();

    let scene = &consumer.scenes()[0];
    assert_eq!(
        scene.node(virtual_item_id(VIRTUAL, 0)).unwrap().bounds.y,
        -10.0
    );
    for index in 0..3 {
        let id = item_id(index);
        let node = scene.node(id).unwrap();
        assert_eq!(node.transformed_bounds.y, 10.0 + index as f32 * 20.0 - 10.0);
        assert_eq!(node.effective_clip, Some(rect(20.0, 10.0, 100.0, 40.0)));
        assert_eq!(
            scene
                .display_list
                .iter()
                .find(|record| record.id == id)
                .unwrap()
                .bounds,
            node.transformed_bounds
        );
        assert_eq!(
            scene
                .hit_index
                .iter()
                .find(|record| record.id == id)
                .unwrap()
                .bounds,
            node.transformed_bounds
        );
        assert_eq!(
            scene.semantics.node(id).unwrap().bounds,
            node.transformed_bounds
        );
        assert_eq!(
            scene
                .damage
                .iter()
                .find(|record| record.id == id)
                .unwrap()
                .current_bounds,
            node.transformed_bounds
        );
    }
}

#[test]
fn invalid_specs_fail_before_callbacks_or_commit() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let invalid = [
        (0.0, 40.0),
        (-1.0, 40.0),
        (f32::NAN, 40.0),
        (20.0, 0.0),
        (20.0, f32::INFINITY),
    ];
    for (item_height, viewport_height) in invalid {
        let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
        let mut consumer = NullSceneConsumer::default();
        let calls = Cell::new(0);
        let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            let _ = ui.virtual_column(
                VirtualListSpec::new(VIRTUAL, 10, item_height, viewport_height),
                DeclarationStyle::new(),
                |_, _| calls.set(calls.get() + 1),
            );
        })
        .unwrap_err();
        assert!(matches!(error, FrameError::InvalidVirtualContent { .. }));
        assert_eq!(calls.get(), 0);
        assert!(core.committed_scene().is_none());
        assert!(consumer.scenes().is_empty());
    }

    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut consumer = NullSceneConsumer::default();
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let _ = ui.virtual_column(
            VirtualListSpec::new(VIRTUAL, usize::MAX, f32::MAX, 40.0),
            DeclarationStyle::new(),
            |_, _| unreachable!(),
        );
    })
    .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidVirtualContent {
            id: VIRTUAL,
            error: VirtualDeclarationError::UnrepresentableContentExtent {
                item_count: usize::MAX,
                item_height: f32::MAX,
            },
        }
    );

    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let mut consumer = NullSceneConsumer::default();
    let calls = Cell::new(0);
    let item_height = f32::MAX / 10.0;
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        let _ = ui.virtual_column(
            VirtualListSpec::new(VIRTUAL, 10, item_height, 40.0).scroll_to(9),
            DeclarationStyle::new().width(100.0),
            |_, _| calls.set(calls.get() + 1),
        );
    })
    .unwrap_err();
    assert_eq!(
        error,
        FrameError::InvalidVirtualContent {
            id: VIRTUAL,
            error: VirtualDeclarationError::UnrepresentableScrollGeometry {
                item_count: 10,
                item_height,
                viewport_height: 40.0,
            },
        }
    );
    assert_eq!(calls.get(), 0);
    assert!(core.committed_scene().is_none());
    assert!(consumer.scenes().is_empty());
}

#[test]
fn failed_shifted_generation_retries_callbacks_and_preserves_virtual_response_once() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    core.set_wheel_scroll_speed(40.0);

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.virtual_column(
                VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(0.0),
                DeclarationStyle::new().width(100.0),
                |ui, index| {
                    ui.button(
                        item_id(index),
                        format!("Item {index}"),
                        ButtonStyle::default().layout(item_style()),
                    );
                },
            )
            .unwrap();
        });
    })
    .unwrap();
    assert!(core.queue_pointer_event(
        esox_ui::frame_core::PointerEventKind::Press,
        7,
        LogicalPoint::new(10.0, 10.0),
    ));
    assert!(core.queue_pointer_event(
        esox_ui::frame_core::PointerEventKind::Release,
        7,
        LogicalPoint::new(10.0, 10.0),
    ));
    assert!(core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 1.0)));

    let failed_calls = RefCell::new(Vec::new());
    let duplicate = WidgetId(99_999);
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.virtual_column(
                VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0),
                DeclarationStyle::new().width(100.0),
                |ui, index| {
                    failed_calls.borrow_mut().push(index);
                    ui.solid_rect(duplicate, item_style(), Color::BLACK);
                },
            )
            .unwrap();
        });
    })
    .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(duplicate));
    assert_eq!(*failed_calls.borrow(), [2, 3]);

    let retry_calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.virtual_column(
                VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0),
                DeclarationStyle::new().width(100.0),
                |ui, index| {
                    retry_calls.borrow_mut().push(index);
                    ui.button(
                        item_id(index),
                        format!("Item {index}"),
                        ButtonStyle::default().layout(item_style()),
                    );
                },
            )
            .unwrap();
        });
    })
    .unwrap();
    assert_eq!(*retry_calls.borrow(), [2, 3]);

    let clicked = Cell::new(false);
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.virtual_column(
                VirtualListSpec::new(VIRTUAL, 5, 20.0, 40.0).with_offset(0.0),
                DeclarationStyle::new().width(100.0),
                |ui, index| {
                    let response = ui.button(
                        item_id(index),
                        format!("Item {index}"),
                        ButtonStyle::default().layout(item_style()),
                    );
                    if index == 0 {
                        clicked.set(response.clicked);
                    }
                },
            )
            .unwrap();
        });
    })
    .unwrap();
    assert!(clicked.get());
}

#[test]
fn application_collision_with_reserved_virtual_wrapper_id_is_atomic() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let calls = Cell::new(0);
    let collision = virtual_item_id(VIRTUAL, 0);
    let error = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.virtual_column(
            VirtualListSpec::new(VIRTUAL, 2, 20.0, 40.0),
            DeclarationStyle::new().width(100.0),
            |ui, index| {
                calls.set(calls.get() + 1);
                ui.solid_rect(
                    if index == 0 {
                        collision
                    } else {
                        item_id(index)
                    },
                    item_style(),
                    Color::BLACK,
                );
            },
        )
        .unwrap();
    })
    .unwrap_err();
    assert_eq!(error, FrameError::DuplicateWidgetId(collision));
    assert_eq!(calls.get(), 2);
    assert!(core.committed_scene().is_none());
    assert!(consumer.scenes().is_empty());
}

#[test]
fn large_representable_wrapper_positions_use_precise_logical_indices() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 2_048.0));
    let calls = RefCell::new(Vec::new());
    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        ui.virtual_column(
            VirtualListSpec::new(VIRTUAL, 1_000_000, 1_024.0, 2_048.0).with_offset(1_023_996_928.0),
            DeclarationStyle::new().width(100.0),
            |ui, index| {
                calls.borrow_mut().push(index);
                ui.solid_rect(
                    item_id(index),
                    DeclarationStyle::new().size(100.0, 20.0),
                    Color::BLACK,
                );
            },
        )
        .unwrap();
    })
    .unwrap();
    assert_eq!(*calls.borrow(), [999_997, 999_998]);
    let scene = &consumer.scenes()[0];
    assert_eq!(
        scene
            .node(virtual_item_id(VIRTUAL, 999_997))
            .unwrap()
            .bounds
            .y,
        0.0
    );
    assert_eq!(
        scene
            .node(virtual_item_id(VIRTUAL, 999_998))
            .unwrap()
            .bounds
            .y,
        1_024.0
    );
}
