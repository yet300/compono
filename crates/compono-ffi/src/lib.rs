//! The Rust side of a Compono tree driven by SwiftUI, Compose or another native UI.
//!
//! This crate does not depend on a binding generator. An application exports its
//! own functions and records with one, and calls [`NativeTree`] from them.
//!
//! [`NativeTree`] can be held by any language runtime and sent between threads,
//! but the tree itself stays on the thread that created it. A call from any other
//! thread returns [`NativeError::WrongThread`].
//!
//! ```
//! use compono::{Component, Cx, NodeId};
//! use compono_ffi::{NativeError, NativeTree, Platform};
//! use std::sync::Arc;
//! use std::sync::atomic::{AtomicUsize, Ordering};
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
//! /// What an exported function returns for the native UI to draw.
//! fn count(tree: &NativeTree, counter: u64) -> Result<u32, NativeError> {
//!     tree.read(|tree| {
//!         let handle = tree.handle_of::<Counter>(NodeId::from_bits(counter));
//!         let counter = handle.and_then(|handle| tree.get(handle));
//!         counter.map(|counter| counter.0).ok_or(NativeError::Gone)
//!     })?
//! }
//!
//! /// What the Swift or Kotlin side implements through its binding generator.
//! #[derive(Default)]
//! struct Ui {
//!     redraws: AtomicUsize,
//! }
//!
//! impl Platform for Ui {
//!     fn commit(&self, dirty: &[u64], _can_back: bool) {
//!         self.redraws.fetch_add(dirty.len(), Ordering::Relaxed);
//!     }
//!     fn schedule_pump(&self) {}
//! }
//!
//! let ui = Arc::new(Ui::default());
//! let tree = NativeTree::new(ui.clone());
//! // An address crosses the language boundary as one number.
//! let counter = tree.with(|tree| tree.mount(|_cx| Counter(0)).id().to_bits())?;
//!
//! // An exported function receives the address back and names the type.
//! tree.send::<Counter>(counter, 5)?;
//! assert_eq!(count(&tree, counter), Ok(5));
//! assert_eq!(ui.redraws.load(Ordering::Relaxed), 1);
//!
//! let elsewhere = std::thread::scope(|scope| {
//!     scope.spawn(|| count(&tree, counter)).join()
//! });
//! assert!(matches!(elsewhere, Ok(Err(NativeError::WrongThread))));
//! # Ok::<(), NativeError>(())
//! ```

mod error;
mod host;
mod model;
mod native_tree;

pub use error::NativeError;
pub use host::{NativeHost, Platform};
pub use model::{NO_NODE, phase_code, phase_from_code};
pub use native_tree::{NativeTree, collect_retired};
