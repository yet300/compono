//! The work both bindings export, so that only the boundary differs.

use compono::{Component, Cx, NodeId};
use compono_ffi::{NativeTree, Platform};
use core::cell::RefCell;
use std::sync::Arc;

struct Counter(u64);

impl Component for Counter {
    type Msg = u64;
    type Output = ();

    fn update(&mut self, msg: u64, _cx: &mut Cx<'_, Self>) {
        self.0 = self.0.wrapping_add(msg);
    }
}

thread_local! {
    static TREE: RefCell<Option<NativeTree>> = const { RefCell::new(None) };
}

/// Creates the tree on this thread and returns the counter's address.
///
/// The first address is 0. If mounting fails, a later [`tap`] returns false.
#[must_use]
pub fn mount(platform: Arc<dyn Platform>) -> u64 {
    let tree = NativeTree::new(platform);
    let Ok(counter) = tree.with(|tree| tree.mount(|_cx| Counter(0))) else {
        return 0;
    };
    TREE.with_borrow_mut(|slot| *slot = Some(tree));
    counter.id().to_bits()
}

/// Sends one message through the tree. The platform hears one commit.
#[must_use]
pub fn tap(node: u64) -> bool {
    TREE.with_borrow(|slot| {
        slot.as_ref()
            .is_some_and(|tree| tree.send::<Counter>(node, 1).is_ok())
    })
}

/// Reads the counter's model.
#[must_use]
pub fn value(node: u64) -> u64 {
    TREE.with_borrow(|slot| {
        slot.as_ref()
            .and_then(|tree| {
                tree.read(|tree| {
                    let counter = tree.handle_of::<Counter>(NodeId::from_bits(node))?;
                    tree.get(counter).map(|counter| counter.0)
                })
                .ok()
                .flatten()
            })
            .unwrap_or(0)
    })
}

/// The text of a screen model of typical size.
#[must_use]
pub fn article_text(id: u64) -> (String, String) {
    (
        format!("Article {id}: router"),
        format!("Body of article {id}. ").repeat(12),
    )
}
