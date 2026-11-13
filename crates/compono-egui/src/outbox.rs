use compono::{Component, Handle, Tree};

type Action = Box<dyn FnOnce(&mut Tree)>;

/// Changes requested by views during a frame.
#[derive(Default)]
pub struct Outbox {
    actions: Vec<Action>,
}

impl std::fmt::Debug for Outbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outbox")
            .field("queued", &self.actions.len())
            .finish()
    }
}

impl Outbox {
    /// Queues a message. A component removed before the flush ignores it.
    pub fn send<C: Component>(&mut self, target: Handle<C>, msg: C::Msg) {
        self.actions.push(Box::new(move |tree| {
            let _stale_is_fine = tree.send(target, msg);
        }));
    }

    /// Queues a Back press.
    pub fn back(&mut self) {
        self.actions.push(Box::new(|tree| {
            let _handled = tree.back();
        }));
    }

    /// Applies queued changes in the order they were requested.
    pub fn flush(&mut self, tree: &mut Tree) {
        for action in self.actions.drain(..) {
            action(tree);
        }
    }
}
