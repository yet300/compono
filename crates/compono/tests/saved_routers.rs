//! `Items` and `Panels` across a restart.

#![cfg(all(feature = "std", feature = "serde"))]
#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::items::{Items, SavedItems};
use compono::router::panels::{PanelTypes, Panels, PanelsMode, SavedPanels};
use compono::{Component, Cx, Handle, Persist, Phase, Tree};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<(u32, Phase)>>>;

struct Leaf {
    name: u32,
    value: u32,
    log: Option<Log>,
}

impl Component for Leaf {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.value = msg;
    }
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        if let Some(log) = &self.log {
            log.borrow_mut().push((self.name, phase));
        }
    }
}

impl Persist for Leaf {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.value
    }
}

fn leaf<P: Component>(name: u32, log: Option<&Log>, cx: &mut Cx<'_, P>) -> Handle<Leaf> {
    let log = log.cloned();
    cx.persistent_child(
        move |cx| Leaf {
            name,
            value: cx.restored().unwrap_or(0),
            log,
        },
        |_parent, (), _cx| {},
    )
}

struct Feed {
    items: Items<u32, Handle<Leaf>, Self>,
}

impl Component for Feed {
    type Msg = core::ops::Range<usize>;
    type Output = ();
    fn update(&mut self, window: core::ops::Range<usize>, cx: &mut Cx<'_, Self>) {
        self.items.set_window(window, cx).unwrap();
    }
}

impl Persist for Feed {
    type Saved = SavedItems<u32>;
    fn save(&self) -> SavedItems<u32> {
        self.items.snapshot()
    }
}

fn feed(tree: &mut Tree, keys: Vec<u32>) -> Handle<Feed> {
    tree.mount_persistent(|cx| {
        let saved = cx.restored();
        Feed {
            items: Items::restore_or(saved, keys, cx, |key, cx| leaf(*key, None, cx)).unwrap(),
        }
    })
}

fn row(tree: &Tree, root: Handle<Feed>, index: usize) -> Option<u32> {
    let child = tree.get(root)?.items.child(index)?;
    tree.get(*child).map(|leaf| leaf.value)
}

#[test]
fn a_list_keeps_its_window_and_the_state_of_its_rows_through_a_restart() {
    let mut tree = Tree::new(NullHost);
    let root = feed(&mut tree, (0..100).collect());
    tree.send(root, 10..14).unwrap();
    let twelfth = *tree.get(root).unwrap().items.child(12).unwrap();
    tree.send(twelfth, 77).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = feed(&mut tree, (0..100).collect());
    let items = &tree.get(root).unwrap().items;
    assert_eq!(items.window(), 10..14);
    assert_eq!(items.live(), 4);
    assert_eq!(row(&tree, root, 12), Some(77));
    assert_eq!(row(&tree, root, 11), Some(0));
}

#[test]
fn a_restored_list_forgets_rows_that_are_no_longer_in_its_window() {
    let mut tree = Tree::new(NullHost);
    let root = feed(&mut tree, (0..100).collect());
    tree.send(root, 10..14).unwrap();
    let twelfth = *tree.get(root).unwrap().items.child(12).unwrap();
    tree.send(twelfth, 77).unwrap();
    let bytes = tree.save(1).unwrap();

    // The list is shorter on the next launch, so the saved window is cut.
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = feed(&mut tree, (0..11).collect());
    assert_eq!(tree.get(root).unwrap().items.window(), 10..11);
    let again = tree.save(1).unwrap();
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &again, 1);
    let root = feed(&mut tree, (0..100).collect());
    tree.send(root, 10..14).unwrap();
    assert_eq!(row(&tree, root, 12), Some(0), "row 12 left the window");
}

struct Three;
impl PanelTypes for Three {
    type Main = u32;
    type MainChild = Handle<Leaf>;
    type Details = u32;
    type DetailsChild = Handle<Leaf>;
    type Extra = u32;
    type ExtraChild = Handle<Leaf>;
}

struct Screen {
    panels: Panels<Three, Self>,
}

enum Msg {
    Main(u32),
    Details(Option<u32>),
    Extra(Option<u32>),
}

impl Component for Screen {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Main(route) => {
                self.panels.set_main(route, cx);
            }
            Msg::Details(route) => self.panels.set_details(route, cx),
            Msg::Extra(route) => self.panels.set_extra(route, cx),
        }
    }
}

impl Persist for Screen {
    type Saved = SavedPanels<u32, u32, u32>;
    fn save(&self) -> Self::Saved {
        self.panels.snapshot()
    }
}

fn screen(tree: &mut Tree, mode: PanelsMode, log: &Log) -> Handle<Screen> {
    tree.set_phase(Phase::Focused);
    let build = |log: &Log| {
        let log = Rc::clone(log);
        move |route: &u32, cx: &mut Cx<'_, Screen>| leaf(*route, Some(&log), cx)
    };
    let (main, details, extra) = (build(log), build(log), build(log));
    tree.mount_persistent(move |cx| {
        let saved = cx.restored();
        Screen {
            panels: Panels::restore_or(saved, 1, mode, cx, main, details, extra),
        }
    })
}

fn routes(tree: &Tree, root: Handle<Screen>) -> (Option<u32>, Option<u32>, Option<u32>) {
    let panels = &tree.get(root).unwrap().panels;
    (
        panels.main_route().copied(),
        panels.details_route().copied(),
        panels.extra_route().copied(),
    )
}

#[test]
fn panels_keep_their_routes_and_state_through_a_restart() {
    let log = Log::default();
    let mut tree = Tree::new(NullHost);
    let root = screen(&mut tree, PanelsMode::Dual, &log);
    tree.send(root, Msg::Details(Some(2))).unwrap();
    tree.send(root, Msg::Extra(Some(3))).unwrap();
    let details = *tree.get(root).unwrap().panels.details().unwrap();
    tree.send(details, 55).unwrap();
    let bytes = tree.save(1).unwrap();

    let log = Log::default();
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = screen(&mut tree, PanelsMode::Dual, &log);
    assert_eq!(routes(&tree, root), (Some(1), Some(2), Some(3)));
    let panels = &tree.get(root).unwrap().panels;
    assert_eq!(tree.get(*panels.details().unwrap()).unwrap().value, 55);
    assert_eq!(
        *log.borrow(),
        [(2, Phase::Visible), (3, Phase::Focused)],
        "each panel goes straight to its phase, and only one is ever focused"
    );
}

#[test]
fn the_main_panel_can_be_replaced() {
    let log = Log::default();
    let mut tree = Tree::new(NullHost);
    let root = screen(&mut tree, PanelsMode::Single, &log);
    let old = *tree.get(root).unwrap().panels.main().unwrap();
    tree.send(root, Msg::Main(9)).unwrap();
    assert_eq!(routes(&tree, root), (Some(9), None, None));
    assert_eq!(tree.get(old).map(|leaf| leaf.value), None);
    let new = *tree.get(root).unwrap().panels.main().unwrap();
    assert_eq!(tree.phase(new), Some(Phase::Focused));
}

struct Repeats {
    items: Items<u32, Handle<Leaf>, Self>,
    refused: Vec<bool>,
}

enum RepeatsMsg {
    Window(core::ops::Range<usize>),
    Keys(Vec<u32>),
}

impl Component for Repeats {
    type Msg = RepeatsMsg;
    type Output = ();
    fn update(&mut self, msg: RepeatsMsg, cx: &mut Cx<'_, Self>) {
        let refused = match msg {
            RepeatsMsg::Window(window) => self.items.set_window(window, cx).is_err(),
            RepeatsMsg::Keys(keys) => self.items.set_keys(keys, cx).is_err(),
        };
        self.refused.push(refused);
    }
}

#[test]
fn a_list_refuses_a_key_that_repeats_inside_its_window_and_changes_nothing() {
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| Repeats {
        // Keys 2 and 3 are equal. Outside the window that does no harm.
        items: Items::new(vec![1, 2, 7, 7, 5], cx, |key, cx| leaf(*key, None, cx)),
        refused: Vec::new(),
    });
    tree.send(root, RepeatsMsg::Window(0..3)).unwrap();
    tree.send(root, RepeatsMsg::Window(1..4)).unwrap();
    tree.send(root, RepeatsMsg::Keys(vec![4, 4, 4])).unwrap();
    let list = tree.get(root).unwrap();
    assert_eq!(list.refused, [false, true, true]);
    assert_eq!(list.items.window(), 0..3);
    assert_eq!(list.items.key(1), Some(&2));
    assert_eq!(list.items.live(), 3);
    assert!(!list.items.is_empty());
}
