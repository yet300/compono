//! Where a desktop application keeps its saved tree.

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The system gave no directory for application state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoStateDir;

impl fmt::Display for NoStateDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("no directory for application state")
    }
}
impl std::error::Error for NoStateDir {}

/// A file holding the bytes of [`Tree::save`](https://docs.rs/compono).
///
/// Writing goes to a temporary file that then replaces the old one, so a crash
/// during a save leaves the previous state readable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateFile {
    path: PathBuf,
}

impl StateFile {
    /// Uses the file at `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Uses `state.bin` in the system's directory for the application `app`.
    ///
    /// That is `~/Library/Application Support/<app>` on macOS,
    /// `%LOCALAPPDATA%\<app>` on Windows, and `$XDG_STATE_HOME/<app>` or
    /// `~/.local/state/<app>` elsewhere.
    ///
    /// # Errors
    ///
    /// Returns [`NoStateDir`] when the environment names no such directory.
    pub fn in_app_dir(app: &str) -> Result<Self, NoStateDir> {
        let base = state_dir().ok_or(NoStateDir)?;
        Ok(Self::new(base.join(app).join("state.bin")))
    }

    /// Returns where the state lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads the saved bytes, or `None` if nothing has been saved yet.
    ///
    /// # Errors
    ///
    /// Returns any error other than a missing file.
    pub fn load(&self) -> io::Result<Option<Vec<u8>>> {
        match fs::read(&self.path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Replaces the saved bytes, creating the directory if needed.
    ///
    /// # Errors
    ///
    /// Returns the error of creating, writing or renaming the file.
    pub fn store(&self, bytes: &[u8]) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut temporary = self.path.clone().into_os_string();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        let written =
            write_synced(&temporary, bytes).and_then(|()| fs::rename(&temporary, &self.path));
        if written.is_err() {
            // The old state stays. The half-written file is of no use.
            let _already_gone = fs::remove_file(&temporary);
        }
        written
    }

    /// Removes the saved bytes. A missing file is not an error.
    ///
    /// # Errors
    ///
    /// Returns any error other than a missing file.
    pub fn clear(&self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value: &OsString| !value.is_empty())
        .map(PathBuf::from)
}

fn state_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library").join("Application Support"))
    } else if cfg!(windows) {
        var("LOCALAPPDATA")
    } else {
        var("XDG_STATE_HOME").or_else(|| var("HOME").map(|home| home.join(".local").join("state")))
    }
}
