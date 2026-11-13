//! Intrusive lists of a component's children, and of those it shows, which make
//! removing a child constant time.

use super::Tree;
use crate::NodeId;

impl Tree {
    pub(super) fn link_child(&mut self, parent: NodeId, id: NodeId) {
        let Some(parent_slot) = self.slot_mut(parent) else {
            return;
        };
        let previous = parent_slot.last_child.replace(id);
        if parent_slot.first_child.is_none() {
            parent_slot.first_child = Some(id);
        }
        if let Some(slot) = self.slot_mut(id) {
            slot.prev_sibling = previous;
        }
        if let Some(slot) = previous.and_then(|previous| self.slot_mut(previous)) {
            slot.next_sibling = Some(id);
        }
    }

    /// Removes a child from its parent's list in constant time.
    pub(super) fn unlink_child(
        &mut self,
        parent: NodeId,
        prev: Option<NodeId>,
        next: Option<NodeId>,
    ) {
        if let Some(slot) = prev.and_then(|prev| self.slot_mut(prev)) {
            slot.next_sibling = next;
        }
        if let Some(slot) = next.and_then(|next| self.slot_mut(next)) {
            slot.prev_sibling = prev;
        }
        if let Some(slot) = self.slot_mut(parent) {
            if prev.is_none() {
                slot.first_child = next;
            }
            if next.is_none() {
                slot.last_child = prev;
            }
        }
    }

    pub(super) fn link_visible(&mut self, parent: NodeId, id: NodeId) {
        let Some(parent_slot) = self.slot_mut(parent) else {
            return;
        };
        let previous = parent_slot.last_visible.replace(id);
        if previous.is_none() {
            parent_slot.first_visible = Some(id);
        }
        if let Some(slot) = self.slot_mut(id) {
            slot.prev_visible = previous;
            slot.next_visible = None;
        }
        if let Some(slot) = previous.and_then(|previous| self.slot_mut(previous)) {
            slot.next_visible = Some(id);
        }
    }

    pub(super) fn unlink_visible(&mut self, parent: NodeId, id: NodeId) {
        let Some(slot) = self.slot_mut(id) else {
            return;
        };
        let (prev, next) = (slot.prev_visible.take(), slot.next_visible.take());
        match prev.and_then(|prev| self.slot_mut(prev)) {
            Some(slot) => slot.next_visible = next,
            None => {
                if let Some(slot) = self.slot_mut(parent) {
                    slot.first_visible = next;
                }
            }
        }
        match next.and_then(|next| self.slot_mut(next)) {
            Some(slot) => slot.prev_visible = prev,
            None => {
                if let Some(slot) = self.slot_mut(parent) {
                    slot.last_visible = prev;
                }
            }
        }
    }
}
