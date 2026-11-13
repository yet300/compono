//! The Compono showcase in a browser.
//!
//! The screens are `showcase-egui`'s, drawn on a canvas. The page adds what a
//! browser needs: Back and Forward move through the root stack, a link such as
//! `#/book` opens that screen, and state survives a reload in `sessionStorage`.
//! [`ScreenPaths`] is the only part that is not browser code, so it is tested
//! natively.

mod paths;
#[cfg(target_arch = "wasm32")]
mod web;

pub use paths::ScreenPaths;
