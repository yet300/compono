//! The `Dialog` component.

use compono::{Component, Cx};

/// Input to [`Dialog`].
#[derive(Clone, Copy, Debug)]
pub enum DialogMsg {
    Dismiss,
}

/// A message box.
#[derive(Debug)]
pub struct Dialog {
    /// The text to show.
    pub text: &'static str,
}

impl Component for Dialog {
    type Msg = DialogMsg;
    type Output = ();

    fn update(&mut self, msg: DialogMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            DialogMsg::Dismiss => cx.emit(()),
        }
    }

    fn back(&self) -> Option<DialogMsg> {
        Some(DialogMsg::Dismiss)
    }
}
