//! A list, its details and the author, in one, two or three panels.

mod author;
mod data;
mod details;
mod list;
pub use author::{AuthorMsg, AuthorPanel};
pub use details::{ArticleDetails, DetailsMsg, DetailsOutput};
pub use list::{ArticleList, ListMsg};

pub use data::{Article, Author, article, author};

use compono::router::panels::{PanelTypes, Panels, PanelsMode};
use compono::{Component, Cx, Handle};

/// How many articles the list has.
pub const ARTICLES: u32 = 1_000;

/// The panels of this screen.
#[derive(Debug)]
pub struct ArticlePanels;

impl PanelTypes for ArticlePanels {
    type Main = ();
    type MainChild = Handle<ArticleList>;
    type Details = u32;
    type DetailsChild = Handle<ArticleDetails>;
    type Extra = u32;
    type ExtraChild = Handle<AuthorPanel>;
}

/// Input to [`Articles`].
#[derive(Clone, Copy, Debug)]
pub enum ArticlesMsg {
    SetMode(PanelsMode),
    Back,
}

/// The articles tab.
#[derive(Debug)]
pub struct Articles {
    /// The panels.
    pub panels: Panels<ArticlePanels, Self>,
}

impl Articles {
    /// Creates the tab with only the list open.
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        let panels = Panels::new(
            (),
            PanelsMode::Dual,
            cx,
            |(), cx| cx.child(|_cx| ArticleList { selected: None }, Self::on_list),
            |id, cx| {
                let article = article(*id);
                cx.child(move |_cx| ArticleDetails { article }, Self::on_details)
            },
            |id, cx| {
                let author = author(*id);
                cx.child(move |_cx| AuthorPanel { author }, Self::on_author)
            },
        );
        Self { panels }
    }

    fn on_list(&mut self, id: u32, cx: &mut Cx<'_, Self>) {
        self.panels.set_details(Some(id), cx);
    }

    fn on_details(&mut self, output: DetailsOutput, cx: &mut Cx<'_, Self>) {
        match output {
            DetailsOutput::ShowAuthor(id) => self.panels.set_extra(Some(id), cx),
            DetailsOutput::Close => self.panels.set_details(None, cx),
        }
    }

    fn on_author(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.panels.set_extra(None, cx);
    }
}

impl Component for Articles {
    type Msg = ArticlesMsg;
    type Output = ();

    fn update(&mut self, msg: ArticlesMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ArticlesMsg::SetMode(mode) => self.panels.set_mode(mode, cx),
            ArticlesMsg::Back => {
                self.panels.pop(cx);
            }
        }
    }

    fn back(&self) -> Option<ArticlesMsg> {
        self.panels.can_pop().then_some(ArticlesMsg::Back)
    }
}
