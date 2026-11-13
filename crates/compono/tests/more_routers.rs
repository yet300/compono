//! Behaviour of `Pages`, `Panels` and `Items`.

#![cfg(all(feature = "std", feature = "serde"))]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

mod common;

use common::NullHost;
use compono::router::items::Items;
use compono::router::pages::{Pages, SavedPages};
use compono::router::panels::{PanelTypes, Panels, PanelsMode, ShownPanels};
use compono::{Component, Cx, Handle, Persist, Phase, Tree};

struct Leaf(u32);
impl Component for Leaf {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 = msg;
    }
}
impl Persist for Leaf {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.0
    }
}

fn leaf<P: Component>(cx: &mut Cx<'_, P>) -> Handle<Leaf> {
    cx.persistent_child(
        |cx| Leaf(cx.restored().unwrap_or(0)),
        |_parent: &mut P, (), _cx| {},
    )
}

struct Book {
    pages: Pages<u32, Handle<Leaf>, Book>,
}
impl Component for Book {
    type Msg = usize;
    type Output = ();
    fn update(&mut self, index: usize, cx: &mut Cx<'_, Self>) {
        self.pages.select(index, cx);
    }
}
impl Persist for Book {
    type Saved = SavedPages<u32>;
    fn save(&self) -> SavedPages<u32> {
        self.pages.snapshot()
    }
}

fn book(tree: &mut Tree) -> Handle<Book> {
    tree.mount_persistent(|cx| {
        let saved = cx.restored();
        Book {
            pages: Pages::restore_or(saved, (0..100).collect(), 1, cx, |_route, cx| leaf(cx))
                .unwrap(),
        }
    })
}

#[test]
fn only_pages_near_the_selection_are_alive() {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = book(&mut tree);
    assert_eq!(tree.get(root).unwrap().pages.live(), 2);
    tree.send(root, 50).unwrap();
    let pages = &tree.get(root).unwrap().pages;
    assert_eq!(pages.live(), 3);
    assert_eq!(pages.selected(), 50);
    assert_eq!(tree.phase(*pages.active().unwrap()), Some(Phase::Focused));
}

/// The phase of the page built for `route`, or `None` if it has no component.
fn page_phase(tree: &Tree, built: &Built, route: u32) -> Option<Phase> {
    let page = built.borrow().get(&route).copied()?;
    tree.phase(page)
}

type Built = std::rc::Rc<std::cell::RefCell<std::collections::HashMap<u32, Handle<Leaf>>>>;

#[test]
fn the_neighbours_of_the_selection_are_alive_and_hidden() {
    let built = Built::default();
    let seen = std::rc::Rc::clone(&built);
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(move |cx| Book {
        pages: Pages::new((0..100).collect(), 0, 1, cx, move |route, cx| {
            let page = leaf(cx);
            seen.borrow_mut().insert(*route, page);
            page
        })
        .unwrap(),
    });
    tree.send(root, 50).unwrap();
    assert_eq!(page_phase(&tree, &built, 49), Some(Phase::Hidden));
    assert_eq!(page_phase(&tree, &built, 50), Some(Phase::Focused));
    assert_eq!(page_phase(&tree, &built, 51), Some(Phase::Hidden));
    assert_eq!(page_phase(&tree, &built, 0), None, "it left the range");
    assert_eq!(page_phase(&tree, &built, 52), None, "it is out of range");

    tree.send(root, 51).unwrap();
    assert_eq!(page_phase(&tree, &built, 49), None);
    assert_eq!(page_phase(&tree, &built, 50), Some(Phase::Hidden));
    assert_eq!(page_phase(&tree, &built, 51), Some(Phase::Focused));
    assert_eq!(page_phase(&tree, &built, 52), Some(Phase::Hidden));
}

#[test]
fn a_page_keeps_its_state_through_a_restart() {
    let mut tree = Tree::new(NullHost);
    let root = book(&mut tree);
    tree.send(root, 7).unwrap();
    tree.send(*tree.get(root).unwrap().pages.active().unwrap(), 77)
        .unwrap();
    let bytes = tree.save(1).unwrap();
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = book(&mut tree);
    let pages = &tree.get(root).unwrap().pages;
    assert_eq!(pages.selected(), 7);
    assert_eq!(tree.get(*pages.active().unwrap()).unwrap().0, 77);
}

#[test]
fn repeated_routes_are_refused() {
    struct Refuser(bool);
    impl Component for Refuser {
        type Msg = ();
        type Output = ();
        fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    }
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| {
        let result = Pages::<u32, (), Refuser>::new(vec![1, 1], 0, 0, cx, |_route, _cx| ());
        Refuser(result.is_err())
    });
    assert!(tree.get(root).unwrap().0);
}

struct Types;
impl PanelTypes for Types {
    type Main = ();
    type MainChild = Handle<Leaf>;
    type Details = u32;
    type DetailsChild = Handle<Leaf>;
    type Extra = u32;
    type ExtraChild = Handle<Leaf>;
}

struct Screen {
    panels: Panels<Types, Screen>,
}

enum PanelMsg {
    Details(Option<u32>),
    Extra(Option<u32>),
    Mode(PanelsMode),
    Pop,
}

impl Component for Screen {
    type Msg = PanelMsg;
    type Output = ();
    fn update(&mut self, msg: PanelMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            PanelMsg::Details(route) => self.panels.set_details(route, cx),
            PanelMsg::Extra(route) => self.panels.set_extra(route, cx),
            PanelMsg::Mode(mode) => self.panels.set_mode(mode, cx),
            PanelMsg::Pop => {
                self.panels.pop(cx);
            }
        }
    }
}

fn screen(tree: &mut Tree) -> Handle<Screen> {
    tree.set_phase(Phase::Focused);
    tree.mount(|cx| Screen {
        panels: Panels::new(
            (),
            PanelsMode::Single,
            cx,
            |(), cx| leaf(cx),
            |_route, cx| leaf(cx),
            |_route, cx| leaf(cx),
        ),
    })
}

fn phases(tree: &Tree, root: Handle<Screen>) -> [Option<Phase>; 3] {
    let panels = &tree.get(root).unwrap().panels;
    [
        panels.main().and_then(|child| tree.phase(*child)),
        panels.details().and_then(|child| tree.phase(*child)),
        panels.extra().and_then(|child| tree.phase(*child)),
    ]
}

#[test]
fn the_mode_decides_which_panels_are_on_screen() {
    use Phase::{Focused, Hidden, Visible};
    let mut tree = Tree::new(NullHost);
    let root = screen(&mut tree);
    assert_eq!(phases(&tree, root), [Some(Focused), None, None]);

    tree.send(root, PanelMsg::Details(Some(1))).unwrap();
    tree.send(root, PanelMsg::Extra(Some(2))).unwrap();
    assert_eq!(
        phases(&tree, root),
        [Some(Hidden), Some(Hidden), Some(Focused)]
    );
    let shown = tree.get(root).unwrap().panels.shown();
    assert_eq!(
        shown,
        ShownPanels {
            main: false,
            details: false,
            extra: true
        }
    );

    tree.send(root, PanelMsg::Mode(PanelsMode::Dual)).unwrap();
    assert_eq!(
        phases(&tree, root),
        [Some(Hidden), Some(Visible), Some(Focused)]
    );
    tree.send(root, PanelMsg::Mode(PanelsMode::Triple)).unwrap();
    assert_eq!(
        phases(&tree, root),
        [Some(Visible), Some(Visible), Some(Focused)]
    );

    tree.send(root, PanelMsg::Pop).unwrap();
    assert_eq!(phases(&tree, root), [Some(Visible), Some(Focused), None]);
    tree.send(root, PanelMsg::Details(None)).unwrap();
    assert_eq!(phases(&tree, root), [Some(Focused), None, None]);
}

struct Feed {
    items: Items<u32, Handle<Leaf>, Feed>,
}
impl Component for Feed {
    type Msg = core::ops::Range<usize>;
    type Output = ();
    fn update(&mut self, window: core::ops::Range<usize>, cx: &mut Cx<'_, Self>) {
        assert!(self.items.set_window(window, cx).is_ok());
    }
}

#[test]
fn a_million_keys_cost_only_the_window() {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(|cx| Feed {
        items: Items::new((0..1_000_000).collect(), cx, |_key, cx| leaf(cx)),
    });
    tree.send(root, 10..30).unwrap();
    let first = *tree.get(root).unwrap().items.child(15).unwrap();
    tree.send(first, 5).unwrap();
    assert_eq!(tree.get(root).unwrap().items.live(), 20);

    tree.send(root, 12..32).unwrap();
    let items = &tree.get(root).unwrap().items;
    assert_eq!(items.child(15), Some(&first), "overlapping rows survive");
    assert_eq!(items.child(10), None);
    assert_eq!(tree.phase(first), Some(Phase::Visible));

    tree.send(root, 999_990..2_000_000).unwrap();
    let items = &tree.get(root).unwrap().items;
    assert_eq!(items.window(), 999_990..1_000_000, "clamped to the list");
    assert_eq!(items.live(), 10);
}
