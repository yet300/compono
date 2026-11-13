//! The `Photo` component.

use super::TICK;
use crate::Services;
use compono::{Component, Cx, Phase, TaskId};

/// Input to [`Photo`].
#[derive(Clone, Copy, Debug)]
pub enum PhotoMsg {
    Tick,
}

/// One frame. It develops only while on screen.
#[derive(Debug)]
pub struct Photo {
    /// Its number.
    pub number: u64,
    /// Grows while the photo is visible.
    pub developed: u64,
    pub(super) timer: Option<TaskId>,
    pub(super) services: Services,
}

impl Photo {
    fn arm(&mut self, cx: &mut Cx<'_, Self>) {
        let sleep = self.services.sleep(TICK);
        self.timer = cx.perform(sleep, |()| PhotoMsg::Tick).ok();
    }
}

impl Component for Photo {
    type Msg = PhotoMsg;
    type Output = ();

    fn update(&mut self, msg: PhotoMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            PhotoMsg::Tick => {
                self.developed += 1;
                self.arm(cx);
            }
        }
    }

    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        cx.unchanged();
        if let Some(task) = self.timer.take() {
            cx.cancel(task);
        }
        if phase >= Phase::Visible {
            self.arm(cx);
        }
    }
}
