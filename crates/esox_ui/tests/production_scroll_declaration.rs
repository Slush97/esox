use std::cell::Cell;

use esox_ui::declaration::{run_declaration_frame, ButtonStyle, DeclarationStyle};
use esox_ui::frame_core::{
    Color, DeterministicMeasurer, FrameCore, LogicalPoint, LogicalRect, LogicalSize,
    NullSceneConsumer, ScrollMetrics, WidgetId,
};

const ROOT: WidgetId = WidgetId(4_000);
const SCROLL: WidgetId = WidgetId(4_001);
const ROW_A: WidgetId = WidgetId(4_002);
const ROW_B: WidgetId = WidgetId(4_003);
const INNER_SCROLL: WidgetId = WidgetId(4_004);
const ROW_C: WidgetId = WidgetId(4_005);

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn row_style() -> ButtonStyle {
    ButtonStyle::default().layout(
        DeclarationStyle::new()
            .size(100.0, 30.0)
            .min_size(Some(100.0), Some(30.0)),
    )
}

#[test]
fn production_scroll_offset_is_current_and_clips_every_scene_product() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));
    let declarations = Cell::new(0);

    for offset_y in [5.0, 25.0] {
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            declarations.set(declarations.get() + 1);
            ui.column(ROOT, DeclarationStyle::new(), |ui| {
                ui.column(
                    SCROLL,
                    DeclarationStyle::new()
                        .size(100.0, 40.0)
                        .clip_children()
                        .scroll_offset(0.0, offset_y),
                    |ui| {
                        ui.button(ROW_A, "Row A", row_style());
                        ui.button(ROW_B, "Row B", row_style());
                    },
                );
            });
        })
        .unwrap();
    }

    assert_eq!(declarations.get(), 2);
    let first = &consumer.scenes()[0];
    let second = &consumer.scenes()[1];
    assert_eq!(
        first.node(ROW_B).unwrap().bounds,
        rect(0.0, 25.0, 100.0, 30.0)
    );
    assert_eq!(
        second.node(ROW_A).unwrap().bounds,
        rect(0.0, -24.0, 100.0, 30.0)
    );
    assert_eq!(
        second.node(ROW_B).unwrap().bounds,
        rect(0.0, 6.0, 100.0, 30.0)
    );
    assert_eq!(
        second.node(SCROLL).unwrap().scroll_metrics,
        Some(ScrollMetrics {
            viewport_extent: LogicalSize::new(100.0, 40.0),
            content_extent: LogicalSize::new(100.0, 64.0),
            requested_offset: LogicalPoint::new(0.0, 25.0),
            applied_offset: LogicalPoint::new(0.0, 24.0),
            maximum_offset: LogicalPoint::new(0.0, 24.0),
        })
    );

    let row = second.node(ROW_B).unwrap();
    let clip = Some(rect(0.0, 0.0, 100.0, 40.0));
    assert_eq!(row.effective_clip, clip);
    assert_eq!(row.paint_bounds, Some(row.bounds));
    assert_eq!(row.hit_bounds, Some(row.bounds));
    assert_eq!(row.semantic_bounds, Some(row.bounds));
    assert_eq!(row.current_damage_bounds, Some(row.bounds));
    assert_eq!(
        second
            .display_list
            .iter()
            .find(|record| record.id == ROW_B)
            .unwrap()
            .effective_clip,
        clip
    );
    assert_eq!(
        second
            .hit_index
            .iter()
            .find(|record| record.id == ROW_B)
            .unwrap()
            .effective_clip,
        clip
    );
    assert_eq!(second.semantics.node(ROW_B).unwrap().effective_clip, clip);
    assert_eq!(
        second
            .damage
            .iter()
            .find(|record| record.id == ROW_B)
            .unwrap()
            .effective_clip,
        clip
    );
    assert_eq!(
        second
            .hit_test(LogicalPoint::new(10.0, 10.0))
            .map(|node| node.id),
        Some(ROW_B)
    );
    assert_eq!(second.hit_test(LogicalPoint::new(10.0, 45.0)), None);
}

#[test]
fn production_content_shrink_clamps_offset_in_the_shrink_generation() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let declarations = Cell::new(0);

    for rows in [3, 1] {
        run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            declarations.set(declarations.get() + 1);
            ui.column(ROOT, DeclarationStyle::new().size(100.0, 40.0), |ui| {
                ui.column(
                    SCROLL,
                    DeclarationStyle::new()
                        .size(100.0, 40.0)
                        .clip_children()
                        .scroll_offset(0.0, 50.0),
                    |ui| {
                        for id in [ROW_A, ROW_B, ROW_C].into_iter().take(rows) {
                            ui.solid_rect(
                                id,
                                DeclarationStyle::new()
                                    .size(100.0, 30.0)
                                    .min_size(Some(100.0), Some(30.0)),
                                Color::rgba(0.2, 0.4, 0.6, 1.0),
                            );
                        }
                    },
                );
            });
        })
        .unwrap();
    }

    assert_eq!(declarations.get(), 2);
    assert_eq!(consumer.scenes().len(), 2);
    let grown = &consumer.scenes()[0];
    assert_eq!(
        grown
            .node(SCROLL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset,
        LogicalPoint::new(0.0, 50.0)
    );
    assert_eq!(grown.node(ROW_C).unwrap().bounds.y, 10.0);

    let shrunk = &consumer.scenes()[1];
    let metrics = shrunk.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.viewport_extent, LogicalSize::new(100.0, 40.0));
    assert_eq!(metrics.content_extent, LogicalSize::new(100.0, 40.0));
    assert_eq!(metrics.requested_offset, LogicalPoint::new(0.0, 50.0));
    assert_eq!(metrics.maximum_offset, LogicalPoint::default());
    assert_eq!(metrics.applied_offset, LogicalPoint::default());
    let row = shrunk.node(ROW_A).unwrap();
    assert_eq!(row.bounds, rect(0.0, 0.0, 100.0, 30.0));
    assert_eq!(row.paint_bounds, Some(row.bounds));
    assert_eq!(row.current_damage_bounds, Some(row.bounds));
}

#[test]
fn nested_offsets_accumulate_and_resize_recomputes_the_current_clip() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 60.0));

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>| {
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            ui.column(
                SCROLL,
                DeclarationStyle::new()
                    .size(100.0, 40.0)
                    .clip_children()
                    .scroll_offset(20.0, 10.0),
                |ui| {
                    ui.column(
                        INNER_SCROLL,
                        DeclarationStyle::new()
                            .size(120.0, 30.0)
                            .min_size(Some(120.0), Some(30.0))
                            .clip_children()
                            .scroll_offset(5.0, 5.0),
                        |ui| {
                            ui.button(
                                ROW_A,
                                "Wide row",
                                ButtonStyle::default().layout(
                                    DeclarationStyle::new()
                                        .size(140.0, 50.0)
                                        .min_size(Some(140.0), Some(50.0)),
                                ),
                            );
                        },
                    );
                },
            );
        });
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();
    core.resize(LogicalSize::new(70.0, 60.0));
    run_declaration_frame(&mut core, &measurer, &mut consumer, declare).unwrap();

    let first = &consumer.scenes()[0];
    let second = &consumer.scenes()[1];
    assert_eq!(
        first.node(ROW_A).unwrap().bounds,
        rect(-25.0, -5.0, 140.0, 50.0)
    );
    assert_eq!(
        first.node(ROW_A).unwrap().effective_clip,
        Some(rect(0.0, 0.0, 100.0, 30.0))
    );
    assert_eq!(
        second.node(ROW_A).unwrap().bounds,
        rect(-25.0, -5.0, 140.0, 50.0)
    );
    assert_eq!(
        second.node(ROW_A).unwrap().effective_clip,
        Some(rect(0.0, 0.0, 70.0, 30.0))
    );
    assert_eq!(
        first.node(SCROLL).unwrap().scroll_metrics,
        Some(ScrollMetrics {
            viewport_extent: LogicalSize::new(100.0, 40.0),
            content_extent: LogicalSize::new(120.0, 40.0),
            requested_offset: LogicalPoint::new(20.0, 10.0),
            applied_offset: LogicalPoint::new(20.0, 0.0),
            maximum_offset: LogicalPoint::new(20.0, 0.0),
        })
    );
    assert_eq!(
        first.node(INNER_SCROLL).unwrap().scroll_metrics,
        Some(ScrollMetrics {
            viewport_extent: LogicalSize::new(120.0, 30.0),
            content_extent: LogicalSize::new(140.0, 50.0),
            requested_offset: LogicalPoint::new(5.0, 5.0),
            applied_offset: LogicalPoint::new(5.0, 5.0),
            maximum_offset: LogicalPoint::new(20.0, 20.0),
        })
    );
    assert!(first.hit_test(LogicalPoint::new(75.0, 10.0)).is_some());
    assert_eq!(second.hit_test(LogicalPoint::new(75.0, 10.0)), None);
}

#[test]
fn production_retained_scroll_uses_wheel_input_and_explicit_requests_win() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(100.0, 40.0));
    let declarations = Cell::new(0);

    let declare = |ui: &mut esox_ui::declaration::DeclarationUi<'_>, explicit| {
        declarations.set(declarations.get() + 1);
        ui.column(ROOT, DeclarationStyle::new(), |ui| {
            let style = DeclarationStyle::new().size(100.0, 40.0).clip_children();
            let style = match explicit {
                Some(offset) => style.scroll_offset(0.0, offset),
                None => style.scrollable(),
            };
            ui.column(SCROLL, style, |ui| {
                ui.solid_rect(
                    ROW_A,
                    DeclarationStyle::new()
                        .size(100.0, 100.0)
                        .min_size(Some(100.0), Some(100.0)),
                    Color::rgba(0.2, 0.4, 0.6, 1.0),
                );
            });
        });
    };

    run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare(ui, Some(0.0));
    })
    .unwrap();
    core.queue_wheel(LogicalPoint::new(10.0, 10.0), LogicalPoint::new(0.0, 0.5));
    let wheel_scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare(ui, None);
    })
    .unwrap()
    .clone();
    assert_eq!(
        wheel_scene
            .node(SCROLL)
            .unwrap()
            .scroll_metrics
            .unwrap()
            .applied_offset,
        LogicalPoint::new(0.0, 20.0)
    );
    assert_eq!(wheel_scene.node(ROW_A).unwrap().bounds.y, -20.0);

    let controlled = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
        declare(ui, Some(5.0));
    })
    .unwrap();
    let metrics = controlled.node(SCROLL).unwrap().scroll_metrics.unwrap();
    assert_eq!(metrics.requested_offset, LogicalPoint::new(0.0, 5.0));
    assert_eq!(metrics.applied_offset, LogicalPoint::new(0.0, 5.0));
    assert_eq!(
        core.scroll_offset(SCROLL).unwrap().applied,
        metrics.applied_offset
    );
    assert_eq!(declarations.get(), 3);
    assert_eq!(consumer.scenes().len(), 3);
}
