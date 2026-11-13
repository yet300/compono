//! `Cx::send` reaches the component itself and its own children, and says when a
//! message reaches neither.

#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::{Component, Cx, Handle, NotOwnChild, Tree};
use std::cell::Cell;
use std::rc::Rc;

struct Leaf {
    got: u32,
}

impl Component for Leaf {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, amount: u32, _cx: &mut Cx<'_, Self>) {
        self.got += amount;
    }
}

/// The root's child, which owns the root's grandchild.
struct Middle {
    got: u32,
}

impl Component for Middle {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, amount: u32, _cx: &mut Cx<'_, Self>) {
        self.got += amount;
    }
}

enum Msg {
    Count(u32),
    ToSelf(Handle<Root>),
    ToChild,
    ToGrandchild,
    ToDestroyedChild,
}

struct Root {
    middle: Handle<Middle>,
    grandchild: Handle<Leaf>,
    counted: u32,
    sent: Option<Result<(), NotOwnChild>>,
}

impl Component for Root {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        let sent = match msg {
            Msg::Count(amount) => {
                self.counted += amount;
                return;
            }
            Msg::ToSelf(me) => cx.send(me, Msg::Count(1)),
            Msg::ToChild => cx.send(self.middle, 1),
            Msg::ToGrandchild => cx.send(self.grandchild, 1),
            Msg::ToDestroyedChild => {
                cx.destroy(self.middle).unwrap();
                cx.send(self.middle, 1)
            }
        };
        self.sent = Some(sent);
    }
}

fn mount(tree: &mut Tree) -> Handle<Root> {
    let found = Rc::new(Cell::new(None));
    let smuggled = Rc::clone(&found);
    tree.mount(move |cx| {
        let middle = cx.child(
            move |cx| {
                let leaf = cx.child(|_cx| Leaf { got: 0 }, |_middle: &mut Middle, (), _cx| {});
                smuggled.set(Some(leaf));
                Middle { got: 0 }
            },
            |_root: &mut Root, (), _cx| {},
        );
        Root {
            middle,
            grandchild: found.get().unwrap(),
            counted: 0,
            sent: None,
        }
    })
}

#[test]
fn a_message_to_itself_is_queued_and_delivered() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);

    tree.send(root, Msg::ToSelf(root)).unwrap();

    let root = tree.get(root).unwrap();
    assert_eq!(root.sent, Some(Ok(())));
    assert_eq!(root.counted, 1);
}

#[test]
fn a_message_to_its_own_child_is_queued_and_delivered() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);

    tree.send(root, Msg::ToChild).unwrap();

    let middle = tree.get(root).unwrap().middle;
    assert_eq!(tree.get(root).unwrap().sent, Some(Ok(())));
    assert_eq!(tree.get(middle).unwrap().got, 1);
}

#[test]
fn a_message_to_a_grandchild_is_refused() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);

    tree.send(root, Msg::ToGrandchild).unwrap();

    let grandchild = tree.get(root).unwrap().grandchild;
    assert_eq!(tree.get(root).unwrap().sent, Some(Err(NotOwnChild)));
    assert_eq!(tree.get(grandchild).unwrap().got, 0);
}

#[test]
fn a_message_to_a_destroyed_child_is_refused() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);

    tree.send(root, Msg::ToDestroyedChild).unwrap();

    assert_eq!(tree.get(root).unwrap().sent, Some(Err(NotOwnChild)));
}
