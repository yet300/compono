//! The `Card` component.

use super::TICK;
use crate::Services;
use compono::{Component, Cx, Phase, TaskId};

/// Input to [`Card`].
#[derive(Clone, Copy, Debug)]
pub enum CardMsg {
    Tick,
}

/// One card. Its counter runs only while it is on top.
#[derive(Debug)]
pub struct Card {
    /// Its number.
    pub number: u32,
    /// Ticks while focused.
    pub value: u64,
    pub(super) timer: Option<TaskId>,
    pub(super) services: Services,
}

impl Component for Card {
    type Msg = CardMsg;
    type Output = ();

    fn update(&mut self, msg: CardMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            CardMsg::Tick => {
                self.value += 1;
                let sleep = self.services.sleep(TICK);
                self.timer = cx.perform(sleep, |()| CardMsg::Tick).ok();
            }
        }
    }

    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        cx.unchanged();
        if phase == Phase::Focused {
            let sleep = self.services.sleep(TICK);
            self.timer = cx.perform(sleep, |()| CardMsg::Tick).ok();
        } else if let Some(task) = self.timer.take() {
            cx.cancel(task);
        }
    }
}
