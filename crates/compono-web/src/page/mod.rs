//! The browser side, built only for `wasm32`.

mod events;
mod history;
mod storage;
mod tasks;

pub use events::{PageEvent, PageEvents, page_phase, prefers_reduced_motion};
pub use history::{Address, BrowserHistory};
pub use storage::SessionStore;
pub use tasks::{Sleep, WebSpawner, sleep};
