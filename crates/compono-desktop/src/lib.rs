//! Desktop conventions for Compono, independent of any UI toolkit.
//!
//! A desktop application keeps its tree's saved state in a [`StateFile`] and
//! treats the system's Back shortcuts, described by [`Convention`], as
//! [`Tree::back`](https://docs.rs/compono). UI adapters translate their own key
//! events into [`Key`] and [`Modifiers`].
//!
//! ```
//! use compono_desktop::{Convention, Key, Modifiers, StateFile};
//!
//! # let dir = std::env::temp_dir().join(format!("compono-doc-{}", std::process::id()));
//! # let path = dir.join("state.bin");
//! // An application normally uses `StateFile::in_app_dir("my-app")`.
//! let file = StateFile::new(&path);
//! assert_eq!(file.load()?, None, "nothing saved yet");
//! file.store(b"bytes from Tree::save")?;
//! assert_eq!(file.load()?.as_deref(), Some(&b"bytes from Tree::save"[..]));
//! file.clear()?;
//! # std::fs::remove_dir_all(&dir)?;
//!
//! let command = Modifiers { logo: true, ..Modifiers::default() };
//! assert!(Convention::Mac.is_back(Key::BracketLeft, command));
//! assert!(!Convention::Other.is_back(Key::BracketLeft, command));
//! assert!(Convention::CURRENT.is_back(Key::Escape, Modifiers::default()));
//! # Ok::<(), std::io::Error>(())
//! ```

mod back;
mod state_file;

pub use back::{Convention, Key, Modifiers};
pub use state_file::{NoStateDir, StateFile};
