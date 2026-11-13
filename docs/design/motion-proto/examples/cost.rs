use motion_proto::*;
use std::hint::black_box;
use std::time::Instant;
fn main() {
    let n = 1_000_000;
    let slide = StackTransition::Slide { parallax: 0.3, dim: 0.1 };
    let timing = Timing::Tween { millis: 300, easing: Easing::EMPHASIZED_DECELERATE };
    let start = Instant::now();
    let mut acc = 0.0f32;
    for i in 0..n {
        let (t, _) = timing.progress(black_box(i as f32 / n as f32 * 0.3));
        let f = slide.frame(Direction::Forward, t);
        acc += f.over.offset_x + f.under.offset_x;
    }
    let tween = start.elapsed().as_nanos() as f64 / n as f64;
    let start = Instant::now();
    let m = 10_000;
    for i in 0..m {
        let (x, _) = Spring::M3_STANDARD_DEFAULT_SPATIAL.position(black_box(i as f32 / m as f32 * 0.5), 0.0);
        acc += x;
    }
    let spring = start.elapsed().as_nanos() as f64 / m as f64;
    let start = Instant::now();
    for i in 0..n {
        let f = BackGestureStyle::Predictive { corner_radius: 28.0 }.frame(black_box(i as f32 / n as f32), Edge::Left, 400.0);
        acc += f.over.scale;
    }
    let back = start.elapsed().as_nanos() as f64 / n as f64;
    println!("tween+slide frame: {tween:.1} ns; spring position (integrated, mid-range t): {spring:.0} ns; predictive back frame: {back:.1} ns; ({acc})");
}
