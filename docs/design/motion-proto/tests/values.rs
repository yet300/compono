use motion_proto::*;

fn close(a: f32, b: f32) { assert!((a - b).abs() < 1e-3, "{a} != {b}"); }

#[test]
fn slide_push_at_quarter_points() {
    let s = StackTransition::Slide { parallax: 0.3, dim: 0.1 };
    let f = s.frame(Direction::Forward, 0.5);
    close(f.over.offset_x, 0.5);
    close(f.under.offset_x, -0.15);
    close(f.under.dim, 0.05);
    let b = s.frame(Direction::Backward, 0.25);
    close(b.over.offset_x, 0.25); // leaving entry moves right
    close(b.under.offset_x, -0.225);
}

#[test]
fn ends_are_at_rest() {
    for s in [StackTransition::Slide { parallax: 0.3, dim: 0.1 }, StackTransition::Fade,
              StackTransition::Scale { front: 1.15, back: 0.95 }, StackTransition::SharedAxisX { shift: 0.08 }] {
        assert_eq!(s.frame(Direction::Forward, 1.0).over, LayerStyle::REST, "{s:?}");
        assert_eq!(s.frame(Direction::Backward, 0.0).over, LayerStyle::REST, "{s:?}");
    }
}

#[test]
fn easing_matches_known_points() {
    // CSS ease-in-out symmetric point and M3 standard at its end points.
    close(Easing::CubicBezier(0.42, 0.0, 0.58, 1.0).ease(0.5), 0.5);
    close(Easing::STANDARD.ease(0.0), 0.0);
    close(Easing::STANDARD.ease(1.0), 1.0);
    // FastOutSlowIn(0.4,0,0.2,1) at x=0.5: reference value from Chromium/Compose ≈ 0.7742
    let v = Easing::FAST_OUT_SLOW_IN.ease(0.5);
    assert!((v - 0.774).abs() < 0.005, "{v}");
}

#[test]
fn springs_settle_in_plausible_times() {
    let m3 = Spring::M3_STANDARD_DEFAULT_SPATIAL.settle_time();
    let ios = Spring::SWIFTUI_DEFAULT.settle_time();
    println!("m3 default spatial settles in {m3:.3}s, swiftui default in {ios:.3}s");
    assert!(m3 > 0.2 && m3 < 0.6, "{m3}");
    assert!(ios > 0.6 && ios < 1.2, "{ios}");
}

#[test]
fn predictive_back_matches_android_guide() {
    let f = BackGestureStyle::Predictive { corner_radius: 28.0 }.frame(1.0, Edge::Left, 400.0);
    close(f.over.scale, 0.9);
    close(f.over.offset_x, 0.05 - 8.0 / 400.0);
    close(f.over.opacity, 0.0);
    close(f.under.scale, 1.0);
    let mid = BackGestureStyle::Predictive { corner_radius: 28.0 }.frame(0.35, Edge::Left, 400.0);
    close(mid.over.opacity, 0.0);
    close(mid.under.opacity, 0.0); // "At the 35% mark, neither screen is showing."
}
