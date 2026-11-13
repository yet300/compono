//! Integrated springs against their closed forms.

use compono_motion::{Spring, Timing};

fn critically_damped(spring: Spring, t: f32, velocity: f32) -> f32 {
    let omega = spring.stiffness.sqrt();
    1.0 + (-1.0 + (velocity - omega) * t) * (-omega * t).exp()
}

fn underdamped(spring: Spring, t: f32) -> f32 {
    let omega = spring.stiffness.sqrt();
    let zeta = spring.damping_ratio;
    let damped = omega * (1.0 - zeta * zeta).sqrt();
    1.0 - (-zeta * omega * t).exp()
        * ((damped * t).cos() + zeta * omega / damped * (damped * t).sin())
}

#[test]
fn presets_have_their_sources_values() {
    assert_eq!(Spring::MATERIAL_SPATIAL.damping_ratio, 0.9);
    assert_eq!(Spring::MATERIAL_SPATIAL.stiffness, 700.0);
    assert_eq!(Spring::CUPERTINO.damping_ratio, 1.0);
    assert!((Spring::CUPERTINO.stiffness - 469.4).abs() < 0.1);
}

#[test]
fn integration_matches_the_closed_form() {
    for step in 0..=60 {
        let t = step as f32 / 100.0;
        let (x, _) = Spring::CUPERTINO.position(t, 0.0);
        assert!(
            (x - critically_damped(Spring::CUPERTINO, t, 0.0)).abs() < 1e-5,
            "{t}"
        );
        let (x, _) = Spring::CUPERTINO.position(t, 4.0);
        assert!(
            (x - critically_damped(Spring::CUPERTINO, t, 4.0)).abs() < 1e-5,
            "{t}"
        );
        let (x, _) = Spring::MATERIAL_SPATIAL.position(t, 0.0);
        assert!(
            (x - underdamped(Spring::MATERIAL_SPATIAL, t)).abs() < 1e-5,
            "{t}"
        );
    }
}

#[test]
fn springs_settle_in_their_measured_times() {
    // The closed forms leave the 0.1 % band for the last time at 0.3166 s and
    // 0.4261 s.
    let material = Spring::MATERIAL_SPATIAL.settle_time();
    let cupertino = Spring::CUPERTINO.settle_time();
    assert!((material - 0.3166).abs() < 0.002, "{material}");
    assert!((cupertino - 0.4261).abs() < 0.002, "{cupertino}");
    for step in 1..=100 {
        let (x, _) = Spring::MATERIAL_SPATIAL.position(material + step as f32 / 100.0, 0.0);
        assert!((x - 1.0).abs() <= 1e-3);
    }
    let (_, done) = Timing::Spring(Spring::MATERIAL_SPATIAL).progress(material - 0.01);
    assert!(!done);
    let (_, done) = Timing::Spring(Spring::MATERIAL_SPATIAL).progress(material + 0.001);
    assert!(done);
}

#[test]
fn degenerate_springs_stay_bounded() {
    let slack = Spring {
        damping_ratio: 0.0,
        stiffness: 100.0,
    };
    assert_eq!(slack.settle_time(), 10.0);
    let (x, _) = slack.position(1e9, 0.0);
    assert!(x.is_finite());
    let none = Spring {
        damping_ratio: 1.0,
        stiffness: 0.0,
    };
    assert_eq!(none.position(1.0, 0.0), (0.0, 0.0));
    assert_eq!(Spring::from_response(0.0, 1.0).stiffness, 0.0);
}
