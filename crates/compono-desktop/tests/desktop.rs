//! Saved-state files and Back shortcuts.

#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

use compono_desktop::{Convention, Key, Modifiers, StateFile};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("compono-desktop-{}-{name}", std::process::id()))
        .join("state.bin")
}

#[test]
fn a_missing_file_is_nothing_saved() {
    let file = StateFile::new(scratch("missing"));
    assert_eq!(file.load().unwrap(), None);
    file.clear().unwrap();
}

#[test]
fn store_replaces_and_creates_the_directory() {
    let path = scratch("store");
    let file = StateFile::new(&path);
    file.store(b"first").unwrap();
    file.store(b"second").unwrap();
    assert_eq!(file.load().unwrap().as_deref(), Some(&b"second"[..]));
    let mut temporary = path.clone().into_os_string();
    temporary.push(".tmp");
    assert!(
        !PathBuf::from(temporary).exists(),
        "the temporary file is renamed away"
    );
    file.clear().unwrap();
    assert_eq!(file.load().unwrap(), None);
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}

#[test]
fn an_app_dir_ends_in_the_app_name() {
    if let Ok(file) = StateFile::in_app_dir("compono-test") {
        assert!(file.path().ends_with("compono-test/state.bin"));
    }
}

fn with(f: impl FnOnce(&mut Modifiers)) -> Modifiers {
    let mut modifiers = Modifiers::default();
    f(&mut modifiers);
    modifiers
}

#[test]
fn every_system_takes_escape_and_back_buttons() {
    for convention in [Convention::Mac, Convention::Other] {
        assert!(convention.is_back(Key::Escape, Modifiers::default()));
        assert!(!convention.is_back(Key::Escape, with(|m| m.ctrl = true)));
        assert!(convention.is_back(Key::MouseBack, Modifiers::default()));
        assert!(convention.is_back(Key::BrowserBack, Modifiers::default()));
        assert!(!convention.is_back(Key::ArrowLeft, Modifiers::default()));
        assert!(!convention.is_back(Key::Other, Modifiers::default()));
    }
}

#[test]
fn mac_uses_command_and_others_use_alt() {
    let logo = with(|m| m.logo = true);
    let alt = with(|m| m.alt = true);
    assert!(Convention::Mac.is_back(Key::BracketLeft, logo));
    assert!(Convention::Mac.is_back(Key::ArrowLeft, logo));
    assert!(!Convention::Mac.is_back(Key::ArrowLeft, alt));
    assert!(Convention::Other.is_back(Key::ArrowLeft, alt));
    assert!(!Convention::Other.is_back(Key::ArrowLeft, logo));
    assert!(!Convention::Other.is_back(Key::BracketLeft, logo));
    assert!(!Convention::Mac.is_back(
        Key::BracketLeft,
        with(|m| {
            m.logo = true;
            m.shift = true;
        })
    ));
}

#[test]
fn a_failed_store_leaves_no_temporary_file() {
    let path = scratch("failed");
    // A directory with something in it cannot be replaced by a file.
    std::fs::create_dir_all(path.join("occupied")).unwrap();
    let file = StateFile::new(&path);
    assert!(file.store(b"state").is_err());
    let mut temporary = path.clone().into_os_string();
    temporary.push(".tmp");
    assert!(!PathBuf::from(temporary).exists());
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
