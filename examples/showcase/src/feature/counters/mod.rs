//! A deep stack of counters. Only the focused counter ticks.

mod counter;
pub use counter::{Counter, CounterMsg, CounterOutput};

use crate::Services;
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist};
use std::time::Duration;

const TICK: Duration = Duration::from_millis(250);
/// How many screens the "go deep" button pushes.
pub const DEEP_PUSH: usize = 1_000;

/// Input to [`Counters`].
#[derive(Clone, Copy, Debug)]
pub enum CountersMsg {
    GoDeep,
    PopToRoot,
    Back,
}

/// The counters tab.
#[derive(Debug)]
pub struct Counters {
    /// The screens.
    pub stack: Stack<usize, Handle<Counter>, Self>,
}

impl Counters {
    /// Creates the tab, resuming saved navigation if there is any.
    pub fn new(cx: &mut Cx<'_, Self>, services: Services) -> Self {
        let saved = cx.restored();
        // A name keeps the saved stack its own if routers are added before it.
        let stack = cx.named_routers("counters", |cx| {
            Stack::restore_or(saved, 0, cx, move |index, cx| {
                let (index, services) = (*index, services.clone());
                cx.persistent_child(
                    move |cx| Counter::new(cx, index, services),
                    Self::on_counter,
                )
            })
        });
        Self { stack }
    }

    fn on_counter(&mut self, output: CounterOutput, cx: &mut Cx<'_, Self>) {
        match output {
            CounterOutput::Next => self.stack.push(self.stack.depth(), cx),
            CounterOutput::Previous => {
                let _at_root = self.stack.pop(cx);
            }
        }
    }
}

impl Component for Counters {
    type Msg = CountersMsg;
    type Output = ();

    fn update(&mut self, msg: CountersMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            CountersMsg::GoDeep => {
                for _ in 0..DEEP_PUSH {
                    self.stack.push(self.stack.depth(), cx);
                }
            }
            CountersMsg::PopToRoot => {
                self.stack.pop_to(1, cx);
            }
            CountersMsg::Back => {
                let _at_root = self.stack.pop(cx);
            }
        }
    }

    fn back(&self) -> Option<CountersMsg> {
        self.stack.can_pop().then_some(CountersMsg::Back)
    }
}

impl Persist for Counters {
    type Saved = SavedStack<usize>;
    const KIND: &'static str = "showcase.counters";

    fn save(&self) -> Self::Saved {
        self.stack.snapshot()
    }
}
