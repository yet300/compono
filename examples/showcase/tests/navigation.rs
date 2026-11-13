//! Every navigation model through the real components, on virtual time.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests assert by failing")]

use compono::router::panels::{PanelsMode, ShownPanels};
use compono::{Handle, Phase, Tree};
use compono_test::{NoHost, Recording, VirtualTime, headless};
use showcase::Services;
use showcase::feature::articles::{Articles, ArticlesMsg, DetailsMsg, ListMsg};
use showcase::feature::book::{Book, BookMsg};
use showcase::feature::cards::{Cards, CardsMsg};
use showcase::feature::counters::{Counter, CounterMsg, Counters, CountersMsg, DEEP_PUSH};
use showcase::feature::feed::{FRAME, Feed, FeedMsg, RowMsg};
use showcase::feature::gallery::{Gallery, GalleryMsg, StripMode};
use showcase::feature::menu::{DialogMsg, Menu, MenuMsg};
use showcase::feature::root::{Child, Root};
use showcase::feature::tabs::{Tab, TabChild, Tabs, TabsMsg};
use std::time::Duration;

struct App {
    tree: Tree,
    root: Handle<Root>,
    time: VirtualTime,
    log: compono_test::CommitLog,
}

fn services(time: &VirtualTime) -> Services {
    let clock = time.clone();
    Services::new(move |delay| Box::pin(clock.sleep(delay)))
}

fn app_from(saved: Option<&[u8]>) -> App {
    let (host, log): (Recording<NoHost>, _) = headless();
    let time = VirtualTime::new();
    let mut tree = match saved {
        Some(bytes) => Tree::with_saved(host, bytes, showcase::saved::SCHEMA).0,
        None => Tree::new(host),
    };
    tree.set_spawner(time.clone());
    let services = services(&time);
    let root = tree.mount_persistent(move |cx| Root::new(cx, services));
    tree.set_phase(Phase::Focused);
    time.settle(&mut tree).unwrap();
    App {
        tree,
        root,
        time,
        log,
    }
}

impl App {
    fn tabs(&self) -> Handle<Tabs> {
        match *self.tree.get(self.root).unwrap().stack.active() {
            Child::Tabs(tabs) => tabs,
            other => panic!("expected tabs, got {other:?}"),
        }
    }

    fn select(&mut self, tab: Tab) -> TabChild {
        let tabs = self.tabs();
        self.tree.send(tabs, TabsMsg::Select(tab)).unwrap();
        self.settle();
        *self.tree.get(tabs).unwrap().stack.active()
    }

    fn settle(&mut self) {
        self.time.settle(&mut self.tree).unwrap();
    }

    fn advance(&mut self, by: Duration) {
        self.time.advance(&mut self.tree, by).unwrap();
    }

    fn menu(&mut self) -> Handle<Menu> {
        match self.select(Tab::Menu) {
            TabChild::Menu(menu) => menu,
            _ => panic!("expected menu"),
        }
    }

    fn counters(&mut self) -> Handle<Counters> {
        match self.select(Tab::Counters) {
            TabChild::Counters(counters) => counters,
            _ => panic!("expected counters"),
        }
    }

    fn top_counter(&self, counters: Handle<Counters>) -> Handle<Counter> {
        *self.tree.get(counters).unwrap().stack.active()
    }

    fn articles(&mut self) -> Handle<Articles> {
        match self.select(Tab::Articles) {
            TabChild::Articles(articles) => articles,
            _ => panic!("expected articles"),
        }
    }

    fn feed(&mut self) -> Handle<Feed> {
        match self.select(Tab::Feed) {
            TabChild::Feed(feed) => feed,
            _ => panic!("expected feed"),
        }
    }

    fn open_book(&mut self) -> Handle<Book> {
        let menu = self.menu();
        self.tree.send(menu, MenuMsg::OpenBook).unwrap();
        match *self.tree.get(self.root).unwrap().stack.active() {
            Child::Book(book) => book,
            other => panic!("expected book, got {other:?}"),
        }
    }

    fn open_gallery(&mut self) -> Handle<Gallery> {
        let menu = self.menu();
        self.tree.send(menu, MenuMsg::OpenGallery).unwrap();
        match *self.tree.get(self.root).unwrap().stack.active() {
            Child::Gallery(gallery) => gallery,
            other => panic!("expected gallery, got {other:?}"),
        }
    }
}

#[test]
fn a_tab_keeps_its_navigation_when_another_is_selected() {
    let mut app = app_from(None);
    let counters = app.counters();
    let top = app.top_counter(counters);
    app.tree.send(top, CounterMsg::Next).unwrap();
    assert_eq!(app.tree.get(counters).unwrap().stack.depth(), 2);

    let _articles = app.articles();
    let again = app.counters();
    assert_eq!(again, counters, "the same component came back");
    assert_eq!(app.tree.get(counters).unwrap().stack.depth(), 2);
}

#[test]
fn only_the_focused_counter_ticks() {
    let mut app = app_from(None);
    let counters = app.counters();
    let first = app.top_counter(counters);
    app.advance(Duration::from_millis(500));
    assert_eq!(app.tree.get(first).unwrap().value, 2);
    app.tree.send(first, CounterMsg::Next).unwrap();
    app.advance(Duration::from_millis(1000));
    assert_eq!(
        app.tree.get(first).unwrap().value,
        2,
        "covered counters stop"
    );
    assert_eq!(app.time.live_tasks(), 1);
}

#[test]
fn a_thousand_screens_cost_one_timer() {
    let mut app = app_from(None);
    let counters = app.counters();
    app.tree.send(counters, CountersMsg::GoDeep).unwrap();
    app.settle();
    assert_eq!(app.tree.get(counters).unwrap().stack.depth(), DEEP_PUSH + 1);
    assert_eq!(app.time.live_tasks(), 1);
    app.tree.send(counters, CountersMsg::PopToRoot).unwrap();
    assert_eq!(app.tree.get(counters).unwrap().stack.depth(), 1);
}

#[test]
fn a_restart_restores_tabs_screens_and_values_lazily() {
    let mut app = app_from(None);
    let counters = app.counters();
    let top = app.top_counter(counters);
    app.advance(Duration::from_millis(750));
    app.tree.send(top, CounterMsg::Next).unwrap();
    let bytes = app.tree.save(showcase::saved::SCHEMA).unwrap();

    let mut app = app_from(Some(&bytes));
    let tabs = app.tabs();
    assert_eq!(*app.tree.get(tabs).unwrap().stack.route(), Tab::Counters);
    let TabChild::Counters(counters) = *app.tree.get(tabs).unwrap().stack.active() else {
        panic!("expected counters");
    };
    assert_eq!(app.tree.get(counters).unwrap().stack.depth(), 2);
    assert!(
        app.tree
            .get(counters)
            .unwrap()
            .stack
            .parked()
            .all(|(_route, child)| child.is_none()),
        "the lower screen is built only when returned to"
    );
    app.tree
        .send(app.top_counter(counters), CounterMsg::Previous)
        .unwrap();
    let first = app.top_counter(counters);
    assert_eq!(app.tree.get(first).unwrap().value, 3);
    assert_eq!(*app.tree.get(tabs).unwrap().stack.route(), Tab::Counters);
}

#[test]
fn back_closes_the_dialog_before_anything_else() {
    let mut app = app_from(None);
    let menu = app.menu();
    assert!(!app.tree.back(), "nothing to go back to");
    app.tree.send(menu, MenuMsg::ShowDialog).unwrap();
    assert!(app.tree.get(menu).unwrap().dialog.is_open());
    assert!(app.tree.back());
    assert!(!app.tree.get(menu).unwrap().dialog.is_open());
    app.tree.send(menu, MenuMsg::ShowDialog).unwrap();
    let dialog = *app.tree.get(menu).unwrap().dialog.active().unwrap();
    app.tree.send(dialog, DialogMsg::Dismiss).unwrap();
    assert!(!app.tree.get(menu).unwrap().dialog.is_open());
}

#[test]
fn cards_keep_their_components_while_reordering() {
    let mut app = app_from(None);
    let TabChild::Cards(cards) = app.select(Tab::Cards) else {
        panic!("expected cards");
    };
    let numbers = |app: &App, cards: Handle<Cards>| -> Vec<u32> {
        let set = &app.tree.get(cards).unwrap().cards;
        set.iter().map(|(number, _child, _phase)| *number).collect()
    };
    assert_eq!(numbers(&app, cards), [1, 2, 3, 4]);
    let last = app
        .tree
        .get(cards)
        .unwrap()
        .cards
        .child_at(3)
        .copied()
        .unwrap();
    app.advance(Duration::from_millis(1000));
    let value = app.tree.get(last).unwrap().value;
    assert_eq!(value, 2);

    app.tree.send(cards, CardsMsg::SendTopToBack).unwrap();
    assert_eq!(numbers(&app, cards), [4, 1, 2, 3]);
    assert_eq!(app.tree.get(cards).unwrap().cards.child_at(0), Some(&last));
    app.advance(Duration::from_millis(1000));
    assert_eq!(
        app.tree.get(last).unwrap().value,
        value,
        "only the top ticks"
    );
    app.tree.send(cards, CardsMsg::Remove).unwrap();
    assert_eq!(numbers(&app, cards), [4, 1, 2]);
}

#[test]
fn article_panels_follow_the_mode_and_back() {
    let mut app = app_from(None);
    let articles = app.articles();
    let panels = |app: &App| app.tree.get(articles).unwrap().panels.shown();
    let list = *app.tree.get(articles).unwrap().panels.main().unwrap();
    app.tree.send(list, ListMsg::Select(5)).unwrap();
    assert_eq!(
        panels(&app),
        ShownPanels {
            main: true,
            details: true,
            extra: false
        }
    );
    let details = *app.tree.get(articles).unwrap().panels.details().unwrap();
    app.tree.send(details, DetailsMsg::ShowAuthor).unwrap();
    assert_eq!(
        panels(&app),
        ShownPanels {
            main: false,
            details: true,
            extra: true
        }
    );
    app.tree
        .send(articles, ArticlesMsg::SetMode(PanelsMode::Triple))
        .unwrap();
    assert!(panels(&app).main);
    assert!(app.tree.back(), "Back closes the deepest panel");
    assert!(!panels(&app).extra);
    assert!(app.tree.back());
    assert!(!panels(&app).details);
    assert!(!app.tree.back());
}

#[test]
fn a_book_keeps_only_neighbours_alive() {
    let mut app = app_from(None);
    let book = app.open_book();
    let pages = |app: &App| {
        let pages = app.tree.get(book).unwrap().pages.as_ref().unwrap();
        (pages.selected(), pages.live())
    };
    assert_eq!(pages(&app), (0, 2));
    app.tree.send(book, BookMsg::Select(50)).unwrap();
    assert_eq!(pages(&app), (50, 3));
    app.tree.send(book, BookMsg::Next).unwrap();
    assert_eq!(pages(&app), (51, 3));
    assert!(app.tree.back(), "Back leaves the book");
    assert!(matches!(
        *app.tree.get(app.root).unwrap().stack.active(),
        Child::Tabs(_)
    ));
}

#[test]
fn the_custom_router_grants_phases_and_removes_frames() {
    let mut app = app_from(None);
    let gallery = app.open_gallery();
    app.settle();
    let strip = |app: &App| {
        let strip = &app.tree.get(gallery).unwrap().strip;
        (strip.frames().count(), strip.mode())
    };
    assert_eq!(strip(&app), (24, StripMode::Contact));
    assert_eq!(app.time.live_tasks(), 24, "every visible frame develops");

    app.tree.send(gallery, GalleryMsg::ToggleMode).unwrap();
    app.settle();
    assert_eq!(app.time.live_tasks(), 1, "hidden frames stop");
    let selected = *app.tree.get(gallery).unwrap().strip.selected().unwrap();
    assert_eq!(app.tree.phase(selected), Some(Phase::Focused));

    app.tree.send(gallery, GalleryMsg::Next).unwrap();
    app.tree.send(gallery, GalleryMsg::RemoveSelected).unwrap();
    assert_eq!(strip(&app).0, 23);
    app.tree.send(gallery, GalleryMsg::Previous).unwrap();
    app.tree.send(gallery, GalleryMsg::Previous).unwrap();
    app.tree.send(gallery, GalleryMsg::Close).unwrap();
    assert!(app.tree.get(gallery).is_none());
    app.settle();
}

#[test]
fn the_feed_costs_only_what_is_on_screen() {
    let mut app = app_from(None);
    let feed = app.feed();
    app.tree.send(feed, FeedMsg::Window(40..70)).unwrap();
    app.settle();
    assert_eq!(app.tree.get(feed).unwrap().items.live(), 30);

    let row = *app.tree.get(feed).unwrap().items.child(45).unwrap();
    app.log.clear();
    app.advance(FRAME);
    let commit = app.log.last().unwrap();
    assert_eq!(commit.dirty.len(), 30, "only the rows changed");
    assert!(!commit.dirty.contains(&feed.id()));
    assert!(app.tree.get(row).unwrap().value > 0);

    app.tree.send(row, RowMsg::Like).unwrap();
    app.tree.send(feed, FeedMsg::Window(45..75)).unwrap();
    assert_eq!(app.tree.get(feed).unwrap().items.child(45), Some(&row));
    assert!(
        app.tree.get(row).unwrap().liked,
        "the row survived scrolling"
    );

    let _menu = app.menu();
    app.settle();
    let frames = app.tree.get(feed).unwrap().frames;
    app.advance(FRAME * 10);
    assert_eq!(
        app.tree.get(feed).unwrap().frames,
        frames,
        "a covered feed stops"
    );
}
