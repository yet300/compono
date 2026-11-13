//! A hundred pages, of which only the selected one and its neighbours exist.

mod page;
pub use page::{Page, PageMsg};

use crate::Services;
use compono::router::pages::Pages;
use compono::{Component, Cx, Handle};

const PAGES: u32 = 100;

/// Input to [`Book`].
#[derive(Clone, Copy, Debug)]
pub enum BookMsg {
    Select(usize),
    Next,
    Previous,
    Close,
}

/// What [`Book`] asks of its owner.
#[derive(Clone, Copy, Debug)]
pub enum BookOutput {
    Close,
}

/// The book screen.
#[derive(Debug)]
pub struct Book {
    /// The pages.
    pub pages: Option<Pages<u32, Handle<Page>, Self>>,
}

impl Book {
    /// Creates the book at its first page.
    pub fn new(cx: &mut Cx<'_, Self>, _services: Services) -> Self {
        let build = |number: &u32, cx: &mut Cx<'_, Self>| {
            let number = *number;
            cx.child(
                move |_cx| Page { number, taps: 0 },
                |_book: &mut Self, (), _cx| {},
            )
        };
        // A range has no repeats, so this is never `None`.
        let pages = Pages::new((1..=PAGES).collect(), 0, 1, cx, build).ok();
        Self { pages }
    }
}

impl Component for Book {
    type Msg = BookMsg;
    type Output = BookOutput;

    fn update(&mut self, msg: BookMsg, cx: &mut Cx<'_, Self>) {
        if matches!(msg, BookMsg::Close) {
            cx.emit(BookOutput::Close);
            cx.unchanged();
            return;
        }
        let Some(pages) = &mut self.pages else {
            cx.unchanged();
            return;
        };
        let selected = pages.selected();
        let moved = match msg {
            BookMsg::Select(index) => pages.select(index, cx),
            BookMsg::Next => pages.select(selected + 1, cx),
            BookMsg::Previous => selected > 0 && pages.select(selected - 1, cx),
            BookMsg::Close => false,
        };
        if !moved {
            cx.unchanged();
        }
    }
}
