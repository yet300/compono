//! The `Page` component.

use compono::{Component, Cx};

/// One page.
#[derive(Debug)]
pub struct Page {
    /// Its number.
    pub number: u32,
    /// How many times it was tapped.
    pub taps: u32,
}

/// Input to [`Page`].
#[derive(Clone, Copy, Debug)]
pub enum PageMsg {
    Tap,
}

impl Component for Page {
    type Msg = PageMsg;
    type Output = ();

    fn update(&mut self, msg: PageMsg, _cx: &mut Cx<'_, Self>) {
        match msg {
            PageMsg::Tap => self.taps += 1,
        }
    }
}
