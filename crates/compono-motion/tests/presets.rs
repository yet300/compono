//! Each preset, and what reduced motion and no motion make of it.

use compono_motion::{
    BackAnimation, BackEvent, Direction, Easing, LayerStyle, Motion, MotionPreference, Size,
    Spring, SwipeEdge, Timing, Transition,
};

const SIZE: Size = Size::new(400.0, 800.0);

fn reduced_fade() -> Timing {
    Timing::Tween {
        millis: 150,
        easing: Easing::STANDARD,
    }
}

fn presets() -> [(&'static str, Motion); 4] {
    [
        ("material", Motion::material()),
        ("cupertino", Motion::cupertino()),
        ("desktop", Motion::desktop()),
        ("web", Motion::web()),
    ]
}

#[test]
fn presets_have_their_platforms_values() {
    let material = Motion::material();
    assert!(
        matches!(material.transition, Transition::SharedAxisX { shift_dp } if shift_dp == 30.0)
    );
    assert_eq!(material.timing, Timing::Spring(Spring::MATERIAL_SPATIAL));
    assert_eq!(
        material.back,
        BackAnimation::Predictive {
            corner_radius: 16.0
        }
    );

    let cupertino = Motion::cupertino();
    assert!(matches!(
        cupertino.transition,
        Transition::Slide { parallax, dim } if parallax == 0.3 && dim == 0.12
    ));
    assert_eq!(cupertino.timing, Timing::Spring(Spring::CUPERTINO));
    assert_eq!(
        cupertino.back,
        BackAnimation::Swipe {
            parallax: 0.3,
            dim: 0.12
        }
    );

    let desktop = Motion::desktop();
    assert!(matches!(desktop.transition, Transition::Fade));
    assert_eq!(desktop.timing, reduced_fade());
    assert_eq!(desktop.back, BackAnimation::None);

    let web = Motion::web();
    assert!(matches!(web.transition, Transition::Fade));
    assert_eq!(
        web.timing,
        Timing::Tween {
            millis: 250,
            easing: Easing::CubicBezier(0.25, 0.1, 0.25, 1.0)
        }
    );
    assert_eq!(web.back, BackAnimation::None);
}

#[test]
fn full_motion_changes_nothing() {
    for (name, motion) in presets() {
        let full = motion.for_preference(MotionPreference::Full);
        assert_eq!(full.timing, motion.timing, "{name}");
        assert_eq!(full.back, motion.back, "{name}");
        assert_eq!(
            full.transition.layer(Direction::EnterFront, 0.5, SIZE),
            motion.transition.layer(Direction::EnterFront, 0.5, SIZE),
            "{name}"
        );
    }
}

#[test]
fn reduced_motion_fades_without_travel() {
    // Material and Cupertino move entries, so they become a 150 ms fade.
    for motion in [Motion::material(), Motion::cupertino()] {
        let reduced = motion.for_preference(MotionPreference::Reduced);
        assert!(matches!(reduced.transition, Transition::Fade));
        assert_eq!(reduced.timing, reduced_fade());
        assert_eq!(reduced.back, BackAnimation::Fade);
    }
    // Desktop and web already fade, so they keep their timing.
    for motion in [Motion::desktop(), Motion::web()] {
        let reduced = motion.for_preference(MotionPreference::Reduced);
        assert!(matches!(reduced.transition, Transition::Fade));
        assert_eq!(reduced.timing, motion.timing);
        assert_eq!(reduced.back, BackAnimation::None);
    }
    // Every reduced layer stays in place and only changes its opacity.
    for (name, motion) in presets() {
        let reduced = motion.for_preference(MotionPreference::Reduced);
        for direction in [
            Direction::EnterFront,
            Direction::ExitFront,
            Direction::EnterBack,
            Direction::ExitBack,
        ] {
            let style = reduced.transition.layer(direction, 0.4, SIZE);
            assert_eq!(
                LayerStyle {
                    opacity: 1.0,
                    ..style
                },
                LayerStyle::REST,
                "{name} {direction:?}"
            );
        }
        let event = BackEvent {
            progress: 0.4,
            edge: SwipeEdge::Left,
            touch_y: 30.0,
        };
        let frame = reduced.back.frame(event, 0.0, SIZE);
        assert_eq!(frame.under, LayerStyle::REST, "{name}");
        assert_eq!(
            LayerStyle {
                opacity: 1.0,
                ..frame.over
            },
            LayerStyle::REST,
            "{name}"
        );
    }
    let gesture = Motion::material()
        .for_preference(MotionPreference::Reduced)
        .back
        .frame(
            BackEvent {
                progress: 0.4,
                edge: SwipeEdge::Left,
                touch_y: 0.0,
            },
            0.0,
            SIZE,
        );
    assert!((gesture.over.opacity - 0.6).abs() < 1e-6);
}

#[test]
fn no_motion_switches_at_once() {
    for (name, motion) in presets() {
        let off = motion.for_preference(MotionPreference::Off);
        assert!(matches!(off.transition, Transition::None), "{name}");
        assert_eq!(off.timing.progress(0.0), (1.0, true), "{name}");
        assert_eq!(off.back, BackAnimation::None, "{name}");
    }
}
