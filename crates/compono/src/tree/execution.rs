//! Runs queued work, up to a limit per operation, and reports one commit.

use super::{NodeSlot, OpenScope, Tree, Work};
use crate::router::Scope;
use crate::{Commit, NodeId};
use alloc::boxed::Box;
use core::any::Any;

/// How many queued messages and outputs one operation delivers at most.
///
/// Far above what a real cascade needs, and low enough that a component loop
/// gives the thread back within a fraction of a second.
pub(crate) const DEFAULT_WORK_LIMIT: usize = 1 << 20;

impl Tree {
    /// Sets how many units of work one operation does at most.
    ///
    /// Work a component queues with [`Cx::emit`](crate::Cx::emit) or
    /// [`Cx::send`](crate::Cx::send) is delivered before the operation commits,
    /// one unit per message or output. A router or
    /// [`Scope`](crate::router::Scope) that is dropped has its components
    /// removed before the operation commits, one unit per component.
    /// Components that keep answering each other, or that drop large routers,
    /// could otherwise hold the thread for as long as they like. Past the
    /// limit the operation commits what it has done, keeps the rest waiting in
    /// order and wakes the host, whose next [`Tree::pump`](crate::Tree::pump)
    /// or any other operation continues it. An entry the limit stops halfway
    /// stays hidden until the rest of it is removed.
    ///
    /// Polls of tasks started with [`Cx::perform_local`](crate::Cx::perform_local)
    /// have their own limit, set by `Tree::set_poll_limit`.
    ///
    /// The default is 1 048 576. A limit of 0 is treated as 1.
    pub fn set_work_limit(&mut self, limit: usize) {
        self.work_limit = limit.max(1);
    }

    pub(crate) fn finish(&mut self) {
        // Removing a dropped router's entries can queue work, and work can drop
        // more routers. Both share one budget, so the operation ends however
        // much is dropped.
        let mut budget = self.work_limit;
        let done = loop {
            if !self.bury(&mut budget) || !self.drain(&mut budget) {
                break false;
            }
            let waiting = self
                .graveyard
                .try_borrow()
                .is_ok_and(|buried| !buried.is_empty());
            if !waiting {
                break true;
            }
        };
        if !done {
            // The rest waits for the next operation, which the host is asked for.
            self.host.waker().wake();
        }
        let can_back = self
            .root
            .and_then(|root| self.slot(root))
            .is_some_and(NodeSlot::offers_back);
        let mut dirty = core::mem::take(&mut self.dirty);
        // A component removed during this operation is not reported.
        dirty.retain(|id| {
            self.slot_mut(*id).is_some_and(|slot| {
                slot.dirty = false;
                true
            })
        });
        if dirty.is_empty() && can_back == self.can_back {
            self.dirty = dirty;
            return;
        }
        self.can_back = can_back;
        self.host.commit(&Commit {
            dirty: &dirty,
            can_back,
        });
        dirty.clear();
        self.dirty = dirty;
    }

    /// Removes the entries of dropped scopes, oldest first, one unit of
    /// `budget` per component. Returns whether none is left waiting.
    ///
    /// An entry the budget stops halfway stays first in line, hidden, and the
    /// next call removes the rest of it.
    fn bury(&mut self, budget: &mut usize) -> bool {
        let mut batch = core::mem::take(&mut self.buried);
        if let Ok(mut graveyard) = self.graveyard.try_borrow_mut() {
            if batch.is_empty() {
                core::mem::swap(&mut *graveyard, &mut batch);
            } else {
                batch.append(&mut graveyard);
            }
        }
        let mut removed = 0;
        for (owner, id) in batch.iter().copied() {
            // An entry already destroyed is gone, or its slot holds another node.
            if self.parent(id) == Some(owner) && !self.destroy_within(id, budget) {
                break;
            }
            removed += 1;
        }
        batch.drain(..removed);
        let finished = batch.is_empty();
        self.buried = batch;
        finished
    }

    /// Returns the components queued for removal by dropped routers.
    #[cfg(feature = "serde")]
    pub(crate) fn leaving(&self) -> alloc::vec::Vec<NodeId> {
        let queued = self.graveyard.try_borrow();
        self.buried
            .iter()
            .chain(queued.iter().flat_map(|queued| queued.iter()))
            .map(|(_owner, id)| *id)
            .collect()
    }

    /// Delivers queued work while `budget` lasts. Returns whether the queue emptied.
    fn drain(&mut self, budget: &mut usize) -> bool {
        while *budget > 0 {
            let Some(work) = self.work.pop_front() else {
                return true;
            };
            *budget -= 1;
            match work {
                Work::Msg { to, payload } => {
                    self.with_node(to, |node, tree| node.accept(payload, tree, to));
                }
                Work::Output { from, payload } => self.deliver_output(from, payload),
            }
        }
        self.work.is_empty()
    }

    fn deliver_output(&mut self, from: NodeId, payload: Box<dyn Any>) {
        let Some(slot) = self.slot_mut(from) else {
            return;
        };
        let (Some(parent), Some(route)) = (slot.parent, slot.route.take()) else {
            return;
        };
        self.with_node(parent, |node, tree| {
            route(node.as_any_mut(), payload, tree, parent)
        });
        // The route is usually what removed the child; then it is dropped here.
        if let Some(slot) = self.slot_mut(from) {
            slot.route = Some(route);
        }
    }

    pub(crate) fn begin_scope(&mut self, owner: NodeId, base: u64) -> Option<OpenScope> {
        self.scope.replace(OpenScope {
            owner,
            base,
            next_child: 0,
        })
    }

    pub(crate) fn created_len(&self) -> usize {
        self.created.len()
    }

    pub(crate) fn end_scope(&mut self, previous: Option<OpenScope>, mark: usize) -> Scope {
        let ended = core::mem::replace(&mut self.scope, previous);
        let mut scope = match ended {
            Some(open) => Scope::new(open.owner, &self.graveyard),
            None => Scope::default(),
        };
        for id in self.created.drain(mark..) {
            scope.add(id);
        }
        scope
    }
}
