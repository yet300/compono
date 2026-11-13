//! Connects a Compono tree to egui.
//!
//! Immediate-mode UI reads the tree while drawing, so it cannot change the tree
//! at the same moment. Views read through `&Tree` and queue changes in an
//! [`Outbox`], which the application flushes once the frame's views are done.
//!
//! ```
//! use compono::{Component, Cx, Tree};
//! use compono_egui::{EguiHost, Outbox};
//!
//! struct Counter(u32);
//!
//! impl Component for Counter {
//!     type Msg = u32;
//!     type Output = ();
//!     fn update(&mut self, amount: u32, _cx: &mut Cx<'_, Self>) {
//!         self.0 += amount;
//!     }
//! }
//!
//! let ctx = egui::Context::default();
//! let mut tree = Tree::new(EguiHost::new(ctx.clone()));
//! let counter = tree.mount(|_cx| Counter(0));
//! let mut out = Outbox::default();
//!
//! // One frame: views read the tree and queue what the user asked for.
//! let mut frame = ctx.run_ui(egui::RawInput::default(), |_ui| {
//!     // A real view draws this value and a button beside it.
//!     let _value = tree.get(counter).map_or(0, |counter| counter.0);
//!     // It sends this when the button is clicked.
//!     out.send(counter, 1);
//! });
//! # frame.textures_delta.clear();
//! // After the frame nothing borrows the tree, so the changes are applied.
//! out.flush(&mut tree);
//! assert_eq!(tree.get(counter).map(|counter| counter.0), Some(1));
//! ```

mod back;
mod host;
mod navigation;
mod outbox;
mod window;

pub use back::back_requested;
pub use host::{BeforeDraw, EguiHost};
pub use navigation::{ListView, RouterView};
pub use outbox::Outbox;
pub use window::window_phase;
