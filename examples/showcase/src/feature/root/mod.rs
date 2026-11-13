//! The whole application: a stack of screens with tabs at the bottom.

use crate::Services;
use crate::feature::book::{Book, BookOutput};
use crate::feature::gallery::{Gallery, GalleryOutput};
use crate::feature::tabs::{Tabs, TabsOutput};
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist};
use serde::{Deserialize, Serialize};

/// A screen of the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Tabs,
    Book,
    Gallery,
}

/// The component behind a screen.
#[derive(Clone, Copy, Debug)]
pub enum Child {
    Tabs(Handle<Tabs>),
    Book(Handle<Book>),
    Gallery(Handle<Gallery>),
}

/// Input to [`Root`].
#[derive(Clone, Debug)]
pub enum RootMsg {
    Back,
    /// Keeps the oldest `keep` screens and opens `open` above them. A `keep` of
    /// zero replaces every screen. Browser history sends this.
    Show {
        keep: usize,
        open: Vec<Screen>,
    },
}

/// The application root.
#[derive(Debug)]
pub struct Root {
    /// The screens.
    pub stack: Stack<Screen, Child, Self>,
}

impl Root {
    /// Creates the root, resuming saved navigation if there is any.
    pub fn new(cx: &mut Cx<'_, Self>, services: Services) -> Self {
        let saved = cx.restored();
        // A name keeps the saved stack its own if routers are added before it.
        let stack = cx.named_routers("screens", |cx| {
            Stack::restore_or(saved, Screen::Tabs, cx, move |screen, cx| {
                let services = services.clone();
                match screen {
                    Screen::Tabs => Child::Tabs(
                        cx.persistent_child(move |cx| Tabs::new(cx, services), Self::on_tabs),
                    ),
                    Screen::Book => {
                        Child::Book(cx.child(move |cx| Book::new(cx, services), Self::on_book))
                    }
                    Screen::Gallery => Child::Gallery(
                        cx.child(move |cx| Gallery::new(cx, services), Self::on_gallery),
                    ),
                }
            })
        });
        Self { stack }
    }

    fn on_tabs(&mut self, output: TabsOutput, cx: &mut Cx<'_, Self>) {
        let screen = match output {
            TabsOutput::OpenBook => Screen::Book,
            TabsOutput::OpenGallery => Screen::Gallery,
        };
        self.stack.push_new(screen, cx);
    }

    fn on_book(&mut self, output: BookOutput, cx: &mut Cx<'_, Self>) {
        match output {
            BookOutput::Close => {
                let _at_root = self.stack.pop(cx);
            }
        }
    }

    fn on_gallery(&mut self, output: GalleryOutput, cx: &mut Cx<'_, Self>) {
        match output {
            GalleryOutput::Close => {
                let _at_root = self.stack.pop(cx);
            }
        }
    }
}

impl Component for Root {
    type Msg = RootMsg;
    type Output = ();

    fn update(&mut self, msg: RootMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            RootMsg::Back => {
                let _at_root = self.stack.pop(cx);
            }
            RootMsg::Show { keep: 0, open } => {
                let _empty = self.stack.replace_all(open, cx);
            }
            RootMsg::Show { keep, open } => {
                self.stack.pop_to(keep, cx);
                for screen in open {
                    self.stack.push(screen, cx);
                }
            }
        }
    }

    fn back(&self) -> Option<RootMsg> {
        self.stack.can_pop().then_some(RootMsg::Back)
    }
}

impl Persist for Root {
    type Saved = SavedStack<Screen>;
    const KIND: &'static str = "showcase.root";

    fn save(&self) -> Self::Saved {
        self.stack.snapshot()
    }
}
