//! A tour of every Compono navigation model, with no UI dependency.
//!
//! | Screen | Navigation |
//! |---|---|
//! | `root` | `Stack` of whole screens |
//! | `tabs` | `Stack` with `bring_to_front`, so each tab keeps its state |
//! | `menu` | `Slot` for a dialog |
//! | `counters` | Deep `Stack`, with state saved across restarts |
//! | `cards` | `ChildSet`, a list reconciled in any order |
//! | `articles` | `Panels`: one, two or three side by side |
//! | `feed` | `Items`: 100 000 rows, with components only on screen |
//! | `book` | `Pages`: keeps only neighbours alive |
//! | `gallery` | A custom navigation model written with the public API |

pub mod feature;
pub mod saved;
mod services;

pub use services::{Services, SleepFuture};
