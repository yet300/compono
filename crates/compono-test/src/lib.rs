//! Test support for Compono: a recording host, virtual time and host checks.
//!
//! ```
//! use compono::{Component, Cx, Tree};
//! use compono_test::headless;
//!
//! struct Counter(u32);
//! impl Component for Counter {
//!     type Msg = u32;
//!     type Output = ();
//!     fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
//!         self.0 += msg;
//!     }
//! }
//!
//! let (host, log) = headless();
//! let mut tree = Tree::new(host);
//! let counter = tree.mount(|_cx| Counter(0));
//! log.clear();
//! assert_eq!(tree.send(counter, 2), Ok(()));
//! assert_eq!(log.len(), 1);
//! ```

mod conformance;
mod recording;
mod time;

pub use conformance::{Violation, check_host};
pub use recording::{
    CommitLog, CommitRecord, DiagnosticRecord, NoHost, Recording, headless, record,
};
pub use time::{Sleep, VirtualTime};
