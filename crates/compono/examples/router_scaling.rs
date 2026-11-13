//! Measures `Pages`, `Items`, `Panels` and `Slot` as the entries that are not on
//! screen grow. The time for one operation must stay flat.

use compono::router::items::Items;
use compono::router::pages::Pages;
use compono::router::panels::{PanelTypes, Panels, PanelsMode};
use compono::router::slot::Slot;
use compono::{Commit, Component, Cx, Handle, Host, Phase, Tree};
use std::time::Instant;

struct NoHost;
impl Host for NoHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

struct Leaf {
    _board: [u64; 8],
}
impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

fn leaf<P: Component>(seed: u32, cx: &mut Cx<'_, P>) -> Handle<Leaf> {
    let board = [u64::from(seed); 8];
    cx.child(move |_cx| Leaf { _board: board }, |_parent, (), _cx| {})
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

struct Root {
    pages: Pages<u32, Handle<Leaf>, Self>,
    items: Items<u32, Handle<Leaf>, Self>,
    panels: Panels<Three, Self>,
    slot: Slot<u32, Handle<Leaf>, Self>,
}

enum Msg {
    Page(usize),
    Window(usize),
    Details(u32),
    Dialog(u32),
}

impl Component for Root {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Page(index) => {
                self.pages.select(index, cx);
            }
            Msg::Window(start) => {
                let _unique = self.items.set_window(start..start + 30, cx);
            }
            Msg::Details(route) => {
                self.panels.set_details(Some(route), cx);
                self.panels.set_details(None, cx);
            }
            Msg::Dialog(route) => {
                self.slot.show(route, cx);
                self.slot.close(cx);
            }
        }
    }
}

const ROUNDS: u32 = 200_000;

fn per_round(mut operation: impl FnMut(u32)) -> u128 {
    let started = Instant::now();
    for round in 0..ROUNDS {
        operation(round);
    }
    started.elapsed().as_nanos() / u128::from(ROUNDS)
}

fn measure(size: u32) -> Option<[u128; 4]> {
    let mut tree = Tree::new(NoHost);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(|cx: &mut Cx<'_, Root>| {
        // The routes are distinct, so this cannot be refused.
        let Ok(pages) = Pages::new((0..size).collect(), 0, 1, cx, |route, cx| leaf(*route, cx))
        else {
            std::process::abort()
        };
        // Entries that exist but are not on screen: `size` hidden siblings.
        let parked: Vec<Handle<Leaf>> = (0..size).map(|seed| leaf(seed, cx)).collect();
        for handle in parked {
            // Created just above, so each is this component's own live child.
            let _own_child = cx.set_child_phase(handle, Phase::Hidden);
        }
        Root {
            pages,
            items: Items::new((0..size).collect(), cx, |key, cx| leaf(*key, cx)),
            panels: Panels::new(
                0,
                PanelsMode::Dual,
                cx,
                |route, cx| leaf(*route, cx),
                |route, cx| leaf(*route, cx),
                |route, cx| leaf(*route, cx),
            ),
            slot: Slot::new(cx, |route, cx| leaf(*route, cx)),
        }
    });
    let span = usize::try_from(size.saturating_sub(40)).ok()?.max(1);
    let at = |round: u32| usize::try_from(round).unwrap_or(0) % span;
    Some([
        per_round(|round| {
            let _stale = tree.send(root, Msg::Page(at(round)));
        }),
        per_round(|round| {
            let _stale = tree.send(root, Msg::Window(at(round)));
        }),
        per_round(|round| {
            let _stale = tree.send(root, Msg::Details(round));
        }),
        per_round(|round| {
            let _stale = tree.send(root, Msg::Dialog(round));
        }),
    ])
}

fn main() {
    for size in [100, 10_000, 1_000_000] {
        match measure(size) {
            Some([page, window, panel, dialog]) => println!(
                "{size:>8} entries: select page {page} ns, scroll one row {window} ns, \
                 open and close details {panel} ns, open and close dialog {dialog} ns"
            ),
            None => println!("{size:>8} entries: setup failed"),
        }
    }
}
