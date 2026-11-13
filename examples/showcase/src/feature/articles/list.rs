//! The `ArticleList` component.

use compono::{Component, Cx};

/// Input to [`ArticleList`].
#[derive(Clone, Copy, Debug)]
pub enum ListMsg {
    Select(u32),
}

/// The list of articles.
#[derive(Debug)]
pub struct ArticleList {
    /// The article shown in details.
    pub selected: Option<u32>,
}

impl Component for ArticleList {
    type Msg = ListMsg;
    type Output = u32;

    fn update(&mut self, msg: ListMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ListMsg::Select(id) => {
                self.selected = Some(id);
                cx.emit(id);
            }
        }
    }
}
