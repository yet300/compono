//! Exact layer values at fixed progress and times.

use compono_motion::{
    BackAnimation, BackEvent, Direction, Easing, LayerStyle, Size, SwipeEdge, Timing, Transition,
};

const SIZE: Size = Size::new(400.0, 800.0);

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "{actual} is not {expected}"
    );
}

fn same(actual: LayerStyle, expected: LayerStyle) {
    close(actual.offset_x, expected.offset_x);
    close(actual.offset_y, expected.offset_y);
    close(actual.scale, expected.scale);
    close(actual.opacity, expected.opacity);
    close(actual.corner_radius, expected.corner_radius);
    close(actual.dim, expected.dim);
}

fn style(f: impl FnOnce(&mut LayerStyle)) -> LayerStyle {
    let mut style = LayerStyle::REST;
    f(&mut style);
    style
}

const ROLES: [Direction; 4] = [
    Direction::EnterFront,
    Direction::ExitFront,
    Direction::EnterBack,
    Direction::ExitBack,
];

#[test]
fn easing_passes_known_points() {
    for easing in [
        Easing::Linear,
        Easing::STANDARD,
        Easing::EMPHASIZED_DECELERATE,
        Easing::STANDARD_DECELERATE,
        Easing::FAST_OUT_SLOW_IN,
    ] {
        close(easing.ease(0.0), 0.0);
        close(easing.ease(1.0), 1.0);
        close(easing.ease(-1.0), 0.0);
        close(easing.ease(2.0), 1.0);
        let mut last = 0.0;
        for step in 1..=100 {
            let value = easing.ease(step as f32 / 100.0);
            assert!(value >= last - 1e-6, "{easing:?} decreases at {step}");
            last = value;
        }
    }
    close(Easing::Linear.ease(0.3), 0.3);
    close(Easing::CubicBezier(0.42, 0.0, 0.58, 1.0).ease(0.5), 0.5);
    // Reference values from bisecting the Bézier in double precision.
    close(Easing::FAST_OUT_SLOW_IN.ease(0.5), 0.775_561);
    close(Easing::STANDARD.ease(0.5), 0.877_834);
}

#[test]
fn tweens_run_for_their_duration() {
    let tween = Timing::Tween {
        millis: 300,
        easing: Easing::Linear,
    };
    let (value, done) = tween.progress(0.15);
    close(value, 0.5);
    assert!(!done);
    assert_eq!(tween.progress(0.3), (1.0, true));
    assert_eq!(tween.progress(-1.0), (0.0, false));
    let instant = Timing::Tween {
        millis: 0,
        easing: Easing::Linear,
    };
    assert_eq!(instant.progress(0.0), (1.0, true));
}

#[test]
fn slide_moves_the_front_entry_across_and_the_back_one_by_its_parallax() {
    let slide = Transition::Slide {
        parallax: 0.3,
        dim: 0.1,
    };
    same(
        slide.layer(Direction::EnterFront, 0.25, SIZE),
        style(|s| s.offset_x = 0.75),
    );
    same(
        slide.layer(Direction::ExitBack, 0.25, SIZE),
        style(|s| {
            s.offset_x = -0.075;
            s.dim = 0.025;
        }),
    );
    same(
        slide.layer(Direction::ExitFront, 0.25, SIZE),
        style(|s| s.offset_x = 0.25),
    );
    same(
        slide.layer(Direction::EnterBack, 0.25, SIZE),
        style(|s| {
            s.offset_x = -0.225;
            s.dim = 0.075;
        }),
    );
}

#[test]
fn fade_scale_and_slide_up() {
    same(
        Transition::Fade.layer(Direction::EnterFront, 0.25, SIZE),
        style(|s| s.opacity = 0.25),
    );
    same(
        Transition::Fade.layer(Direction::ExitBack, 0.25, SIZE),
        style(|s| s.opacity = 0.75),
    );
    let scale = Transition::Scale {
        front: 1.15,
        back: 0.95,
    };
    same(
        scale.layer(Direction::EnterFront, 0.5, SIZE),
        style(|s| {
            s.scale = 1.075;
            s.opacity = 0.5;
        }),
    );
    same(
        scale.layer(Direction::ExitFront, 0.5, SIZE),
        style(|s| {
            s.scale = 1.075;
            s.opacity = 0.5;
        }),
    );
    same(
        scale.layer(Direction::EnterBack, 0.5, SIZE),
        style(|s| s.scale = 0.975),
    );
    same(
        scale.layer(Direction::ExitBack, 0.5, SIZE),
        style(|s| s.scale = 0.975),
    );
    same(
        Transition::SlideUp.layer(Direction::EnterFront, 0.25, SIZE),
        style(|s| s.offset_y = 0.75),
    );
    same(
        Transition::SlideUp.layer(Direction::ExitFront, 0.25, SIZE),
        style(|s| s.offset_y = 0.25),
    );
    same(
        Transition::SlideUp.layer(Direction::ExitBack, 0.25, SIZE),
        LayerStyle::REST,
    );
}

#[test]
fn shared_axis_shifts_by_logical_pixels_and_fades_through() {
    let axis = Transition::SharedAxisX { shift_dp: 30.0 };
    same(
        axis.layer(Direction::EnterFront, 0.5, SIZE),
        style(|s| {
            s.offset_x = 0.0375;
            s.opacity = 0.15 / 0.65;
        }),
    );
    same(
        axis.layer(Direction::ExitBack, 0.2, SIZE),
        style(|s| {
            s.offset_x = -0.015;
            s.opacity = 1.0 - 0.2 / 0.35;
        }),
    );
    same(
        axis.layer(Direction::ExitBack, 0.5, SIZE),
        style(|s| {
            s.offset_x = -0.0375;
            s.opacity = 0.0;
        }),
    );
    same(
        axis.layer(Direction::ExitFront, 0.2, SIZE),
        style(|s| {
            s.offset_x = 0.015;
            s.opacity = 1.0 - 0.2 / 0.35;
        }),
    );
    same(
        axis.layer(Direction::EnterBack, 0.5, SIZE),
        style(|s| {
            s.offset_x = -0.0375;
            s.opacity = 0.15 / 0.65;
        }),
    );
    // With no width yet, nothing shifts.
    close(
        axis.layer(Direction::EnterFront, 0.5, Size::default())
            .offset_x,
        0.0,
    );
}

#[test]
fn every_transition_ends_at_rest() {
    fn custom(direction: Direction, progress: f32) -> LayerStyle {
        let mut style = LayerStyle::REST;
        if direction.is_enter() {
            style.offset_y = 1.0 - progress;
        } else {
            style.offset_y = progress;
        }
        style
    }
    for transition in [
        Transition::Slide {
            parallax: 0.3,
            dim: 0.1,
        },
        Transition::Fade,
        Transition::Scale {
            front: 1.15,
            back: 0.95,
        },
        Transition::SharedAxisX { shift_dp: 30.0 },
        Transition::SlideUp,
        Transition::Custom(custom),
    ] {
        for role in ROLES {
            let rest = if role.is_enter() { 1.0 } else { 0.0 };
            same(transition.layer(role, rest, SIZE), LayerStyle::REST);
        }
    }
    same(
        Transition::Custom(custom).layer(Direction::EnterFront, 0.25, SIZE),
        style(|s| s.offset_y = 0.75),
    );
    same(
        Transition::None.layer(Direction::EnterFront, 0.0, SIZE),
        LayerStyle::REST,
    );
    close(
        Transition::None
            .layer(Direction::ExitBack, 0.0, SIZE)
            .opacity,
        0.0,
    );
}

fn event(progress: f32, edge: SwipeEdge, touch_y: f32) -> BackEvent {
    BackEvent {
        progress,
        edge,
        touch_y,
    }
}

#[test]
fn swipe_follows_the_finger_and_finishes_the_slide() {
    let swipe = BackAnimation::Swipe {
        parallax: 0.3,
        dim: 0.1,
    };
    let frame = swipe.frame(event(0.5, SwipeEdge::Left, 0.0), 0.0, SIZE);
    same(frame.over, style(|s| s.offset_x = 0.5));
    same(
        frame.under,
        style(|s| {
            s.offset_x = -0.15;
            s.dim = 0.05;
        }),
    );
    let frame = swipe.frame(event(0.5, SwipeEdge::Left, 0.0), 0.5, SIZE);
    same(frame.over, style(|s| s.offset_x = 0.75));
    same(
        frame.under,
        style(|s| {
            s.offset_x = -0.075;
            s.dim = 0.025;
        }),
    );
    let frame = swipe.frame(event(0.0, SwipeEdge::Left, 0.0), 0.0, SIZE);
    same(frame.over, LayerStyle::REST);
}

#[test]
fn predictive_back_shrinks_toward_the_far_edge() {
    let back = BackAnimation::Predictive {
        corner_radius: 16.0,
    };
    let frame = back.frame(event(1.0, SwipeEdge::Left, 0.0), 0.0, SIZE);
    same(
        frame.over,
        style(|s| {
            s.scale = 0.9;
            s.offset_x = 0.05 - 8.0 / 400.0;
            s.corner_radius = 16.0;
        }),
    );
    same(frame.under, style(|s| s.dim = 0.25));
    close(
        back.frame(event(1.0, SwipeEdge::Right, 0.0), 0.0, SIZE)
            .over
            .offset_x,
        -0.03,
    );
    close(
        back.frame(event(1.0, SwipeEdge::None, 0.0), 0.0, SIZE)
            .over
            .offset_x,
        0.0,
    );
    // The touch moves the entry vertically, at most height / 20 - 8 dp, growing
    // with three times the progress.
    let frame = back.frame(event(0.5, SwipeEdge::Left, 100.0), 0.0, SIZE);
    same(
        frame.over,
        style(|s| {
            s.scale = 0.95;
            s.offset_x = 0.025 - 0.01;
            s.offset_y = 32.0 / 800.0;
            s.corner_radius = 8.0;
        }),
    );
    close(
        back.frame(event(0.1, SwipeEdge::Left, -100.0), 0.0, SIZE)
            .over
            .offset_y,
        -9.6 / 800.0,
    );
    // After a commit the entry fades and the scrim lifts.
    let frame = back.frame(event(1.0, SwipeEdge::Left, 0.0), 0.5, SIZE);
    close(frame.over.opacity, 0.5);
    close(frame.under.dim, 0.125);
    same(
        back.frame(event(0.0, SwipeEdge::Left, 0.0), 0.0, SIZE).over,
        LayerStyle::REST,
    );
}

#[test]
fn reduced_and_absent_back_animations() {
    let frame = BackAnimation::Fade.frame(event(0.5, SwipeEdge::Left, 40.0), 0.0, SIZE);
    same(frame.over, style(|s| s.opacity = 0.5));
    same(frame.under, LayerStyle::REST);
    close(
        BackAnimation::Fade
            .frame(event(0.5, SwipeEdge::Left, 0.0), 1.0, SIZE)
            .over
            .opacity,
        0.0,
    );
    let frame = BackAnimation::None.frame(event(0.7, SwipeEdge::Left, 40.0), 0.5, SIZE);
    same(frame.over, LayerStyle::REST);
    same(frame.under, LayerStyle::REST);
}
