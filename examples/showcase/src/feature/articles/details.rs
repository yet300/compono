//! The `ArticleDetails` component.

use super::data::Article;
use compono::{Component, Cx};

/// Input to [`ArticleDetails`].
#[derive(Clone, Copy, Debug)]
pub enum DetailsMsg {
    ShowAuthor,
    Close,
}

/// What [`ArticleDetails`] asks of its owner.
#[derive(Clone, Copy, Debug)]
pub enum DetailsOutput {
    ShowAuthor(u32),
    Close,
}

/// One article.
#[derive(Debug)]
pub struct ArticleDetails {
    /// The article.
    pub article: Article,
}

impl Component for ArticleDetails {
    type Msg = DetailsMsg;
    type Output = DetailsOutput;

    fn update(&mut self, msg: DetailsMsg, cx: &mut Cx<'_, Self>) {
        cx.emit(match msg {
            DetailsMsg::ShowAuthor => DetailsOutput::ShowAuthor(self.article.author),
            DetailsMsg::Close => DetailsOutput::Close,
        });
    }
}
