//! The synchronisation primitives, swapped for loom's model-checked ones under `cfg(loom)`.

#[cfg(not(loom))]
pub(crate) use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(loom)]
pub(crate) use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(loom)]
pub(crate) use loom::sync::{Mutex, MutexGuard};
#[cfg(not(loom))]
pub(crate) use std::sync::{Mutex, MutexGuard};
