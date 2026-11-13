//! The keys a UI reads to animate `Slot`, `Pages` and `Panels`.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

mod common;

use common::NullHost;
use compono::router::pages::Pages;
use compono::router::panels::{PanelKeys, PanelTypes, Panels, PanelsMode};
use compono::router::slot::Slot;
use compono::{Component, Cx, Handle, Phase, Tree};

struct Types;

impl PanelTypes for Types {
    type Main = u8;
    type MainChild = u8;
    type Details = u8;
    type DetailsChild = u8;
    type Extra = u8;
    type ExtraChild = u8;
}

struct Root {
    slot: Slot<u8, u8, Root>,
    pages: Pages<u8, u8, Root>,
    panels: Panels<Types, Root>,
}

type Change = Box<dyn FnOnce(&mut Root, &mut Cx<'_, Root>)>;

impl Component for Root {
    type Msg = Change;
    type Output = ();
    fn update(&mut self, change: Change, cx: &mut Cx<'_, Self>) {
        change(self, cx);
    }
}

fn mount() -> (Tree, Handle<Root>) {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(|cx| Root {
        slot: Slot::new(cx, |route, _cx| *route),
        pages: Pages::new(vec![10, 11, 12, 13], 0, 1, cx, |route, _cx| *route).unwrap(),
        panels: Panels::new(
            1,
            PanelsMode::Single,
            cx,
            |route, _cx| *route,
            |route, _cx| *route,
            |route, _cx| *route,
        ),
    });
    (tree, root)
}

fn change(
    tree: &mut Tree,
    root: Handle<Root>,
    f: impl FnOnce(&mut Root, &mut Cx<'_, Root>) + 'static,
) {
    tree.send(root, Box::new(f)).unwrap();
}

#[test]
fn a_slot_gives_each_visit_its_own_key() {
    let (mut tree, root) = mount();
    assert_eq!(tree.get(root).unwrap().slot.active_key(), None);

    change(&mut tree, root, |r, cx| {
        let _ = r.slot.show(7, cx);
    });
    let first = tree.get(root).unwrap().slot.active_key();
    assert!(first.is_some());

    // The same route shown again is a new visit.
    change(&mut tree, root, |r, cx| {
        let _ = r.slot.show(7, cx);
    });
    let second = tree.get(root).unwrap().slot.active_key();
    assert!(second.is_some());
    assert_ne!(first, second);

    change(&mut tree, root, |r, cx| {
        let _ = r.slot.close(cx);
    });
    assert_eq!(tree.get(root).unwrap().slot.active_key(), None);

    change(&mut tree, root, |r, cx| {
        let _ = r.slot.show(8, cx);
    });
    let third = tree.get(root).unwrap().slot.active_key();
    assert!(third.is_some() && third != second && third != first);
}

fn pages(tree: &Tree, root: Handle<Root>) -> [Option<u8>; 4] {
    let pages = &tree.get(root).unwrap().pages;
    [0, 1, 2, 3].map(|index| pages.child(index).copied())
}

#[test]
fn pages_give_every_page_that_exists() {
    let (mut tree, root) = mount();
    // Selected 0 with `retain` 1: pages 0 and 1 exist.
    assert_eq!(pages(&tree, root), [Some(10), Some(11), None, None]);
    assert_eq!(tree.get(root).unwrap().pages.child(9), None);

    change(&mut tree, root, |r, cx| assert!(r.pages.select(2, cx)));
    assert_eq!(pages(&tree, root), [None, Some(11), Some(12), Some(13)]);
    assert_eq!(tree.get(root).unwrap().pages.active(), Some(&12));

    change(&mut tree, root, |r, cx| {
        r.pages.set_routes(vec![13, 12], cx).unwrap()
    });
    // Selected stays at index 1, now route 12.
    assert_eq!(pages(&tree, root), [Some(13), Some(12), None, None]);
}

fn keys(tree: &Tree, root: Handle<Root>) -> PanelKeys {
    tree.get(root).unwrap().panels.keys()
}

#[test]
fn panels_give_a_key_for_each_open_panel() {
    let (mut tree, root) = mount();
    let start = keys(&tree, root);
    assert!(start.main.is_some());
    assert_eq!((start.details, start.extra), (None, None));

    change(&mut tree, root, |r, cx| r.panels.set_details(Some(2), cx));
    let details = keys(&tree, root);
    assert_eq!(details.main, start.main);
    assert!(details.details.is_some());

    change(&mut tree, root, |r, cx| r.panels.set_extra(Some(3), cx));
    let extra = keys(&tree, root);
    assert_eq!((extra.main, extra.details), (start.main, details.details));
    assert!(extra.extra.is_some());

    // Showing other details is a new visit; it closes nothing else.
    change(&mut tree, root, |r, cx| r.panels.set_details(Some(4), cx));
    let other = keys(&tree, root);
    assert_ne!(other.details, details.details);
    assert_eq!(other.extra, extra.extra);

    // The mode changes what fits, not what is open.
    change(&mut tree, root, |r, cx| {
        r.panels.set_mode(PanelsMode::Triple, cx)
    });
    assert_eq!(keys(&tree, root), other);

    change(&mut tree, root, |r, cx| assert!(r.panels.pop(cx)));
    assert_eq!(keys(&tree, root).extra, None);

    change(&mut tree, root, |r, cx| r.panels.set_main(5, cx));
    let main = keys(&tree, root);
    assert_ne!(main.main, start.main);
    assert_eq!(main.details, other.details);

    change(&mut tree, root, |r, cx| r.panels.set_details(None, cx));
    assert_eq!(keys(&tree, root).details, None);
}
