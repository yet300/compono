//! Routers named with `Cx::named_routers` find their saved state by name, and
//! unnamed routers save and restore as they always have.

#![cfg(feature = "serde")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::slot::{SavedSlot, Slot};
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist, Tree};
use serde::{Deserialize, Serialize};

struct Cell(u32);

impl Component for Cell {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, value: u32, _cx: &mut Cx<'_, Self>) {
        self.0 = value;
    }
}

impl Persist for Cell {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.0
    }
}

fn cell<P: Component>(_route: &u32, cx: &mut Cx<'_, P>) -> Handle<Cell> {
    cx.persistent_child(|cx| Cell(cx.restored().unwrap_or(0)), |_owner, (), _cx| {})
}

fn value(tree: &Tree, cell: Handle<Cell>) -> u32 {
    tree.get(cell).unwrap().0
}

/// Unnamed routers and children of every kind, as an application before
/// named routers would have them.
struct Plain {
    stack: Stack<u32, Handle<Cell>, Plain>,
    dialog: Slot<u32, Handle<Cell>, Plain>,
    numbered: Handle<Cell>,
    named: Handle<Cell>,
}

#[derive(Serialize, Deserialize)]
struct PlainSaved {
    stack: SavedStack<u32>,
    dialog: SavedSlot<u32>,
}

enum PlainMsg {
    Push(u32),
    Pop,
    Show(u32),
}

impl Component for Plain {
    type Msg = PlainMsg;
    type Output = ();
    fn update(&mut self, msg: PlainMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            PlainMsg::Push(route) => self.stack.push(route, cx),
            PlainMsg::Pop => {
                self.stack.pop(cx).unwrap();
            }
            PlainMsg::Show(route) => {
                self.dialog.show(route, cx);
            }
        }
    }
}

impl Persist for Plain {
    type Saved = PlainSaved;
    fn save(&self) -> PlainSaved {
        PlainSaved {
            stack: self.stack.snapshot(),
            dialog: self.dialog.snapshot(),
        }
    }
}

fn mount_plain(tree: &mut Tree) -> Handle<Plain> {
    tree.mount_persistent(|cx| {
        let saved: Option<PlainSaved> = cx.restored();
        let (stack, dialog) = match saved {
            Some(saved) => (Some(saved.stack), Some(saved.dialog)),
            None => (None, None),
        };
        Plain {
            stack: Stack::restore_or(stack, 0, cx, cell),
            dialog: Slot::restore(dialog, cx, cell),
            numbered: cell(&0, cx),
            named: cx.persistent_child_keyed(
                "draft",
                |cx| Cell(cx.restored().unwrap_or(0)),
                |_plain, (), _cx| {},
            ),
        }
    })
}

/// What `plain_launch` saves. Its keys are those saved before routers could be
/// named, in the envelope whose entries record a kind and a version.
#[rustfmt::skip]
const PLAIN_BYTES: &[u8] = &[130, 128, 128, 6, 1, 6, 1, 0, 0, 0, 10, 2, 0, 0, 1, 1, 2, 1, 0, 5, 1, 3, 0, 0, 0, 0, 0, 1, 10, 3, 0, 1, 0, 0, 0, 1, 11, 3, 0, 128, 128, 128, 128, 128, 128, 64, 0, 0, 0, 1, 12, 2, 0, 128, 128, 128, 128, 128, 128, 128, 128, 128, 1, 0, 0, 1, 13, 2, 0, 187, 160, 152, 155, 164, 142, 154, 200, 197, 1, 0, 0, 1, 14];

/// Two screens holding 10 and 11, a dialog holding 12, and children holding 13 and 14.
fn plain_launch() -> Vec<u8> {
    let mut tree = Tree::new(NullHost);
    let plain = mount_plain(&mut tree);
    let root = *tree.get(plain).unwrap().stack.active();
    tree.send(root, 10).unwrap();
    tree.send(plain, PlainMsg::Push(1)).unwrap();
    let top = *tree.get(plain).unwrap().stack.active();
    tree.send(top, 11).unwrap();
    tree.send(plain, PlainMsg::Show(5)).unwrap();
    let dialog = *tree.get(plain).unwrap().dialog.active().unwrap();
    tree.send(dialog, 12).unwrap();
    let (numbered, named) = {
        let plain = tree.get(plain).unwrap();
        (plain.numbered, plain.named)
    };
    tree.send(numbered, 13).unwrap();
    tree.send(named, 14).unwrap();
    tree.save(1).unwrap()
}

#[test]
fn unnamed_routers_save_the_same_bytes_as_before() {
    assert_eq!(plain_launch(), PLAIN_BYTES);
}

#[test]
fn unnamed_routers_restore_bytes_saved_before() {
    let (mut tree, _outcome) = Tree::with_saved(NullHost, PLAIN_BYTES, 1);
    let plain = mount_plain(&mut tree);
    let top = *tree.get(plain).unwrap().stack.active();
    assert_eq!(value(&tree, top), 11);
    let dialog = *tree.get(plain).unwrap().dialog.active().unwrap();
    assert_eq!(value(&tree, dialog), 12);
    let (numbered, named) = {
        let plain = tree.get(plain).unwrap();
        (plain.numbered, plain.named)
    };
    assert_eq!((value(&tree, numbered), value(&tree, named)), (13, 14));
    // The screen below the restored top was never shown, and keeps its state.
    tree.send(plain, PlainMsg::Pop).unwrap();
    let root = *tree.get(plain).unwrap().stack.active();
    assert_eq!(value(&tree, root), 10);
}

/// A numbered dialog and a named one. Later versions add a router before them,
/// or construct the named one first.
struct App {
    numbered: Slot<u32, Handle<Cell>, App>,
    named: Slot<u32, Handle<Cell>, App>,
    _extra: Option<Slot<u32, Handle<Cell>, App>>,
}

#[derive(Serialize, Deserialize)]
struct AppSaved {
    numbered: SavedSlot<u32>,
    named: SavedSlot<u32>,
}

enum AppMsg {
    ShowNumbered(u32),
    ShowNamed(u32),
}

impl Component for App {
    type Msg = AppMsg;
    type Output = ();
    fn update(&mut self, msg: AppMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            AppMsg::ShowNumbered(route) => {
                self.numbered.show(route, cx);
            }
            AppMsg::ShowNamed(route) => {
                self.named.show(route, cx);
            }
        }
    }
}

impl Persist for App {
    type Saved = AppSaved;
    fn save(&self) -> AppSaved {
        AppSaved {
            numbered: self.numbered.snapshot(),
            named: self.named.snapshot(),
        }
    }
}

/// How one version of the application constructs its routers.
#[derive(Clone, Copy)]
struct Version {
    /// Constructs a transient router before the others.
    extra_first: bool,
    /// Constructs the named dialog before the numbered one.
    named_first: bool,
}

const FIRST: Version = Version {
    extra_first: false,
    named_first: false,
};

fn mount_app(tree: &mut Tree, version: Version) -> Handle<App> {
    tree.mount_persistent(move |cx| {
        let saved: Option<AppSaved> = cx.restored();
        let (numbered, named) = match saved {
            Some(saved) => (Some(saved.numbered), Some(saved.named)),
            None => (None, None),
        };
        let extra = version.extra_first.then(|| Slot::new(cx, cell));
        let named_slot = |cx: &mut Cx<'_, App>, saved| {
            cx.named_routers("dialog", |cx| Slot::restore(saved, cx, cell))
        };
        let (numbered, named) = if version.named_first {
            let named = named_slot(cx, named);
            (Slot::restore(numbered, cx, cell), named)
        } else {
            let numbered = Slot::restore(numbered, cx, cell);
            (numbered, named_slot(cx, named))
        };
        App {
            numbered,
            named,
            _extra: extra,
        }
    })
}

/// Saves a numbered dialog holding 111 and a named one holding 222 with the
/// first version, and returns what each holds after a relaunch with `later`.
fn upgraded_to(later: Version) -> (Option<u32>, Option<u32>) {
    let mut tree = Tree::new(NullHost);
    let app = mount_app(&mut tree, FIRST);
    tree.send(app, AppMsg::ShowNumbered(1)).unwrap();
    tree.send(app, AppMsg::ShowNamed(2)).unwrap();
    let (numbered, named) = {
        let app = tree.get(app).unwrap();
        (
            *app.numbered.active().unwrap(),
            *app.named.active().unwrap(),
        )
    };
    tree.send(numbered, 111).unwrap();
    tree.send(named, 222).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let app = mount_app(&mut tree, later);
    let app = tree.get(app).unwrap();
    let numbered = app.numbered.active().map(|cell| value(&tree, *cell));
    let named = app.named.active().map(|cell| value(&tree, *cell));
    (numbered, named)
}

#[test]
fn a_named_router_keeps_its_state_when_a_router_is_added_before_it() {
    let (_numbered, named) = upgraded_to(Version {
        extra_first: true,
        named_first: false,
    });
    assert_eq!(named, Some(222));
}

#[test]
fn a_named_router_and_a_numbered_router_keep_their_own_state() {
    let both = upgraded_to(Version {
        extra_first: false,
        named_first: true,
    });
    assert_eq!(both, (Some(111), Some(222)));
}

/// Two dialogs that ask for the same name.
struct Twins {
    first: Slot<u32, Handle<Cell>, Twins>,
    second: Slot<u32, Handle<Cell>, Twins>,
}

#[derive(Serialize, Deserialize)]
struct TwinsSaved {
    first: SavedSlot<u32>,
    second: SavedSlot<u32>,
}

impl Component for Twins {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.first.show(1, cx);
        self.second.show(1, cx);
    }
}

impl Persist for Twins {
    type Saved = TwinsSaved;
    fn save(&self) -> TwinsSaved {
        TwinsSaved {
            first: self.first.snapshot(),
            second: self.second.snapshot(),
        }
    }
}

fn mount_twins(tree: &mut Tree) -> Handle<Twins> {
    tree.mount_persistent(|cx| {
        let saved: Option<TwinsSaved> = cx.restored();
        let (first, second) = match saved {
            Some(saved) => (Some(saved.first), Some(saved.second)),
            None => (None, None),
        };
        Twins {
            first: cx.named_routers("dialog", |cx| Slot::restore(first, cx, cell)),
            second: cx.named_routers("dialog", |cx| Slot::restore(second, cx, cell)),
        }
    })
}

#[test]
fn a_repeated_name_does_not_share_state() {
    let mut tree = Tree::new(NullHost);
    let twins = mount_twins(&mut tree);
    tree.send(twins, ()).unwrap();
    let (first, second) = {
        let twins = tree.get(twins).unwrap();
        (
            *twins.first.active().unwrap(),
            *twins.second.active().unwrap(),
        )
    };
    tree.send(first, 111).unwrap();
    tree.send(second, 222).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let twins = mount_twins(&mut tree);
    let twins = tree.get(twins).unwrap();
    let first = value(&tree, *twins.first.active().unwrap());
    let second = value(&tree, *twins.second.active().unwrap());
    assert_eq!(first, 111, "the first router keeps the name");
    assert_eq!(second, 0, "the second router saves nothing");
}

/// A named stack its owner rebuilds from its own snapshot.
struct Screens {
    stack: Option<Stack<u32, Handle<Cell>, Screens>>,
}

enum ScreensMsg {
    Push(u32),
    Rebuild,
}

impl Component for Screens {
    type Msg = ScreensMsg;
    type Output = ();
    fn update(&mut self, msg: ScreensMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ScreensMsg::Push(route) => self.stack.as_mut().unwrap().push(route, cx),
            ScreensMsg::Rebuild => {
                let old = self.stack.take().unwrap();
                let saved = Some(old.snapshot());
                self.stack =
                    Some(cx.replace_router(old, |cx| Stack::restore_or(saved, 0, cx, cell)));
            }
        }
    }
}

impl Persist for Screens {
    type Saved = SavedStack<u32>;
    fn save(&self) -> SavedStack<u32> {
        self.stack.as_ref().unwrap().snapshot()
    }
}

fn mount_screens(tree: &mut Tree, numbered_before: bool) -> Handle<Screens> {
    tree.mount_persistent(move |cx| {
        let saved = cx.restored();
        if numbered_before {
            drop(Slot::new(cx, cell));
        }
        Screens {
            stack: Some(cx.named_routers("screens", |cx| Stack::restore_or(saved, 0, cx, cell))),
        }
    })
}

#[test]
fn a_rebuilt_named_router_keeps_its_name() {
    let mut tree = Tree::new(NullHost);
    let screens = mount_screens(&mut tree, false);
    tree.send(screens, ScreensMsg::Rebuild).unwrap();
    tree.send(screens, ScreensMsg::Push(2)).unwrap();
    let top = *tree.get(screens).unwrap().stack.as_ref().unwrap().active();
    tree.send(top, 222).unwrap();
    let bytes = tree.save(1).unwrap();

    // A later version constructs a numbered router first.
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let screens = mount_screens(&mut tree, true);
    let top = *tree.get(screens).unwrap().stack.as_ref().unwrap().active();
    assert_eq!(value(&tree, top), 222);
}
