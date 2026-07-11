//! FrameCore Tonight — windowed showcase for the current-generation pipeline.
//!
//! Frames run through [`FrameCore`] and the production declaration facade, and
//! the committed display list is submitted to the renderer through
//! `frame_scene_consumer`. All inspector and hover data shown while declaring
//! comes from post-commit snapshots stored in [`DemoState`], never from
//! geometry reads inside the declaration closure.

use std::cell::RefCell;

use esox_gfx::{Frame, GpuContext, RenderResources};
use esox_platform::config::{PlatformConfig, WindowConfig};
use esox_platform::esox_input::{CursorIcon, Key, KeyEvent, Modifiers};
use esox_platform::{AppDelegate, MouseInputEvent};
use esox_ui::TextRenderer;
use esox_ui::declaration::DeclarationUi;
use esox_ui::frame_core::{
    CommittedScene, FrameCore, ImageMeasureRequest, IntrinsicMeasurer, LogicalPoint, LogicalSize,
    PointerEventKind, SceneConsumer, TextMeasureRequest, WidgetId,
};
use esox_ui::frame_scene_consumer::{TextRendererFramePaint, submit_display_list};
use frame_core_tonight::{
    CONTAINER_CLOSURES, DeclarationProbe, DemoState, SIBLING_CARD, ShowcaseFrame, TARGET_BUTTON,
    TARGET_CARD, declare_showcase, inspect, widget_label,
};

/// Synthetic pointer identity used by the stale-click script's capture.
const SCRIPT_POINTER: u64 = 7;

/// Multi-frame script driven from post-commit state, one stage per frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Script {
    Idle,
    /// A synthetic press for [`SCRIPT_POINTER`] is queued; the next declaration
    /// observes it and requests capture plus keyboard focus.
    AwaitPress,
    /// Capture is held; the stale click is queued and the target hides next
    /// frame, so the queued input must dispatch against the prior scene.
    Verify,
}

#[derive(Default)]
struct PresentSink {
    last_generation: u64,
}

impl SceneConsumer for PresentSink {
    fn consume(&mut self, scene: &CommittedScene) {
        self.last_generation = scene.generation;
    }
}

/// Measures text through the production font stack; no GPU work happens here.
struct SystemTextMeasurer<'a> {
    renderer: RefCell<&'a mut TextRenderer>,
}

impl IntrinsicMeasurer for SystemTextMeasurer<'_> {
    fn measure_text(&self, request: TextMeasureRequest<'_>) -> LogicalSize {
        let mut renderer = self.renderer.borrow_mut();
        let size = request.properties.font_size;
        let width = renderer.measure_text(request.content, size);
        let height = renderer.line_height(size);
        LogicalSize::new(
            request.known_dimensions.width.unwrap_or(width),
            request.known_dimensions.height.unwrap_or(height),
        )
    }

    fn measure_image(&self, request: ImageMeasureRequest) -> LogicalSize {
        LogicalSize::new(
            request.known_dimensions.width.unwrap_or(0.0),
            request.known_dimensions.height.unwrap_or(0.0),
        )
    }
}

struct Tonight {
    core: FrameCore,
    sink: PresentSink,
    text: Option<TextRenderer>,
    state: DemoState,
    probe: DeclarationProbe,
    script: Script,
    cursor: LogicalPoint,
    activations: u32,
    stale_key_pending: bool,
    prev_focus: Option<WidgetId>,
    prev_hidden: bool,
    prev_disabled: bool,
}

impl Tonight {
    fn new() -> Self {
        Self {
            core: FrameCore::new(LogicalSize::new(1280.0, 820.0)),
            sink: PresentSink::default(),
            text: None,
            state: DemoState::default(),
            probe: DeclarationProbe::default(),
            script: Script::Idle,
            cursor: LogicalPoint::new(-1.0, -1.0),
            activations: 0,
            stale_key_pending: false,
            prev_focus: None,
            prev_hidden: false,
            prev_disabled: false,
        }
    }

    fn log(&mut self, line: String) {
        self.state.log.push_back(line);
        while self.state.log.len() > 40 {
            self.state.log.pop_front();
        }
    }

    fn toggle_hidden(&mut self) {
        self.state.target_hidden = !self.state.target_hidden;
        self.state.flash = 1.0;
        let line = format!(
            "toggle: target {} next frame",
            if self.state.target_hidden {
                "hides"
            } else {
                "restores"
            }
        );
        self.log(line);
    }

    fn toggle_disabled(&mut self) {
        self.state.target_disabled = !self.state.target_disabled;
        self.state.flash = 1.0;
        let line = format!(
            "toggle: target {} next frame",
            if self.state.target_disabled {
                "disables"
            } else {
                "re-enables"
            }
        );
        self.log(line);
    }

    fn target_button_center(scene: &CommittedScene) -> Option<LogicalPoint> {
        scene.node(TARGET_BUTTON).map(|node| {
            LogicalPoint::new(
                node.bounds.x + node.bounds.width / 2.0,
                node.bounds.y + node.bounds.height / 2.0,
            )
        })
    }

    fn advance_script(&mut self, scene: &CommittedScene, out: ShowcaseFrame) {
        match self.script {
            Script::Idle => {
                let requested = out.stale_requested || std::mem::take(&mut self.stale_key_pending);
                if !requested {
                    return;
                }
                if self.state.target_hidden || self.state.target_disabled {
                    self.log("script: restore the target first, then queue again".to_owned());
                    return;
                }
                if let Some(center) = Self::target_button_center(scene) {
                    self.core
                        .queue_pointer_event(PointerEventKind::Press, SCRIPT_POINTER, center);
                    self.log(format!(
                        "script: press (ptr {SCRIPT_POINTER}) queued at {}, {} against gen {}",
                        center.x, center.y, scene.generation
                    ));
                    self.script = Script::AwaitPress;
                }
            }
            Script::AwaitPress => {
                let captured = self.core.pointer_capture(SCRIPT_POINTER) == Some(TARGET_BUTTON);
                self.log(if captured {
                    format!(
                        "script: capture(ptr {SCRIPT_POINTER}) granted to demo button, focus follows"
                    )
                } else {
                    "script: capture was not granted — aborting".to_owned()
                });
                if !captured {
                    self.script = Script::Idle;
                    return;
                }
                if let Some(center) = Self::target_button_center(scene) {
                    self.core
                        .queue_pointer_event(PointerEventKind::Press, 0, center);
                    self.core
                        .queue_pointer_event(PointerEventKind::Release, 0, center);
                }
                self.state.target_hidden = true;
                self.state.flash = 1.0;
                self.log(format!(
                    "script: stale click queued against gen {}; hiding target next frame",
                    scene.generation
                ));
                self.script = Script::Verify;
            }
            Script::Verify => {
                self.log(if out.target_clicked {
                    "script: !! stale click ACTIVATED — contract broken".to_owned()
                } else {
                    "script: stale click discarded — hidden target never activated".to_owned()
                });
                self.script = Script::Idle;
            }
        }
    }

    fn after_commit(&mut self, scene: &CommittedScene, out: ShowcaseFrame) {
        self.state.generation_label = format!("gen {}", scene.generation);
        self.state.target_pressed = out.target_pressed;

        if out.toggle_hidden {
            self.toggle_hidden();
        }
        if out.toggle_disabled {
            self.toggle_disabled();
        }
        if let Some(target) = out.select
            && target != self.state.selected
        {
            self.state.selected = target;
            self.log(format!("inspector: subject -> {}", widget_label(target)));
        }
        if out.target_clicked {
            self.activations += 1;
            self.state.activation_label = format!("activations: {}", self.activations);
            self.log(format!("demo action activated (gen {})", scene.generation));
        }

        self.advance_script(scene, out);

        while let Some(cancel) = self.core.take_cancellation() {
            self.log(format!(
                "cancel: pointer {} on {} — capture owner left the scene (gen {})",
                cancel.pointer,
                widget_label(cancel.target),
                cancel.committed_generation
            ));
        }

        let focus = self.core.keyboard_focus();
        if focus != self.prev_focus {
            let name = |id: Option<WidgetId>| id.map_or("—", widget_label);
            self.log(format!(
                "focus: {} -> {}",
                name(self.prev_focus),
                name(focus)
            ));
            self.prev_focus = focus;
        }
        self.state.focus_label = format!("focus: {}", focus.map_or("—", widget_label));
        self.state.capture_label = format!(
            "capture: {}",
            self.core
                .pointer_capture(SCRIPT_POINTER)
                .map_or("—", widget_label)
        );

        let hidden = scene
            .node(TARGET_CARD)
            .is_some_and(|node| node.effective_hidden);
        if hidden != self.prev_hidden {
            if let Some(sibling) = scene.node(SIBLING_CARD) {
                self.log(if hidden {
                    format!(
                        "gen {}: target collapsed — sibling reflowed to x {}, width {}",
                        scene.generation, sibling.bounds.x, sibling.bounds.width
                    )
                } else {
                    format!(
                        "gen {}: target restored in its first frame — sibling back to x {}",
                        scene.generation, sibling.bounds.x
                    )
                });
            }
            self.prev_hidden = hidden;
        }
        let disabled = scene
            .node(TARGET_CARD)
            .is_some_and(|node| node.effective_disabled);
        if disabled != self.prev_disabled {
            self.log(if disabled {
                format!(
                    "gen {}: target disabled — layout and paint unchanged, hit/focus removed",
                    scene.generation
                )
            } else {
                format!("gen {}: target interactive again", scene.generation)
            });
            self.prev_disabled = disabled;
        }

        self.state.hovered = scene.hit_test(self.cursor).map(|node| node.id);
        self.state.inspector = inspect(scene, self.state.selected);
        self.state.flash = (self.state.flash - 0.06).max(0.0);
    }
}

impl AppDelegate for Tonight {
    fn on_init(&mut self, gpu: &GpuContext, _resources: &mut RenderResources) {
        self.text = Some(TextRenderer::new(gpu).expect("a system sans-serif font is available"));
    }

    fn on_redraw(
        &mut self,
        gpu: &GpuContext,
        resources: &mut RenderResources,
        frame: &mut Frame,
        _perf: &esox_platform::perf::PerfMonitor,
    ) {
        let (scene, out) = {
            let renderer = self.text.as_mut().expect("initialized in on_init");
            let measurer = SystemTextMeasurer {
                renderer: RefCell::new(renderer),
            };
            let state = &self.state;
            let probe = &self.probe;
            let stage = self.script;
            let out_cell = RefCell::new(ShowcaseFrame::default());
            let scene = self
                .core
                .run_frame(&measurer, &mut self.sink, |widget_state| {
                    let mut ui = DeclarationUi::new(widget_state);
                    let out = declare_showcase(&mut ui, state, probe);
                    let root = ui.finish();
                    if stage == Script::AwaitPress && out.target_pressed {
                        widget_state.request_pointer_capture(SCRIPT_POINTER, TARGET_BUTTON);
                        widget_state.request_keyboard_focus(TARGET_BUTTON);
                    }
                    if out.target_clicked {
                        widget_state.request_keyboard_focus(TARGET_BUTTON);
                    }
                    out_cell.replace(out);
                    root
                })
                .expect("the showcase declaration is valid")
                .clone();
            (scene, out_cell.into_inner())
        };
        assert_eq!(self.probe.take(), CONTAINER_CLOSURES);
        assert_eq!(self.sink.last_generation, scene.generation);

        if out.select.is_some() || out.target_clicked || out.stale_requested {
            eprintln!("DBG gen={} out={out:?}", scene.generation);
        }
        self.after_commit(&scene, out);

        let renderer = self.text.as_mut().expect("initialized in on_init");
        let mut boundary = TextRendererFramePaint::new(renderer, gpu, resources);
        submit_display_list(&scene.display_list, frame, &mut boundary)
            .expect("the showcase display list uses only supported primitives");
    }

    fn on_key(&mut self, event: &KeyEvent, _modifiers: Modifiers) {
        if !event.pressed {
            return;
        }
        match &event.key {
            Key::Character(character) if character == "v" => self.toggle_hidden(),
            Key::Character(character) if character == "e" => self.toggle_disabled(),
            Key::Character(character) if character == "q" => self.stale_key_pending = true,
            _ => {}
        }
    }

    fn on_resize(&mut self, width: u32, height: u32, _gpu: &GpuContext) {
        self.core
            .resize(LogicalSize::new(width as f32, height as f32));
    }

    fn on_mouse(&mut self, event: MouseInputEvent) {
        match event {
            MouseInputEvent::Moved { x, y } => {
                self.cursor = LogicalPoint::new(x as f32, y as f32);
                self.core
                    .queue_pointer_event(PointerEventKind::Move, 0, self.cursor);
            }
            MouseInputEvent::Press { x, y, button: 0 } => {
                self.core.queue_pointer_event(
                    PointerEventKind::Press,
                    0,
                    LogicalPoint::new(x as f32, y as f32),
                );
            }
            MouseInputEvent::Release { x, y, button: 0 } => {
                self.core.queue_pointer_event(
                    PointerEventKind::Release,
                    0,
                    LogicalPoint::new(x as f32, y as f32),
                );
            }
            _ => {}
        }
    }

    fn on_scale_changed(&mut self, _scale_factor: f64, _gpu: &GpuContext) {}
    fn on_paste(&mut self, _text: &str) {}
    fn on_ime_commit(&mut self, _text: &str) {}

    fn on_copy(&mut self) -> Option<String> {
        None
    }

    fn needs_continuous_redraw(&self) -> bool {
        self.script != Script::Idle || self.stale_key_pending || self.state.flash > 0.0
    }

    fn cursor_icon(&self, x: f64, y: f64) -> CursorIcon {
        let point = LogicalPoint::new(x as f32, y as f32);
        let over_hit = self
            .core
            .committed_scene()
            .and_then(|scene| scene.hit_test(point))
            .is_some();
        if over_hit {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = PlatformConfig {
        window: WindowConfig {
            title: "FrameCore Tonight — v/e toggle state, q queues a stale click".into(),
            width: Some(1280),
            height: Some(820),
            ..Default::default()
        },
        background: "#0b0e14".into(),
        ..Default::default()
    };
    esox_platform::run(config, Box::new(Tonight::new())).unwrap();
}
