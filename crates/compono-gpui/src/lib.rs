//! Connects a Compono tree to GPUI.
//!
//! A GPUI view owns the [`Tree`](compono::Tree). Everything that changes the tree goes through
//! [`update`], which redraws only the views of components that really changed.
//! [`listen`] runs on the foreground executor and delivers work finished on
//! other threads.

mod back;
mod component_view;
mod host;
mod navigation;
mod spawner;
mod view;
mod window;

pub use back::{is_back_button, is_back_key};
pub use component_view::{ComponentViews, Draw};
pub use host::{Changes, GpuiHost};
pub use navigation::{ListView, RouterView};
pub use spawner::GpuiSpawner;
pub use view::{TreeView, listen, send, update};
pub use window::{follow_window, window_phase};
