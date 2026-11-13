//! The `Counter` component.

use super::TICK;
use crate::Services;
use compono::{Component, Cx, Persist, Phase, TaskId};

/// Input to [`Counter`].
#[derive(Clone, Copy, Debug)]
pub enum CounterMsg {
    Tick,
    Next,
    Previous,
}

/// What [`Counter`] asks of its owner.
#[derive(Clone, Copy, Debug)]
pub enum CounterOutput {
    Next,
    Previous,
}

/// One screen in the stack.
#[derive(Debug)]
pub struct Counter {
    /// Its position in the stack.
    pub index: usize,
    /// Ticks while focused. Survives a restart.
    pub value: u64,
    timer: Option<TaskId>,
    services: Services,
}

impl Counter {
    pub(super) fn new(cx: &mut Cx<'_, Self>, index: usize, services: Services) -> Self {
        Self {
            index,
            value: cx.restored().unwrap_or(0),
            timer: None,
            services,
        }
    }

    fn arm(&mut self, cx: &mut Cx<'_, Self>) {
        let sleep = self.services.sleep(TICK);
        self.timer = cx.perform(sleep, |()| CounterMsg::Tick).ok();
    }
}

impl Component for Counter {
    type Msg = CounterMsg;
    type Output = CounterOutput;

    fn update(&mut self, msg: CounterMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            CounterMsg::Tick => {
                self.value += 1;
                self.arm(cx);
            }
            CounterMsg::Next => cx.emit(CounterOutput::Next),
            CounterMsg::Previous => cx.emit(CounterOutput::Previous),
        }
    }

    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        cx.unchanged();
        if phase == Phase::Focused {
            self.arm(cx);
        } else if let Some(task) = self.timer.take() {
            cx.cancel(task);
        }
    }
}

impl Persist for Counter {
    type Saved = u64;
    const KIND: &'static str = "showcase.counter";

    fn save(&self) -> u64 {
        self.value
    }
}
