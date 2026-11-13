//! The browser as a Compono platform.
//!
//! [`history`] keeps browser history and a [`Stack`](compono::router::stack::Stack)
//! in step and is plain logic that runs anywhere. On `wasm32` the crate also
//! provides the browser side: `BrowserHistory`, page lifecycle through
//! `PageEvents`, `SessionStore` for saved state, `WebSpawner` for tasks and
//! `sleep` for timers.

pub mod history;

#[cfg(target_arch = "wasm32")]
mod page;

#[cfg(target_arch = "wasm32")]
pub use page::{
    Address, BrowserHistory, PageEvent, PageEvents, SessionStore, Sleep, WebSpawner, page_phase,
    sleep,
};
