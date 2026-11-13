//! The first tab: links to other screens and a dialog in a `Slot`.

mod dialog;
pub use dialog::{Dialog, DialogMsg};

use compono::router::slot::Slot;
use compono::{Component, Cx, Handle};

/// Input to [`Menu`].
#[derive(Clone, Copy, Debug)]
pub enum MenuMsg {
    OpenBook,
    OpenGallery,
    ShowDialog,
}

/// What [`Menu`] asks of its owner.
#[derive(Clone, Copy, Debug)]
pub enum MenuOutput {
    OpenBook,
    OpenGallery,
}

/// The menu screen.
#[derive(Debug)]
pub struct Menu {
    /// The open dialog.
    pub dialog: Slot<&'static str, Handle<Dialog>, Self>,
}

impl Menu {
    /// Creates the menu.
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        let dialog = Slot::new(cx, |text: &&'static str, cx| {
            let text = *text;
            cx.child(move |_cx| Dialog { text }, Self::on_dialog)
        });
        Self { dialog }
    }

    fn on_dialog(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.dialog.close(cx);
    }
}

impl Component for Menu {
    type Msg = MenuMsg;
    type Output = MenuOutput;

    fn update(&mut self, msg: MenuMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            MenuMsg::OpenBook => cx.emit(MenuOutput::OpenBook),
            MenuMsg::OpenGallery => cx.emit(MenuOutput::OpenGallery),
            MenuMsg::ShowDialog => {
                self.dialog.show("Slots hold zero or one child.", cx);
            }
        }
    }
}
