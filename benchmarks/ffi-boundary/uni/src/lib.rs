//! The boundary benchmark through UniFFI.

use compono_ffi::Platform;
use std::sync::Arc;

uniffi::setup_scaffolding!();

#[derive(Clone, Copy, uniffi::Record)]
pub struct RowModel {
    pub id: u64,
    pub value: u64,
    pub liked: bool,
}

#[derive(Clone, Copy, uniffi::Record)]
pub struct EntryModel {
    pub key: u64,
    pub kind: u32,
    pub node: u64,
    pub phase: u8,
}

#[derive(Clone, uniffi::Record)]
pub struct ArticleModel {
    pub id: u64,
    pub title: String,
    pub body: String,
}

#[uniffi::export(with_foreign)]
pub trait Listener: Send + Sync {
    /// Addresses are `i64` because BoltFFI 0.32.1 cannot bind a Kotlin `ULongArray` here.
    fn commit(&self, dirty: Vec<i64>, can_back: bool);
    fn schedule_pump(&self);
}

struct Forward(Arc<dyn Listener>);

impl Platform for Forward {
    fn commit(&self, dirty: &[u64], can_back: bool) {
        self.0
            .commit(dirty.iter().map(|id| id.cast_signed()).collect(), can_back);
    }

    fn schedule_pump(&self) {
        self.0.schedule_pump();
    }
}

#[uniffi::export]
pub fn noop() {}

#[uniffi::export]
pub fn echo(value: u64) -> u64 {
    value
}

#[uniffi::export]
pub fn row(id: u64) -> RowModel {
    RowModel {
        id,
        value: id.wrapping_mul(7),
        liked: id % 2 == 0,
    }
}

#[uniffi::export]
pub fn rows(start: u64, count: u32) -> Vec<RowModel> {
    (start..start + u64::from(count)).map(row).collect()
}

#[uniffi::export]
pub fn entries(count: u32) -> Vec<EntryModel> {
    (0..u64::from(count))
        .map(|key| EntryModel {
            key,
            kind: 1,
            node: key << 32 | key,
            phase: 2,
        })
        .collect()
}

#[uniffi::export]
pub fn dirty(count: u32) -> Vec<u64> {
    (0..u64::from(count)).collect()
}

#[uniffi::export]
pub fn article(id: u64) -> ArticleModel {
    let (title, body) = ffi_scenario::article_text(id);
    ArticleModel { id, title, body }
}

#[uniffi::export]
pub fn mount(listener: Arc<dyn Listener>) -> u64 {
    ffi_scenario::mount(Arc::new(Forward(listener)))
}

#[uniffi::export]
pub fn tap(node: u64) -> bool {
    ffi_scenario::tap(node)
}

#[uniffi::export]
pub fn value(node: u64) -> u64 {
    ffi_scenario::value(node)
}
