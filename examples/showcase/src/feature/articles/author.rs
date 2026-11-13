//! The `AuthorPanel` component.

use super::data::Author;
use compono::{Component, Cx};

/// Input to [`AuthorPanel`].
#[derive(Clone, Copy, Debug)]
pub enum AuthorMsg {
    Close,
}

/// An article's author.
#[derive(Debug)]
pub struct AuthorPanel {
    /// The author.
    pub author: Author,
}

impl Component for AuthorPanel {
    type Msg = AuthorMsg;
    type Output = ();

    fn update(&mut self, msg: AuthorMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            AuthorMsg::Close => cx.emit(()),
        }
    }
}
