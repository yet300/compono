//! A list, its details and the author, in one, two or three panels.

use crate::component::{ChildRef, Ctx, address};
use boltffi::{data, export};
use compono::NodeId;
use compono::router::panels::{Panel, PanelsMode};
use showcase::feature::articles::{
    ARTICLES, ArticleDetails, ArticleList, Articles, ArticlesMsg, AuthorMsg, AuthorPanel,
    DetailsMsg, ListMsg, article,
};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Main,
    Details,
    Extra,
}

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanelsLayout {
    Single,
    Dual,
    Triple,
}

/// One visible panel.
#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PanelChild {
    pub kind: PanelKind,
    pub child: ChildRef,
}

#[data]
#[derive(Clone, PartialEq, Eq)]
pub struct ArticleText {
    pub title: String,
    pub body: String,
}

#[export]
pub struct ArticlesComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl ArticlesComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn layout(&self) -> Option<PanelsLayout> {
        self.ctx
            .read::<Articles, _>(self.node, |articles| match articles.panels.mode() {
                PanelsMode::Single => PanelsLayout::Single,
                PanelsMode::Dual => PanelsLayout::Dual,
                PanelsMode::Triple => PanelsLayout::Triple,
            })
    }

    /// Tells how many panels fit, as a window size class does.
    pub fn set_layout(&self, layout: PanelsLayout) {
        let mode = match layout {
            PanelsLayout::Single => PanelsMode::Single,
            PanelsLayout::Dual => PanelsMode::Dual,
            PanelsLayout::Triple => PanelsMode::Triple,
        };
        self.ctx
            .send::<Articles>(self.node, ArticlesMsg::SetMode(mode));
    }

    /// The panels to show, from the main one to the deepest.
    pub fn panels(&self) -> Vec<PanelChild> {
        let shown: Vec<(PanelKind, NodeId)> = self
            .ctx
            .read::<Articles, _>(self.node, |articles| {
                articles
                    .panels
                    .visible()
                    .map(|panel| match panel {
                        Panel::Main(list) => (PanelKind::Main, list.id()),
                        Panel::Details(details) => (PanelKind::Details, details.id()),
                        Panel::Extra(author) => (PanelKind::Extra, author.id()),
                    })
                    .collect()
            })
            .unwrap_or_default();
        shown
            .into_iter()
            .filter_map(|(kind, id)| {
                let key = match kind {
                    PanelKind::Main => 0,
                    PanelKind::Details => 1,
                    PanelKind::Extra => 2,
                };
                Some(PanelChild {
                    kind,
                    child: self.ctx.child(key, id)?,
                })
            })
            .collect()
    }

    pub fn list(&self, panel: PanelChild) -> Option<ArticleListComponent> {
        (panel.kind == PanelKind::Main).then(|| ArticleListComponent {
            ctx: self.ctx.clone(),
            node: address(panel.child),
        })
    }

    pub fn details(&self, panel: PanelChild) -> Option<ArticleDetailsComponent> {
        (panel.kind == PanelKind::Details).then(|| ArticleDetailsComponent {
            ctx: self.ctx.clone(),
            node: address(panel.child),
        })
    }

    pub fn author(&self, panel: PanelChild) -> Option<AuthorComponent> {
        (panel.kind == PanelKind::Extra).then(|| AuthorComponent {
            ctx: self.ctx.clone(),
            node: address(panel.child),
        })
    }
}

#[export]
pub struct ArticleListComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl ArticleListComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn count(&self) -> i32 {
        i32::try_from(ARTICLES).unwrap_or(i32::MAX)
    }

    pub fn title(&self, id: i32) -> String {
        article(u32::try_from(id).unwrap_or(0)).title
    }

    /// The selected article, or -1.
    pub fn selected(&self) -> i32 {
        self.ctx
            .read::<ArticleList, _>(self.node, |list| list.selected)
            .flatten()
            .and_then(|id| i32::try_from(id).ok())
            .unwrap_or(-1)
    }

    pub fn select(&self, id: i32) {
        if let Ok(id) = u32::try_from(id) {
            self.ctx.send::<ArticleList>(self.node, ListMsg::Select(id));
        }
    }
}

#[export]
pub struct ArticleDetailsComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl ArticleDetailsComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn text(&self) -> Option<ArticleText> {
        self.ctx
            .read::<ArticleDetails, _>(self.node, |details| ArticleText {
                title: details.article.title.clone(),
                body: details.article.body.clone(),
            })
    }

    pub fn show_author(&self) {
        self.ctx
            .send::<ArticleDetails>(self.node, DetailsMsg::ShowAuthor);
    }

    pub fn dismiss(&self) {
        self.ctx
            .send::<ArticleDetails>(self.node, DetailsMsg::Close);
    }
}

#[export]
pub struct AuthorComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl AuthorComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn name(&self) -> Option<String> {
        self.ctx
            .read::<AuthorPanel, _>(self.node, |panel| panel.author.name.clone())
    }

    pub fn dismiss(&self) {
        self.ctx.send::<AuthorPanel>(self.node, AuthorMsg::Close);
    }
}
