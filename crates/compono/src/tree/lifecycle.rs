//! Phase propagation through the tree.

use super::Tree;
use crate::{NodeId, Phase};

impl Tree {
    pub(crate) fn assign(&mut self, id: NodeId, phase: Phase) {
        let Some(slot) = self.slot_mut(id) else {
            return;
        };
        let previous = core::mem::replace(&mut slot.assigned, phase);
        // Following the grant, not the effective phase, keeps the order of
        // siblings when a whole subtree is hidden and shown again.
        self.update_visible_list(id, previous, phase);
        self.refresh(id);
    }

    /// Brings a component and everything below it to the phase they should have.
    ///
    /// The walk uses the tree's own links and no call stack, so its depth is not
    /// limited. A subtree only ever moves one way: raising a parent can only
    /// raise its children, and lowering it can only lower them.
    ///
    /// Only children granted `Visible` or above are visited. A child granted
    /// `Hidden` stays hidden whatever its parent does, and so does everything
    /// below it, so the cost follows what is shown and not how many children
    /// there are.
    pub(crate) fn refresh(&mut self, id: NodeId) {
        match self.pending_phase(id) {
            Some((current, target)) if target > current => self.raise(id),
            Some(_) => self.lower(id),
            None => {}
        }
    }

    /// Returns the effective phase and the one it should have, if they differ.
    fn pending_phase(&self, id: NodeId) -> Option<(Phase, Phase)> {
        let slot = self.slot(id)?;
        let ceiling = match slot.parent {
            Some(parent) => self
                .slot(parent)
                .map_or(Phase::Hidden, |parent| parent.effective),
            None => self.root_phase,
        };
        let target = slot.assigned.min(ceiling);
        (target != slot.effective).then_some((slot.effective, target))
    }

    fn apply_phase(&mut self, id: NodeId, target: Phase) {
        self.change_back(id, |slot| slot.effective = target);
        if target < Phase::Visible {
            self.end_gesture_of(id);
        }
    }

    /// Raises parents before children, and children in the order they were shown.
    fn raise(&mut self, root: NodeId) {
        let mut cursor = root;
        loop {
            let mut below = None;
            if let Some((_current, target)) = self.pending_phase(cursor) {
                self.apply_phase(cursor, target);
                self.notify_phase(cursor, target);
                below = self.slot(cursor).and_then(|slot| slot.first_visible);
            }
            if let Some(child) = below {
                cursor = child;
                continue;
            }
            loop {
                if cursor == root {
                    return;
                }
                let Some(slot) = self.slot(cursor) else {
                    return;
                };
                if let Some(next) = slot.next_visible {
                    cursor = next;
                    break;
                }
                let Some(parent) = slot.parent else {
                    return;
                };
                cursor = parent;
            }
        }
    }

    /// Lowers children before parents, and the child shown most recently first.
    fn lower(&mut self, root: NodeId) {
        let mut cursor = root;
        'walk: loop {
            if let Some((_current, target)) = self.pending_phase(cursor) {
                // The new phase is set before the children are visited, because
                // it is their ceiling. The component hears of it after them.
                self.apply_phase(cursor, target);
                if let Some(child) = self.slot(cursor).and_then(|slot| slot.last_visible) {
                    cursor = child;
                    continue;
                }
                self.notify_phase(cursor, target);
            }
            loop {
                if cursor == root {
                    return;
                }
                let Some(slot) = self.slot(cursor) else {
                    return;
                };
                if let Some(previous) = slot.prev_visible {
                    cursor = previous;
                    continue 'walk;
                }
                let Some(parent) = slot.parent else {
                    return;
                };
                cursor = parent;
                let phase = self.phase_of(cursor);
                self.notify_phase(cursor, phase);
            }
        }
    }

    fn update_visible_list(&mut self, id: NodeId, from: Phase, to: Phase) {
        let was_visible = from >= Phase::Visible;
        let is_visible = to >= Phase::Visible;
        if was_visible == is_visible {
            return;
        }
        let Some(parent) = self.parent(id) else {
            return;
        };
        if is_visible {
            self.link_visible(parent, id);
        } else {
            self.unlink_visible(parent, id);
        }
    }

    fn notify_phase(&mut self, id: NodeId, phase: Phase) {
        self.with_node(id, |node, tree| node.phase_changed(phase, tree, id));
    }
}
