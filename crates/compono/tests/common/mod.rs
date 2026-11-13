//! What the integration tests of `compono` share.
//!
//! `compono_test::NoHost` does the same, but a dev-dependency on `compono-test`
//! turns on `compono`'s default features, so `cargo test -p compono
//! --no-default-features` would no longer test without them.

use compono::{Commit, Host};

/// A host that ignores every commit.
pub struct NullHost;

impl Host for NullHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}
