//! Back handling and the predictive gesture.

use super::{NodeSlot, Tree};
use crate::{NodeId, Phase};

impl Tree {
    /// Runs Back on the deepest visible component that handles it.
    pub fn back(&mut self) -> bool {
        self.operate(false, |tree| tree.back_now())
    }

    fn back_now(&mut self) -> bool {
        let ended = self.gesture.take();
        if let Some(target) = ended {
            // The UI must learn the gesture is over even if Back changes nothing.
            self.mark_dirty(target);
        }
        let Some(target) = self.root.and_then(|root| self.back_target(root)) else {
            if ended.is_some() {
                self.finish();
            }
            return false;
        };
        self.with_node(target, |node, tree| node.run_back(tree, target));
        self.finish();
        true
    }

    /// Starts a predictive gesture. Returns whether anything would handle it.
    pub fn back_start(&mut self) -> bool {
        self.operate(false, |tree| tree.back_start_now())
    }

    fn back_start_now(&mut self) -> bool {
        let Some(target) = self.root.and_then(|root| self.back_target(root)) else {
            return false;
        };
        self.gesture = Some(target);
        self.mark_dirty(target);
        self.finish();
        true
    }

    /// Abandons the gesture without navigating.
    pub fn back_cancel(&mut self) {
        self.operate((), |tree| tree.back_cancel_now())
    }

    fn back_cancel_now(&mut self) {
        if let Some(target) = self.gesture.take() {
            self.mark_dirty(target);
            self.finish();
        }
    }

    /// Completes the gesture. If its target has gone, Back is resolved afresh.
    pub fn back_commit(&mut self) -> bool {
        self.operate(false, Self::back_now)
    }

    /// Returns the component that handles Back if the gesture in progress
    /// commits, or `None` when no gesture is in progress.
    #[must_use]
    pub const fn gesture_target(&self) -> Option<NodeId> {
        self.gesture
    }

    /// Ends the gesture if `id` was its target and can no longer handle Back.
    pub(super) fn end_gesture_of(&mut self, id: NodeId) {
        if self.gesture == Some(id) {
            self.gesture = None;
            self.mark_dirty(id);
        }
    }

    /// Finds the deepest handler, taking the most recently shown child at each level.
    ///
    /// A child that offers Back has a handler at or below it, so the walk never
    /// has to return to a level it left.
    pub(super) fn back_target(&self, id: NodeId) -> Option<NodeId> {
        let mut slot = self.slot(id).filter(|slot| slot.offers_back())?;
        let mut target = id;
        loop {
            let mut cursor = slot.last_visible;
            let mut deeper = None;
            while let Some(child) = cursor {
                let Some(child_slot) = self.slot(child) else {
                    break;
                };
                if child_slot.offers_back() {
                    deeper = Some((child, child_slot));
                    break;
                }
                cursor = child_slot.prev_visible;
            }
            match deeper {
                Some((child, child_slot)) => {
                    target = child;
                    slot = child_slot;
                }
                None => return slot.own_back.then_some(target),
            }
        }
    }

    /// Applies `change` and updates the handler counts of affected ancestors.
    ///
    /// Keeping counts makes `can_back` constant time, however many components
    /// are visible.
    pub(super) fn change_back(&mut self, id: NodeId, change: impl FnOnce(&mut NodeSlot)) {
        let Some(slot) = self.slot_mut(id) else {
            return;
        };
        let before = slot.offers_back();
        change(slot);
        let after = slot.offers_back();
        let mut parent = slot.parent.filter(|_| before != after);
        while let Some(ancestor) = parent {
            let Some(slot) = self.slot_mut(ancestor) else {
                return;
            };
            let before = slot.offers_back();
            slot.live_back = if after {
                slot.live_back.saturating_add(1)
            } else {
                slot.live_back.saturating_sub(1)
            };
            parent = slot.parent.filter(|_| before != slot.offers_back());
        }
    }
}

impl NodeSlot {
    /// Returns whether Back would be handled at or below this visible component.
    pub(super) fn offers_back(&self) -> bool {
        self.effective >= Phase::Visible && (self.own_back || self.live_back > 0)
    }
}
