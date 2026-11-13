//! Writes the golden table that the Swift and Kotlin tests compare with.
//!
//! Each row is one layer at one moment. `x` is seconds for `push` and `pop`,
//! gesture progress for `back` and `back_right`, and `finish` for `back_finish`,
//! which holds progress at 0.5. The router is 400 × 800 logical pixels and the
//! touch has not moved vertically, except in `back_touch`, where `x` is the touch's
//! travel at progress 0.5.

use compono_motion::{BackEvent, Direction, LayerStyle, Motion, MotionPreference, Size, SwipeEdge};
use std::fmt::Write;

const SIZE: Size = Size::new(400.0, 800.0);
const TIMES: [f32; 10] = [0.0, 0.025, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.5, 1.0];
const PROGRESS: [f32; 7] = [0.0, 0.1, 0.25, 0.35, 0.5, 0.75, 1.0];
const FINISH: [f32; 4] = [0.0, 0.25, 0.5, 1.0];
const TOUCH: [f32; 3] = [-100.0, 20.0, 100.0];

/// The header line of the table.
pub const HEADER: &str =
    "preset,preference,section,role,x,offset_x,offset_y,scale,opacity,corner_radius,dim";

fn row(out: &mut String, start: [&str; 4], x: f32, style: LayerStyle) {
    let [preset, preference, section, role] = start;
    // Adding 0.0 turns -0.0 into 0.0, so both languages print the same.
    let n = |value: f32| value + 0.0;
    let _ = writeln!(
        out,
        "{preset},{preference},{section},{role},{:.3},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
        n(x),
        n(style.offset_x),
        n(style.offset_y),
        n(style.scale),
        n(style.opacity),
        n(style.corner_radius),
        n(style.dim),
    );
}

fn role(direction: Direction) -> &'static str {
    match direction {
        Direction::EnterFront => "enter_front",
        Direction::ExitFront => "exit_front",
        Direction::EnterBack => "enter_back",
        Direction::ExitBack => "exit_back",
    }
}

/// Returns the whole table.
pub fn table() -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{HEADER}");
    let presets = [
        ("material", Motion::material()),
        ("cupertino", Motion::cupertino()),
        ("desktop", Motion::desktop()),
        ("web", Motion::web()),
    ];
    let preferences = [
        ("full", MotionPreference::Full),
        ("reduced", MotionPreference::Reduced),
        ("off", MotionPreference::Off),
    ];
    for (preset, motion) in presets {
        for (preference, choice) in preferences {
            let motion = motion.for_preference(choice);
            for (section, roles) in [
                ("push", [Direction::ExitBack, Direction::EnterFront]),
                ("pop", [Direction::EnterBack, Direction::ExitFront]),
            ] {
                for t in TIMES {
                    let (progress, done) = motion.timing.progress(t);
                    for direction in roles {
                        let style = if done {
                            LayerStyle::REST
                        } else {
                            motion.transition.layer(direction, progress, SIZE)
                        };
                        row(
                            &mut out,
                            [preset, preference, section, role(direction)],
                            t,
                            style,
                        );
                    }
                }
            }
            let back = |progress, edge, touch_y, finish| {
                motion.back.frame(
                    BackEvent {
                        progress,
                        edge,
                        touch_y,
                    },
                    finish,
                    SIZE,
                )
            };
            let mut gesture = |section, x: f32, frame: compono_motion::StackFrame| {
                row(
                    &mut out,
                    [preset, preference, section, "under"],
                    x,
                    frame.under,
                );
                row(
                    &mut out,
                    [preset, preference, section, "over"],
                    x,
                    frame.over,
                );
            };
            for p in PROGRESS {
                gesture("back", p, back(p, SwipeEdge::Left, 0.0, 0.0));
            }
            for p in PROGRESS {
                gesture("back_right", p, back(p, SwipeEdge::Right, 0.0, 0.0));
            }
            for f in FINISH {
                gesture("back_finish", f, back(0.5, SwipeEdge::Left, 0.0, f));
            }
            for y in TOUCH {
                gesture("back_touch", y, back(0.5, SwipeEdge::Left, y, 0.0));
            }
        }
    }
    out
}
