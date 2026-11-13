//! The layout of a saved key.
//!
//! Saved state finds a component by its path: one `u64` key for each level from
//! the root. The high bits of a key say what it identifies:
//!
//! | High bits | Rest | Identifies |
//! |---|---|---|
//! | `0` | router number (15), entry (48) | an entry of a numbered router; number `0x7fff` saves nothing |
//! | `100` | counter (61) | a numbered child |
//! | `101` | name (29), entry (32) | an entry of a named router; an entry key above 32 bits saves nothing |
//! | `11` | name (62) | a named child |
//!
//! A name is the top bits of its FNV-1a hash, the same on every launch, platform
//! and build. A numbered child past the 61-bit counter saves nothing.
//!
//! These keys are part of the saved-state format, so none of them may change.

use crate::tree::{FNV_OFFSET, fnv1a};

const ENTRY_MASK: u64 = (1 << ROUTER_SHIFT) - 1;
pub(crate) const ROUTER_SHIFT: u32 = 48;
/// The router number that saves nothing. Every numbered router a component
/// makes past the numbers below it gets this one.
pub(crate) const UNSAVED_ROUTER: u16 = 0x7fff;
/// A key in the range that is never saved, so it cannot take or overwrite
/// another component's state.
pub(crate) const UNSAVED: u64 = (UNSAVED_ROUTER as u64) << ROUTER_SHIFT;
const NAMED_ROUTER_TAG: u64 = 0b101 << 61;
const TAG_MASK: u64 = 0b111 << 61;
pub(crate) const NAME_SHIFT: u32 = 32;
/// A named router saves entries whose keys fit in 32 bits.
const NAMED_ENTRY_MASK: u64 = (1 << NAME_SHIFT) - 1;
const NUMBERED_CHILD_TAG: u64 = 1 << 63;
/// The first number a numbered child cannot have, where named routers begin.
pub(crate) const NUMBERED_CHILD_LIMIT: u64 = 1 << 61;
const NAMED_CHILD_TAG: u64 = 0b11 << 62;

/// Returns the bits above the entry in every key of the `number`th numbered
/// router. A number past the last saved one gives the router that saves nothing.
pub(crate) const fn numbered_router(number: u16) -> u64 {
    let number = if number >= UNSAVED_ROUTER {
        UNSAVED_ROUTER
    } else {
        number
    };
    (number as u64) << ROUTER_SHIFT
}

/// Returns the bits above the entry in every key of the router named by the
/// FNV-1a `hash` of its name.
pub(crate) const fn named_router(hash: u64) -> u64 {
    NAMED_ROUTER_TAG | ((hash >> (64 - 29)) << NAME_SHIFT)
}

/// Returns whether `router`, from [`numbered_router`] or [`named_router`], is named.
pub(crate) const fn is_named_router(router: u64) -> bool {
    router & TAG_MASK == NAMED_ROUTER_TAG
}

/// Returns the key of the entry `entry` of `router`.
///
/// An entry of a named router whose key does not fit in 32 bits saves nothing.
pub(crate) const fn router_entry(router: u64, entry: u64) -> u64 {
    if !is_named_router(router) {
        router | (entry & ENTRY_MASK)
    } else if entry > NAMED_ENTRY_MASK {
        UNSAVED
    } else {
        router | entry
    }
}

/// Returns the lowest and highest key an entry of `router` can have.
#[cfg(any(feature = "serde", test))]
pub(crate) const fn router_entries(router: u64) -> (u64, u64) {
    if is_named_router(router) {
        (router, router | NAMED_ENTRY_MASK)
    } else {
        (router_entry(router, 0), router_entry(router, ENTRY_MASK))
    }
}

/// Returns whether a component with this key belongs to an entry of a router
/// that saves nothing, so neither it nor anything below it is saved.
#[cfg(feature = "serde")]
pub(crate) const fn is_unsaved_entry(key: u64) -> bool {
    key >> 63 == 0 && (key >> ROUTER_SHIFT) as u16 & UNSAVED_ROUTER == UNSAVED_ROUTER
}

/// Returns the key of the `number`th numbered child of a parent. A number past
/// the counter saves nothing, so it never reaches the keys of named routers.
pub(crate) const fn numbered_child(number: u64) -> u64 {
    if number < NUMBERED_CHILD_LIMIT {
        NUMBERED_CHILD_TAG | number
    } else {
        UNSAVED
    }
}

/// Returns the key of a child named `name`.
pub(crate) const fn named_child(name: &str) -> u64 {
    NAMED_CHILD_TAG | (fnv1a(FNV_OFFSET, name.as_bytes()) >> 2)
}

#[cfg(test)]
mod tests {
    use super::{
        NUMBERED_CHILD_LIMIT, UNSAVED, named_child, named_router, numbered_child, numbered_router,
        router_entries, router_entry,
    };
    use crate::tree::{FNV_OFFSET, fnv1a};

    #[test]
    fn named_routers_have_keys_no_child_and_no_numbered_router_can_have() {
        let numbered_children = numbered_child(0)..=numbered_child(NUMBERED_CHILD_LIMIT - 1);
        for name in ["", "tabs", "dialog", "a much longer name of a router"] {
            let (first, last) = router_entries(named_router(fnv1a(FNV_OFFSET, name.as_bytes())));
            for key in [first, last] {
                assert!(!numbered_children.contains(&key), "{name}: {key:#x}");
                assert_eq!(key >> 62, 0b10, "{name}: above the router entries");
                assert_ne!(
                    key >> 62,
                    named_child(name) >> 62,
                    "{name}: below named children"
                );
            }
            for number in [0, 1, 0x7ffe, 0x7fff] {
                let (low, high) = router_entries(numbered_router(number));
                assert!(high < first, "{name}: numbered router {number}");
                assert!(low >> 63 == 0);
            }
        }
        // An entry key too large for a named router saves nothing.
        let tabs = named_router(fnv1a(FNV_OFFSET, b"tabs"));
        assert_eq!(router_entry(tabs, 1 << 32), UNSAVED);
    }
}
