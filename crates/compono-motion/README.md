# compono-motion

Router transitions for Compono UIs: easing curves, springs, the presets of each
platform, the direction of a stack change and the layer values of each frame. It is
`no_std`, has no dependencies and never allocates.

The egui, iced and GPUI adapters call it every frame. The Swift package and the
Compose library use their own animation engines; their tests compare their values
with `golden/motion.csv`, which this command writes:

```bash
cargo run -p compono-motion --example golden > crates/compono-motion/golden/motion.csv
```

A test fails when the file and the presets disagree.

| Preset | Transition | Timing | Back gesture |
|---|---|---|---|
| `Motion::material()` | shared axis X, 30 dp | spring, damping 0.9, stiffness 700 | Material predictive Back |
| `Motion::cupertino()` | slide, parallax 0.3, shade 0.12 | spring, response 0.29 s, damping 1 (UIKit push, measured) | swipe |
| `Motion::desktop()` | fade | 150 ms, Material standard easing | none |
| `Motion::web()` | fade | 250 ms, CSS `ease` | none |

`Motion::for_preference` applies reduced motion: a spatial transition becomes a
150 ms fade and a Back gesture only fades the top entry; `Off` removes motion.

This crate is part of [Compono](https://crates.io/crates/compono). Its documentation
has a compiled example on the first page.

The minimum Rust version is 1.96. `VERSIONING.md` in the repository says what a
version number promises.

## Licence

Apache-2.0. See `LICENSE`.
