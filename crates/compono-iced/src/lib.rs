//! Connects a Compono tree to iced.
//!
//! iced redraws after every message, so a [`Commit`](compono::Commit) needs no
//! action here. What the adapter supplies is the way back into the update loop:
//! [`wakes`] turns work finished on other threads into a message, and [`Action`]
//! lets a view say what should happen to the tree.
//!
//! ```no_run
//! # use compono::Tree;
//! # use compono_iced::{Action, IcedHost, wakes};
//! # use iced::Subscription;
//! #[derive(Clone, Debug)]
//! enum Message {
//!     Tree(Action),
//! }
//!
//! let host = IcedHost::new();
//! let signal = host.signal();
//! let mut tree = Tree::new(host);
//! let subscription: Subscription<Message> = wakes(&signal).map(Message::Tree);
//! let Message::Tree(action) = Message::Tree(Action::Pump);
//! action.apply(&mut tree);
//! # drop(subscription);
//! ```

mod action;
mod back;
mod host;
mod navigation;
mod window;

pub use action::{Action, send, wakes};
pub use back::{BackRequest, back_requests, is_back};
pub use host::IcedHost;
pub use navigation::{ListView, RouterView};
pub use window::{Presence, presence};
