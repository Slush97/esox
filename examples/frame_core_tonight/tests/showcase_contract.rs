//! Focused regression for the FrameCore Tonight showcase: every declaration
//! closure runs exactly once per frame, and renderer submission leaves the
//! committed scene untouched.

use std::convert::Infallible;

use esox_gfx::{Color as GfxColor, Frame, ShapeBuilder};
use esox_ui::declaration::run_declaration_frame;
use esox_ui::frame_core::{
    DeterministicMeasurer, FrameCore, LogicalSize, NullSceneConsumer, WidgetId,
};
use esox_ui::frame_scene_consumer::submit_display_list;
use esox_ui::scene_submission::{TextPaintBoundary, TextPaintRequest};
use frame_core_tonight::{
    CONTAINER_CLOSURES, DeclarationProbe, DemoState, SIBLING_CARD, TARGET_CARD, declare_showcase,
};

#[derive(Default)]
struct MarkerTextPaint {
    painted: Vec<WidgetId>,
}

impl TextPaintBoundary<Frame> for MarkerTextPaint {
    type Error = Infallible;

    fn paint_text(
        &mut self,
        frame: &mut Frame,
        request: TextPaintRequest<'_>,
    ) -> Result<(), Self::Error> {
        self.painted.push(request.id);
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

#[test]
fn every_closure_runs_once_and_submission_leaves_the_scene_unchanged() {
    let measurer = DeterministicMeasurer::new(8.0, 18.0);
    let mut consumer = NullSceneConsumer::default();
    let mut core = FrameCore::new(LogicalSize::new(1280.0, 820.0));
    let probe = DeclarationProbe::default();

    let enabled = DemoState::default();
    let hidden = DemoState {
        target_hidden: true,
        ..DemoState::default()
    };
    let disabled = DemoState {
        target_disabled: true,
        ..DemoState::default()
    };
    let restored = DemoState::default();

    for (frame_index, state) in [&enabled, &hidden, &disabled, &restored]
        .into_iter()
        .enumerate()
    {
        let mut applications = 0;
        let scene = run_declaration_frame(&mut core, &measurer, &mut consumer, |ui| {
            applications += 1;
            declare_showcase(ui, state, &probe);
        })
        .unwrap()
        .clone();

        assert_eq!(applications, 1, "frame {frame_index}");
        assert_eq!(probe.take(), CONTAINER_CLOSURES, "frame {frame_index}");
        assert_eq!(scene.generation, frame_index as u64 + 1);

        let scene_before_submission = scene.clone();
        let mut frame = Frame::new();
        let mut text = MarkerTextPaint::default();
        submit_display_list(&scene.display_list, &mut frame, &mut text).unwrap();

        assert_eq!(scene, scene_before_submission, "frame {frame_index}");
        assert_eq!(frame.instance_data().len(), scene.display_list.len());
        assert!(!text.painted.is_empty());

        let target = scene.node(TARGET_CARD).unwrap();
        let sibling = scene.node(SIBLING_CARD).unwrap();
        assert_eq!(target.effective_hidden, state.target_hidden);
        assert_eq!(target.effective_disabled, state.target_disabled);
        if state.target_hidden {
            assert!(sibling.bounds.x < 100.0, "sibling reflows into the gap");
        } else {
            assert!(target.paint_bounds.is_some());
        }
    }
}
