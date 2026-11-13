//! A component tree that sits between business logic and any UI.
//!
//! A [`Tree`] owns every [`Component`]. A component reports upward only through
//! its own `Output`, so it never knows its parent. While a component runs it is
//! outside the tree, and [`Cx`] is its only access to the tree, so a running
//! component cannot be reentered.
//!
//! ```
//! use compono::{Commit, Component, Cx, Host, Tree};
//!
//! struct Counter {
//!     value: u32,
//! }
//!
//! enum Msg {
//!     Add(u32),
//! }
//!
//! impl Component for Counter {
//!     type Msg = Msg;
//!     type Output = ();
//!
//!     fn update(&mut self, msg: Msg, _cx: &mut Cx<'_, Self>) {
//!         match msg {
//!             Msg::Add(amount) => self.value += amount,
//!         }
//!     }
//! }
//!
//! /// A host is told what changed after each operation and redraws that.
//! struct Redraw;
//!
//! impl Host for Redraw {
//!     fn commit(&mut self, commit: &Commit<'_>) {
//!         assert_eq!(commit.dirty.len(), 1);
//!     }
//! }
//!
//! let mut tree = Tree::new(Redraw);
//! let counter = tree.mount(|_cx| Counter { value: 0 });
//! tree.send(counter, Msg::Add(2))?;
//! assert_eq!(tree.get(counter).map(|counter| counter.value), Some(2));
//! # Ok::<(), compono::StaleHandle>(())
//! ```
//!
//! Navigation lives in [`router`]. Saved state is [`Persist`] with
//! [`Tree::save`] and [`Tree::with_saved`], behind the `serde` feature. Work on
//! other threads reaches a component through [`Sender`] and [`Cx::perform`],
//! behind the `std` feature.

#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod component;
#[cfg(feature = "std")]
mod concurrency;
mod host;
#[cfg(feature = "serde")]
mod persist;
pub mod router;
mod saved_key;
mod tree;

pub use component::{Component, Cx, Handle, NodeId, NotOwnChild, Phase, StaleHandle};
#[cfg(feature = "std")]
pub use concurrency::{
    NoSpawner, QueueClosed, SendError, Sender, Spawner, TaskGuard, TaskId, WakeSignal,
};
pub use host::{Commit, Diagnostic, Host};
#[cfg(feature = "serde")]
pub use persist::{Persist, RestoreOutcome, SaveError};
pub use tree::Tree;
