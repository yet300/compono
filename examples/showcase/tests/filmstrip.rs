//! `Filmstrip` follows the rules for routers, as the built-in ones do.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::router::Router;
use compono::{Commit, Component, Cx, Handle, Host, Phase, Tree};
use showcase::feature::gallery::Filmstrip;
use std::cell::Cell;
use std::rc::Rc;

struct NullHost;
impl Host for NullHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

/// Counts how many frames have been removed.
struct Frame(Rc<Cell<usize>>);

impl Component for Frame {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Drop for Frame {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

struct Owner {
    strip: Option<Filmstrip<Handle<Frame>, Owner>>,
    removed: Rc<Cell<usize>>,
    /// How many frames were gone when `destroy` returned.
    gone_at_once: Option<usize>,
}

impl Component for Owner {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        if let Some(strip) = self.strip.take() {
            strip.destroy(cx);
            self.gone_at_once = Some(self.removed.get());
        }
    }
}

#[test]
fn destroying_a_filmstrip_removes_every_frame_at_once() {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let removed = Rc::new(Cell::new(0));
    let counter = Rc::clone(&removed);
    let owner = tree.mount(move |cx| {
        let frames = Rc::clone(&counter);
        let strip = Filmstrip::new(1..=3, cx, move |_key, cx| {
            let frames = Rc::clone(&frames);
            cx.child(|_cx| Frame(frames), |_owner, (), _cx| {})
        });
        Owner {
            strip: Some(strip),
            removed: counter,
            gone_at_once: None,
        }
    });
    let frames: Vec<Handle<Frame>> = tree
        .get(owner)
        .unwrap()
        .strip
        .as_ref()
        .unwrap()
        .frames()
        .map(|(_key, frame, _selected)| *frame)
        .collect();

    tree.send(owner, ()).unwrap();

    assert_eq!(tree.get(owner).unwrap().gone_at_once, Some(3));
    assert!(frames.iter().all(|frame| tree.get(*frame).is_none()));
}
