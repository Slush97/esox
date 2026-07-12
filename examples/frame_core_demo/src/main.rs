use esox_gfx::{Color, Frame, GpuContext, Rect, RenderResources, ShapeBuilder};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::esox_input::{
    CursorIcon, Key, KeyEvent, LogicalPosition, LogicalViewport, Modifiers, NamedKey, PointerEvent,
    PointerPhase,
};
use esox_platform::{AppDelegate, MouseInputEvent};
use esox_ui::frame_core::{
    Axis, CommittedScene, DeterministicMeasurer, Element, FrameCore, GridTrack, LogicalRect,
    LogicalSize, SceneConsumer, WidgetId,
};
use esox_ui::frame_scene_consumer::RendererScale;

const ROOT: WidgetId = WidgetId(1);
const HEADER: WidgetId = WidgetId(2);
const BODY: WidgetId = WidgetId(3);
const SIDEBAR: WidgetId = WidgetId(4);
const CONTENT: WidgetId = WidgetId(5);
const GRID: WidgetId = WidgetId(6);
const IMAGE: WidgetId = WidgetId(7);
const TEXT: WidgetId = WidgetId(8);
const BANNER: WidgetId = WidgetId(9);
const CARD: WidgetId = WidgetId(10);
const DECLARATIONS: WidgetId = WidgetId(100);

#[derive(Default)]
struct PresentSink {
    last_generation: u64,
}

impl SceneConsumer for PresentSink {
    fn consume(&mut self, scene: &CommittedScene) {
        self.last_generation = scene.generation;
    }
}

struct Showcase {
    core: FrameCore,
    consumer: PresentSink,
    viewport: LogicalSize,
    renderer_scale: RendererScale,
    alternate: bool,
}

impl Showcase {
    fn new() -> Self {
        let viewport = LogicalSize::new(900.0, 600.0);
        Self {
            core: FrameCore::new(viewport),
            consumer: PresentSink::default(),
            viewport,
            renderer_scale: RendererScale::one(),
            alternate: false,
        }
    }

    fn declare(viewport: LogicalSize, alternate: bool) -> Element {
        let header_height = 88.0_f32.min(viewport.height);
        let body_height = (viewport.height - header_height - 18.0).max(0.0);
        let sidebar_width = 190.0_f32.min(viewport.width * 0.32);

        let body = if alternate {
            Element::flex(BODY, Axis::Row, 18.0)
                .with_flex_grow(1.0)
                .clip_children()
                .with_children(vec![
                    Element::flex(CONTENT, Axis::Column, 18.0)
                        .with_flex_grow(1.0)
                        .clip_children()
                        .with_children(vec![
                            Element::fixed(BANNER, viewport.width, 105.0).interactive(),
                            Element::text(
                                TEXT,
                                "Structural updates use only this generation's resolved geometry",
                            )
                            .interactive()
                            .with_flex_grow(1.0),
                        ]),
                    Element::fixed(SIDEBAR, sidebar_width, body_height).interactive(),
                    Element::fixed(CARD, 150.0, body_height).interactive(),
                ])
        } else {
            Element::flex(BODY, Axis::Row, 18.0)
                .with_flex_grow(1.0)
                .clip_children()
                .with_children(vec![
                    Element::fixed(SIDEBAR, sidebar_width, body_height).interactive(),
                    Element::flex(CONTENT, Axis::Column, 18.0)
                        .with_flex_grow(1.0)
                        .clip_children()
                        .with_children(vec![Element::grid(
                            GRID,
                            vec![GridTrack::Fraction(1.0), GridTrack::Fraction(1.4)],
                            18.0,
                        )
                        .with_flex_grow(1.0)
                        .with_children(vec![
                            Element::image(IMAGE, 1),
                            Element::text(
                                TEXT,
                                "Resize the window: text, layout, clips, hit bounds, and paint all update in the first frame",
                            )
                            .interactive(),
                        ])]),
                ])
        };

        Element::flex(ROOT, Axis::Column, 18.0).with_children(vec![
            Element::fixed(HEADER, viewport.width, header_height).interactive(),
            body,
        ])
    }

    fn update_renderer_scale(&mut self, scale_factor: f64) -> bool {
        let Some(scale) = RendererScale::new(scale_factor) else {
            return false;
        };
        self.renderer_scale = scale;
        true
    }

    fn update_logical_viewport(&mut self, viewport: LogicalViewport) -> bool {
        if !self.core.resize_logical_viewport(viewport) {
            return false;
        }
        self.viewport = LogicalSize::new(viewport.width, viewport.height);
        true
    }

    fn draw_scene(frame: &mut Frame, scene: &CommittedScene, scale: RendererScale) -> bool {
        let Some(shapes) = prepare_scene(scene, scale) else {
            return false;
        };
        for shape in shapes {
            shape.submit(frame);
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DemoShape {
    RoundedRect {
        bounds: LogicalRect,
        radius: f32,
        color: Color,
        stroke_width: Option<f32>,
        clip: Option<LogicalRect>,
    },
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        color: Color,
        clip: Option<LogicalRect>,
    },
    Triangle {
        bounds: LogicalRect,
        color: Color,
        clip: Option<LogicalRect>,
    },
}

impl DemoShape {
    fn scaled(self, scale: RendererScale) -> Option<Self> {
        let scalar = |value: f32| {
            let value = value * scale.get();
            value.is_finite().then_some(value)
        };
        let rect = |bounds: LogicalRect| {
            Some(LogicalRect {
                x: scalar(bounds.x)?,
                y: scalar(bounds.y)?,
                width: scalar(bounds.width)?,
                height: scalar(bounds.height)?,
            })
        };
        let clip = |value: Option<LogicalRect>| match value {
            Some(value) => Some(Some(rect(value)?)),
            None => Some(None),
        };

        match self {
            Self::RoundedRect {
                bounds,
                radius,
                color,
                stroke_width,
                clip: shape_clip,
            } => Some(Self::RoundedRect {
                bounds: rect(bounds)?,
                radius: scalar(radius)?,
                color,
                stroke_width: match stroke_width {
                    Some(width) => Some(scalar(width)?),
                    None => None,
                },
                clip: clip(shape_clip)?,
            }),
            Self::Circle {
                x,
                y,
                radius,
                color,
                clip: shape_clip,
            } => Some(Self::Circle {
                x: scalar(x)?,
                y: scalar(y)?,
                radius: scalar(radius)?,
                color,
                clip: clip(shape_clip)?,
            }),
            Self::Triangle {
                bounds,
                color,
                clip: shape_clip,
            } => Some(Self::Triangle {
                bounds: rect(bounds)?,
                color,
                clip: clip(shape_clip)?,
            }),
        }
    }

    fn submit(self, frame: &mut Frame) {
        match self {
            Self::RoundedRect {
                bounds,
                radius,
                color,
                stroke_width,
                clip,
            } => {
                let mut shape = ShapeBuilder::rounded_rect(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    radius,
                )
                .color(color)
                .clip(clip.map(gfx_rect));
                if let Some(width) = stroke_width {
                    shape = shape.stroke(width);
                }
                frame.push(shape.build());
            }
            Self::Circle {
                x,
                y,
                radius,
                color,
                clip,
            } => frame.push(
                ShapeBuilder::circle(x, y, radius)
                    .color(color)
                    .clip(clip.map(gfx_rect))
                    .build(),
            ),
            Self::Triangle {
                bounds,
                color,
                clip,
            } => frame.push(
                ShapeBuilder::triangle(bounds.x, bounds.y, bounds.width, bounds.height)
                    .color(color)
                    .clip(clip.map(gfx_rect))
                    .build(),
            ),
        }
    }
}

fn prepare_scene(scene: &CommittedScene, scale: RendererScale) -> Option<Vec<DemoShape>> {
    logical_scene_shapes(scene)
        .into_iter()
        .map(|shape| shape.scaled(scale))
        .collect()
}

impl AppDelegate for Showcase {
    fn on_init(&mut self, _gpu: &GpuContext, _resources: &mut RenderResources) {}

    fn on_redraw(
        &mut self,
        _gpu: &GpuContext,
        _resources: &mut RenderResources,
        frame: &mut Frame,
        _perf: &esox_platform::perf::PerfMonitor,
    ) {
        let viewport = self.viewport;
        let alternate = self.alternate;
        let glyph_width = if alternate { 10.0 } else { 8.0 };
        let measurer = DeterministicMeasurer::new(glyph_width, 22.0)
            .with_image(1, LogicalSize::new(280.0, 260.0));
        let scene = self
            .core
            .run_frame(&measurer, &mut self.consumer, |state| {
                let declarations = state.get(DECLARATIONS).unwrap_or(0) + 1;
                state.insert(DECLARATIONS, declarations);
                Self::declare(viewport, alternate)
            })
            .expect("the demo declaration has unique widget IDs");
        assert!(
            Self::draw_scene(frame, scene, self.renderer_scale),
            "the committed demo scene fits the validated renderer transform"
        );
    }

    fn on_key(&mut self, event: &KeyEvent, _modifiers: Modifiers) {
        if event.pressed
            && matches!(
                event.key,
                Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter)
            )
        {
            self.alternate = !self.alternate;
        }
    }

    fn on_resize(&mut self, _width: u32, _height: u32, _gpu: &GpuContext) {}

    fn on_mouse(&mut self, _event: MouseInputEvent) {}

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        match event.phase {
            PointerPhase::Move
            | PointerPhase::Press { button: 0 }
            | PointerPhase::Release { button: 0 } => {
                let accepted = self.core.queue_pointer_input(0, event);
                if accepted && matches!(event.phase, PointerPhase::Press { button: 0 }) {
                    self.alternate = !self.alternate;
                }
                accepted
            }
            PointerPhase::Press { .. } | PointerPhase::Release { .. } => true,
        }
    }

    fn on_scale_changed(&mut self, scale_factor: f64, _gpu: &GpuContext) {
        let _ = self.update_renderer_scale(scale_factor);
    }

    fn on_logical_viewport_changed(&mut self, viewport: LogicalViewport) {
        let _ = self.update_logical_viewport(viewport);
    }
    fn on_paste(&mut self, _text: &str) {}
    fn on_ime_commit(&mut self, _text: &str) {}
    fn on_copy(&mut self) -> Option<String> {
        None
    }

    fn cursor_icon(&self, _x: f64, _y: f64) -> CursorIcon {
        CursorIcon::Pointer
    }

    fn logical_cursor_icon(&self, position: LogicalPosition) -> Option<CursorIcon> {
        let point = esox_ui::frame_core::LogicalPoint::new(position.x, position.y);
        Some(
            if self
                .core
                .committed_scene()
                .and_then(|scene| scene.hit_test(point))
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            },
        )
    }
}

fn gfx_rect(rect: LogicalRect) -> Rect {
    Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn node_color(id: WidgetId) -> Color {
    match id {
        ROOT => Color::from_srgb(15, 23, 42, 1.0),
        HEADER => Color::from_srgb(79, 70, 229, 1.0),
        BODY => Color::from_srgb(30, 41, 59, 1.0),
        SIDEBAR => Color::from_srgb(14, 116, 144, 1.0),
        CONTENT => Color::from_srgb(51, 65, 85, 1.0),
        GRID => Color::from_srgb(71, 85, 105, 1.0),
        IMAGE => Color::from_srgb(234, 88, 12, 1.0),
        TEXT => Color::from_srgb(22, 163, 74, 1.0),
        BANNER => Color::from_srgb(190, 24, 93, 1.0),
        CARD => Color::from_srgb(124, 58, 237, 1.0),
        _ => Color::WHITE,
    }
}

fn logical_scene_shapes(scene: &CommittedScene) -> Vec<DemoShape> {
    let mut shapes = Vec::new();
    for node in &scene.nodes {
        let Some(bounds) = node.paint_bounds else {
            continue;
        };
        let clip = node.effective_clip;
        let radius = if node.id == ROOT { 0.0 } else { 14.0 };
        shapes.push(DemoShape::RoundedRect {
            bounds: LogicalRect {
                x: bounds.x + 6.0,
                y: bounds.y + 6.0,
                width: (bounds.width - 12.0).max(0.0),
                height: (bounds.height - 12.0).max(0.0),
            },
            radius,
            color: node_color(node.id),
            stroke_width: None,
            clip,
        });

        if node.id == TEXT {
            draw_text_lines(&mut shapes, bounds, clip);
        } else if node.id == IMAGE {
            draw_image_mark(&mut shapes, bounds, clip);
        } else if node.id == HEADER {
            draw_header_mark(&mut shapes, bounds, clip);
        }

        if node.hit_bounds.is_some() && node.id != HEADER {
            shapes.push(DemoShape::RoundedRect {
                bounds: LogicalRect {
                    x: bounds.x + 8.0,
                    y: bounds.y + 8.0,
                    width: (bounds.width - 16.0).max(0.0),
                    height: (bounds.height - 16.0).max(0.0),
                },
                radius: 12.0,
                color: Color::from_srgb(226, 232, 240, 0.8),
                stroke_width: Some(2.0),
                clip,
            });
        }
    }
    shapes
}

fn draw_text_lines(shapes: &mut Vec<DemoShape>, bounds: LogicalRect, clip: Option<LogicalRect>) {
    let available = (bounds.width - 48.0).max(0.0);
    for (index, fraction) in [1.0, 0.82, 0.93, 0.68].into_iter().enumerate() {
        shapes.push(DemoShape::RoundedRect {
            bounds: LogicalRect {
                x: bounds.x + 24.0,
                y: bounds.y + 30.0 + index as f32 * 25.0,
                width: available * fraction,
                height: 9.0,
            },
            radius: 4.5,
            color: Color::from_srgb(220, 252, 231, 0.9),
            stroke_width: None,
            clip,
        });
    }
}

fn draw_image_mark(shapes: &mut Vec<DemoShape>, bounds: LogicalRect, clip: Option<LogicalRect>) {
    let radius = bounds.width.min(bounds.height) * 0.18;
    shapes.push(DemoShape::Circle {
        x: bounds.x + bounds.width * 0.38,
        y: bounds.y + bounds.height * 0.42,
        radius,
        color: Color::from_srgb(254, 215, 170, 1.0),
        clip,
    });
    shapes.push(DemoShape::Triangle {
        bounds: LogicalRect {
            x: bounds.x + bounds.width * 0.18,
            y: bounds.y + bounds.height * 0.55,
            width: bounds.width * 0.64,
            height: bounds.height * 0.3,
        },
        color: Color::from_srgb(124, 45, 18, 0.75),
        clip,
    });
}

fn draw_header_mark(shapes: &mut Vec<DemoShape>, bounds: LogicalRect, clip: Option<LogicalRect>) {
    for index in 0..3 {
        shapes.push(DemoShape::RoundedRect {
            bounds: LogicalRect {
                x: bounds.x + 28.0 + index as f32 * 24.0,
                y: bounds.y + bounds.height * 0.5 - 7.0,
                width: 14.0,
                height: 14.0,
            },
            radius: 7.0,
            color: Color::from_srgb(224, 231, 255, 0.95),
            stroke_width: None,
            clip,
        });
    }
    shapes.push(DemoShape::RoundedRect {
        bounds: LogicalRect {
            x: bounds.x + 120.0,
            y: bounds.y + bounds.height * 0.5 - 6.0,
            width: (bounds.width - 170.0).max(0.0),
            height: 12.0,
        },
        radius: 6.0,
        color: Color::from_srgb(199, 210, 254, 0.8),
        stroke_width: None,
        clip,
    });
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = PlatformConfig {
        window: WindowConfig {
            title: "FrameCore showcase — click or press Space to change the tree".into(),
            width: Some(900),
            height: Some(600),
            ..Default::default()
        },
        background: "#0f172a".into(),
        ..Default::default()
    };
    esox_platform::run(config, Box::new(Showcase::new())).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use esox_platform::{PhysicalPosition, PhysicalViewport, WindowCoordinateTransform};
    use esox_ui::frame_core::InputResponse;

    fn commit(
        showcase: &mut Showcase,
        response_target: Option<WidgetId>,
    ) -> (CommittedScene, Option<InputResponse>) {
        let viewport = showcase.viewport;
        let alternate = showcase.alternate;
        let measurer =
            DeterministicMeasurer::new(8.0, 22.0).with_image(1, LogicalSize::new(280.0, 260.0));
        let mut response = None;
        let scene = showcase
            .core
            .run_frame(&measurer, &mut showcase.consumer, |state| {
                if let Some(target) = response_target {
                    response = state.take_response(target);
                }
                Showcase::declare(viewport, alternate)
            })
            .unwrap()
            .clone();
        (scene, response)
    }

    fn assert_rect_scaled(logical: LogicalRect, physical: LogicalRect, scale: f32) {
        assert_eq!(physical.x, logical.x * scale);
        assert_eq!(physical.y, logical.y * scale);
        assert_eq!(physical.width, logical.width * scale);
        assert_eq!(physical.height, logical.height * scale);
    }

    fn assert_shape_scaled(logical: DemoShape, physical: DemoShape, scale: f32) {
        match (logical, physical) {
            (
                DemoShape::RoundedRect {
                    bounds: logical_bounds,
                    radius: logical_radius,
                    stroke_width: logical_stroke,
                    clip: logical_clip,
                    ..
                },
                DemoShape::RoundedRect {
                    bounds: physical_bounds,
                    radius: physical_radius,
                    stroke_width: physical_stroke,
                    clip: physical_clip,
                    ..
                },
            ) => {
                assert_rect_scaled(logical_bounds, physical_bounds, scale);
                assert_eq!(physical_radius, logical_radius * scale);
                assert_eq!(physical_stroke, logical_stroke.map(|width| width * scale));
                match (logical_clip, physical_clip) {
                    (Some(logical), Some(physical)) => {
                        assert_rect_scaled(logical, physical, scale);
                    }
                    (None, None) => {}
                    clips => panic!("clip presence changed: {clips:?}"),
                }
            }
            (
                DemoShape::Circle {
                    x: logical_x,
                    y: logical_y,
                    radius: logical_radius,
                    clip: logical_clip,
                    ..
                },
                DemoShape::Circle {
                    x: physical_x,
                    y: physical_y,
                    radius: physical_radius,
                    clip: physical_clip,
                    ..
                },
            ) => {
                assert_eq!(physical_x, logical_x * scale);
                assert_eq!(physical_y, logical_y * scale);
                assert_eq!(physical_radius, logical_radius * scale);
                match (logical_clip, physical_clip) {
                    (Some(logical), Some(physical)) => {
                        assert_rect_scaled(logical, physical, scale);
                    }
                    (None, None) => {}
                    clips => panic!("clip presence changed: {clips:?}"),
                }
            }
            (
                DemoShape::Triangle {
                    bounds: logical_bounds,
                    clip: logical_clip,
                    ..
                },
                DemoShape::Triangle {
                    bounds: physical_bounds,
                    clip: physical_clip,
                    ..
                },
            ) => {
                assert_rect_scaled(logical_bounds, physical_bounds, scale);
                match (logical_clip, physical_clip) {
                    (Some(logical), Some(physical)) => {
                        assert_rect_scaled(logical, physical, scale);
                    }
                    (None, None) => {}
                    clips => panic!("clip presence changed: {clips:?}"),
                }
            }
            shapes => panic!("shape kind changed: {shapes:?}"),
        }
    }

    #[test]
    fn typed_pointer_reaches_frame_core_logically_at_common_scales() {
        for scale in [1.0, 2.0, 1.5] {
            let mut showcase = Showcase::new();
            commit(&mut showcase, None);
            let transform =
                WindowCoordinateTransform::new(PhysicalViewport::new(900, 600), scale).unwrap();
            let logical = LogicalPosition::new(40.0, 40.0);
            let position = transform
                .logical_position(PhysicalPosition::new(
                    f64::from(logical.x) * scale,
                    f64::from(logical.y) * scale,
                ))
                .unwrap();

            assert!(showcase.on_pointer(PointerEvent {
                phase: PointerPhase::Press { button: 0 },
                position,
            }));
            let (_, response) = commit(&mut showcase, Some(HEADER));
            let response = response.unwrap();

            assert_eq!(response.position.x, logical.x);
            assert_eq!(response.position.y, logical.y);
        }
    }

    #[test]
    fn logical_viewport_and_renderer_scale_apply_to_the_next_frame() {
        let mut showcase = Showcase::new();
        let transform =
            WindowCoordinateTransform::new(PhysicalViewport::new(1200, 750), 1.5).unwrap();
        assert!(showcase.update_logical_viewport(transform.logical_viewport().unwrap()));
        assert!(showcase.update_renderer_scale(1.5));

        let (scene, _) = commit(&mut showcase, None);
        assert_eq!(showcase.viewport, LogicalSize::new(800.0, 500.0));
        assert_eq!(scene.node(HEADER).unwrap().bounds.width, 800.0);
        let logical = logical_scene_shapes(&scene);
        let physical = prepare_scene(&scene, showcase.renderer_scale).unwrap();
        for (logical, physical) in logical.into_iter().zip(physical) {
            assert_shape_scaled(logical, physical, 1.5);
        }
    }

    #[test]
    fn initial_hidpi_submission_scales_once_without_mutating_the_scene() {
        let mut showcase = Showcase::new();
        assert!(showcase.update_renderer_scale(2.0));
        let (scene, _) = commit(&mut showcase, None);
        let committed = scene.clone();
        let logical = logical_scene_shapes(&scene);
        let physical = prepare_scene(&scene, showcase.renderer_scale).unwrap();

        assert_eq!(scene, committed);
        assert!(logical.iter().any(|shape| matches!(
            shape,
            DemoShape::RoundedRect {
                stroke_width: Some(2.0),
                ..
            }
        )));
        assert!(
            logical
                .iter()
                .any(|shape| matches!(shape, DemoShape::Circle { .. }))
        );
        assert!(
            logical
                .iter()
                .any(|shape| matches!(shape, DemoShape::Triangle { .. }))
        );
        for (logical, physical) in logical.into_iter().zip(physical) {
            assert_shape_scaled(logical, physical, 2.0);
        }
        assert_eq!(scene, committed);
    }

    #[test]
    fn invalid_updates_and_overflow_leave_valid_state_and_frame_untouched() {
        let mut showcase = Showcase::new();
        assert!(showcase.update_renderer_scale(2.0));
        assert!(showcase.update_logical_viewport(LogicalViewport::new(450.0, 300.0)));
        let valid_viewport = showcase.viewport;
        let valid_scale = showcase.renderer_scale;
        assert!(!showcase.update_renderer_scale(f64::NAN));
        assert!(!showcase.update_logical_viewport(LogicalViewport::new(f32::NAN, 300.0)));
        assert_eq!(showcase.viewport, valid_viewport);
        assert_eq!(showcase.renderer_scale, valid_scale);

        let (scene, _) = commit(&mut showcase, None);
        let alternate = showcase.alternate;
        assert!(!showcase.on_pointer(PointerEvent {
            phase: PointerPhase::Press { button: 0 },
            position: LogicalPosition::new(f32::NAN, 20.0),
        }));
        assert_eq!(showcase.alternate, alternate);
        let (_, response) = commit(&mut showcase, Some(HEADER));
        assert!(response.is_none());

        let mut frame = Frame::new();
        assert!(!Showcase::draw_scene(
            &mut frame,
            &scene,
            RendererScale::new(f64::from(f32::MAX)).unwrap(),
        ));
        assert_eq!(frame.instance_len(), 0);
    }

    #[test]
    fn logical_cursor_lookup_uses_committed_scene_coordinates() {
        let mut showcase = Showcase::new();
        commit(&mut showcase, None);

        assert_eq!(
            showcase.logical_cursor_icon(LogicalPosition::new(40.0, 40.0)),
            Some(CursorIcon::Pointer)
        );
        assert_eq!(
            showcase.logical_cursor_icon(LogicalPosition::new(2_000.0, 2_000.0)),
            Some(CursorIcon::Default)
        );
    }
}
