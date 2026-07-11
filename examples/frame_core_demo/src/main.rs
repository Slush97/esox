use esox_gfx::{Color, Frame, GpuContext, Rect, RenderResources, ShapeBuilder};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::esox_input::{Key, KeyEvent, Modifiers, NamedKey};
use esox_platform::{AppDelegate, MouseInputEvent};
use esox_ui::frame_core::{
    Axis, CommittedScene, DeterministicMeasurer, Element, FrameCore, GridTrack, LogicalRect,
    LogicalSize, SceneConsumer, WidgetId,
};

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
    alternate: bool,
}

impl Showcase {
    fn new() -> Self {
        let viewport = LogicalSize::new(900.0, 600.0);
        Self {
            core: FrameCore::new(viewport),
            consumer: PresentSink::default(),
            viewport,
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

    fn draw_scene(frame: &mut Frame, scene: &CommittedScene) {
        for node in &scene.nodes {
            let Some(bounds) = node.paint_bounds else {
                continue;
            };
            let clip = node.effective_clip.map(gfx_rect);
            let radius = if node.id == ROOT { 0.0 } else { 14.0 };
            let shape = ShapeBuilder::rounded_rect(
                bounds.x + 6.0,
                bounds.y + 6.0,
                (bounds.width - 12.0).max(0.0),
                (bounds.height - 12.0).max(0.0),
                radius,
            )
            .color(node_color(node.id))
            .clip(clip)
            .build();
            frame.push(shape);

            if node.id == TEXT {
                draw_text_lines(frame, bounds, clip);
            } else if node.id == IMAGE {
                draw_image_mark(frame, bounds, clip);
            } else if node.id == HEADER {
                draw_header_mark(frame, bounds, clip);
            }

            if node.hit_bounds.is_some() && node.id != HEADER {
                frame.push(
                    ShapeBuilder::rounded_rect(
                        bounds.x + 8.0,
                        bounds.y + 8.0,
                        (bounds.width - 16.0).max(0.0),
                        (bounds.height - 16.0).max(0.0),
                        12.0,
                    )
                    .color(Color::from_srgb(226, 232, 240, 0.8))
                    .stroke(2.0)
                    .clip(clip)
                    .build(),
                );
            }
        }
    }
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
        Self::draw_scene(frame, scene);
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

    fn on_resize(&mut self, width: u32, height: u32, _gpu: &GpuContext) {
        self.viewport = LogicalSize::new(width as f32, height as f32);
        self.core.resize(self.viewport);
    }

    fn on_mouse(&mut self, event: MouseInputEvent) {
        if matches!(event, MouseInputEvent::Press { button: 0, .. }) {
            self.alternate = !self.alternate;
        }
    }

    fn on_scale_changed(&mut self, _scale_factor: f64, _gpu: &GpuContext) {}
    fn on_paste(&mut self, _text: &str) {}
    fn on_ime_commit(&mut self, _text: &str) {}
    fn on_copy(&mut self) -> Option<String> {
        None
    }

    fn cursor_icon(&self, _x: f64, _y: f64) -> esox_platform::esox_input::CursorIcon {
        esox_platform::esox_input::CursorIcon::Pointer
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

fn draw_text_lines(frame: &mut Frame, bounds: LogicalRect, clip: Option<Rect>) {
    let available = (bounds.width - 48.0).max(0.0);
    for (index, fraction) in [1.0, 0.82, 0.93, 0.68].into_iter().enumerate() {
        frame.push(
            ShapeBuilder::rounded_rect(
                bounds.x + 24.0,
                bounds.y + 30.0 + index as f32 * 25.0,
                available * fraction,
                9.0,
                4.5,
            )
            .color(Color::from_srgb(220, 252, 231, 0.9))
            .clip(clip)
            .build(),
        );
    }
}

fn draw_image_mark(frame: &mut Frame, bounds: LogicalRect, clip: Option<Rect>) {
    let radius = bounds.width.min(bounds.height) * 0.18;
    frame.push(
        ShapeBuilder::circle(
            bounds.x + bounds.width * 0.38,
            bounds.y + bounds.height * 0.42,
            radius,
        )
        .color(Color::from_srgb(254, 215, 170, 1.0))
        .clip(clip)
        .build(),
    );
    frame.push(
        ShapeBuilder::triangle(
            bounds.x + bounds.width * 0.18,
            bounds.y + bounds.height * 0.55,
            bounds.width * 0.64,
            bounds.height * 0.3,
        )
        .color(Color::from_srgb(124, 45, 18, 0.75))
        .clip(clip)
        .build(),
    );
}

fn draw_header_mark(frame: &mut Frame, bounds: LogicalRect, clip: Option<Rect>) {
    for index in 0..3 {
        frame.push(
            ShapeBuilder::rounded_rect(
                bounds.x + 28.0 + index as f32 * 24.0,
                bounds.y + bounds.height * 0.5 - 7.0,
                14.0,
                14.0,
                7.0,
            )
            .color(Color::from_srgb(224, 231, 255, 0.95))
            .clip(clip)
            .build(),
        );
    }
    frame.push(
        ShapeBuilder::rounded_rect(
            bounds.x + 120.0,
            bounds.y + bounds.height * 0.5 - 6.0,
            (bounds.width - 170.0).max(0.0),
            12.0,
            6.0,
        )
        .color(Color::from_srgb(199, 210, 254, 0.8))
        .clip(clip)
        .build(),
    );
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
