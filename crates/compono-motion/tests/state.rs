//! `TransitionState` driven with virtual time.

#![allow(clippy::expect_used, reason = "a failed expectation is a failed test")]

use compono_motion::{
    Direction, Easing, LayerStyle, Motion, Size, Spring, StackChange, Timing, Transition,
    TransitionState,
};

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "{actual} is not {expected}"
    );
}

fn linear_fade() -> Motion {
    Motion::none()
        .with_transition(Transition::Fade)
        .with_timing(Timing::Tween {
            millis: 200,
            easing: Easing::Linear,
        })
}

#[test]
fn a_new_state_rests_on_its_entry() {
    let mut state = TransitionState::new(4);
    let frame = state.frame(1.0);
    assert_eq!(frame.under, None);
    assert_eq!(frame.over.key, 4);
    assert_eq!(frame.over.style, LayerStyle::REST);
    assert!(!state.is_running());
    assert_eq!(TransitionState::default().active(), 0);
}

#[test]
fn a_push_draws_the_old_entry_under_the_new_one() {
    let mut state = TransitionState::new(0);
    state.start(StackChange::Forward { from: 0, to: 1 }, linear_fade(), 10.0);
    assert!(state.is_running());
    assert_eq!(state.active(), 1);
    let frame = state.frame(10.05);
    let under = frame.under.expect("a transition draws two layers");
    assert_eq!((under.key, under.direction), (0, Direction::ExitBack));
    assert_eq!(
        (frame.over.key, frame.over.direction),
        (1, Direction::EnterFront)
    );
    close(under.style.opacity, 0.75);
    close(frame.over.style.opacity, 0.25);
    let layers: Vec<u64> = frame.layers().map(|layer| layer.key).collect();
    assert_eq!(layers, [0, 1]);

    let frame = state.frame(10.2);
    assert!(!state.is_running());
    assert_eq!(frame.under, None);
    assert_eq!(frame.over.key, 1);
    assert_eq!(frame.over.style, LayerStyle::REST);
}

#[test]
fn a_pop_draws_the_leaving_entry_over_the_revealed_one() {
    let mut state = TransitionState::new(1);
    state.start(StackChange::Backward { from: 1, to: 0 }, linear_fade(), 0.0);
    let frame = state.frame(0.1);
    let under = frame.under.expect("a transition draws two layers");
    assert_eq!((under.key, under.direction), (0, Direction::EnterBack));
    assert_eq!(
        (frame.over.key, frame.over.direction),
        (1, Direction::ExitFront)
    );
    close(under.style.opacity, 0.5);
    close(frame.over.style.opacity, 0.5);
}

#[test]
fn time_before_the_start_shows_the_first_frame() {
    let mut state = TransitionState::new(0);
    state.start(StackChange::Forward { from: 0, to: 1 }, linear_fade(), 5.0);
    close(state.frame(4.0).over.style.opacity, 0.0);
    assert!(state.is_running());
}

#[test]
fn a_spring_runs_until_it_settles() {
    let mut state = TransitionState::new(0);
    let motion = Motion::none()
        .with_transition(Transition::Slide {
            parallax: 0.3,
            dim: 0.1,
        })
        .with_timing(Timing::Spring(Spring::MATERIAL_SPATIAL));
    state.start(StackChange::Forward { from: 0, to: 1 }, motion, 0.0);
    let mut now = 0.0;
    let mut last = 1.0;
    while now < 0.22 {
        now += 1.0 / 120.0;
        let offset = state.frame(now).over.style.offset_x;
        assert!(offset <= last + 1e-6, "the entry moves back at {now}");
        last = offset;
        assert!(state.is_running());
    }
    let settle = f64::from(Spring::MATERIAL_SPATIAL.settle_time());
    let _ = state.frame(settle + 0.001);
    assert!(!state.is_running());
    // Frames that skip far ahead give the same values as frames every 8 ms.
    let mut jumpy = TransitionState::new(0);
    jumpy.start(StackChange::Forward { from: 0, to: 1 }, motion, 0.0);
    let mut smooth = TransitionState::new(0);
    smooth.start(StackChange::Forward { from: 0, to: 1 }, motion, 0.0);
    for step in 1..=12 {
        let _ = smooth.frame(f64::from(step) / 120.0);
    }
    close(
        jumpy.frame(0.1).over.style.offset_x,
        smooth.frame(0.1).over.style.offset_x,
    );
}

#[test]
fn no_change_and_no_motion_start_nothing() {
    let mut state = TransitionState::new(3);
    state.start(StackChange::None, linear_fade(), 0.0);
    assert!(!state.is_running());
    state.start(StackChange::Forward { from: 3, to: 4 }, Motion::none(), 0.0);
    assert!(!state.is_running());
    assert_eq!(state.frame(0.0).over.key, 4);
    assert_eq!(state.frame(0.0).under, None);
}

#[test]
fn a_change_during_a_transition_starts_from_the_new_pair() {
    let mut state = TransitionState::new(0);
    state.start(StackChange::Forward { from: 0, to: 1 }, linear_fade(), 0.0);
    let _ = state.frame(0.1);
    state.start(StackChange::Forward { from: 1, to: 2 }, linear_fade(), 0.1);
    let frame = state.frame(0.1);
    assert_eq!(frame.under.map(|layer| layer.key), Some(1));
    assert_eq!(frame.over.key, 2);
    close(frame.over.style.opacity, 0.0);
}

#[test]
fn shared_axis_uses_the_size_it_was_given() {
    let mut state = TransitionState::new(0);
    state.set_size(Size::new(300.0, 600.0));
    let motion = Motion::none()
        .with_transition(Transition::SharedAxisX { shift_dp: 30.0 })
        .with_timing(Timing::Tween {
            millis: 100,
            easing: Easing::Linear,
        });
    state.start(StackChange::Forward { from: 0, to: 1 }, motion, 0.0);
    close(state.frame(0.0).over.style.offset_x, 0.1);
}
