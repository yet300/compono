//! `Cx::destroy` and `Cx::set_child_phase` act on the component's own live
//! children and say when the target is anything else.

#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::{Component, Cx, Handle, NotOwnChild, Phase, Tree};

struct Node;

impl Component for Node {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

#[derive(Clone, Copy)]
enum Target {
    Child,
    Grandchild,
    Removed,
}

enum Msg {
    Destroy(Target),
    Lower(Target),
}

struct Root {
    child: Handle<Node>,
    grandchild: Handle<Node>,
    removed: Handle<Node>,
    result: Option<Result<(), NotOwnChild>>,
}

impl Root {
    const fn target(&self, target: Target) -> Handle<Node> {
        match target {
            Target::Child => self.child,
            Target::Grandchild => self.grandchild,
            Target::Removed => self.removed,
        }
    }
}

impl Component for Root {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        let target = |target| self.target(target);
        self.result = Some(match msg {
            Msg::Destroy(which) => cx.destroy(target(which)),
            Msg::Lower(which) => cx.set_child_phase(target(which), Phase::Hidden),
        });
    }
}

fn setup() -> (Tree, Handle<Root>, Root) {
    let mut tree = Tree::new(NullHost);
    let mut handles = None;
    let root = tree.mount(|cx| {
        let mut grandchild = None;
        let child = cx.child(
            |cx| {
                grandchild = Some(cx.child(|_cx| Node, |_node, (), _cx| {}));
                Node
            },
            |_root, (), _cx| {},
        );
        let removed = cx.child(|_cx| Node, |_root, (), _cx| {});
        cx.destroy(removed).unwrap();
        let root = Root {
            child,
            grandchild: grandchild.unwrap(),
            removed,
            result: None,
        };
        handles = Some((root.child, root.grandchild, root.removed));
        root
    });
    let (child, grandchild, removed) = handles.unwrap();
    tree.set_phase(Phase::Focused);
    let copy = Root {
        child,
        grandchild,
        removed,
        result: None,
    };
    (tree, root, copy)
}

fn result(tree: &Tree, root: Handle<Root>) -> Option<Result<(), NotOwnChild>> {
    tree.get(root).unwrap().result
}

#[test]
fn destroying_a_live_child_succeeds() {
    let (mut tree, root, handles) = setup();
    tree.send(root, Msg::Destroy(Target::Child)).unwrap();
    assert_eq!(result(&tree, root), Some(Ok(())));
    assert!(!tree.contains(handles.child.id()));
    assert!(!tree.contains(handles.grandchild.id()));
}

#[test]
fn destroying_a_grandchild_is_refused() {
    let (mut tree, root, handles) = setup();
    tree.send(root, Msg::Destroy(Target::Grandchild)).unwrap();
    assert_eq!(result(&tree, root), Some(Err(NotOwnChild)));
    assert!(
        tree.contains(handles.grandchild.id()),
        "nothing was removed"
    );
}

#[test]
fn destroying_a_removed_child_is_refused() {
    let (mut tree, root, _handles) = setup();
    tree.send(root, Msg::Destroy(Target::Removed)).unwrap();
    assert_eq!(result(&tree, root), Some(Err(NotOwnChild)));
}

#[test]
fn lowering_a_live_child_succeeds() {
    let (mut tree, root, handles) = setup();
    assert_eq!(tree.node_phase(handles.child.id()), Some(Phase::Focused));
    tree.send(root, Msg::Lower(Target::Child)).unwrap();
    assert_eq!(result(&tree, root), Some(Ok(())));
    assert_eq!(tree.node_phase(handles.child.id()), Some(Phase::Hidden));
}

#[test]
fn lowering_a_grandchild_is_refused() {
    let (mut tree, root, handles) = setup();
    tree.send(root, Msg::Lower(Target::Grandchild)).unwrap();
    assert_eq!(result(&tree, root), Some(Err(NotOwnChild)));
    assert_eq!(
        tree.node_phase(handles.grandchild.id()),
        Some(Phase::Focused),
        "nothing was lowered"
    );
}

#[test]
fn lowering_a_removed_child_is_refused() {
    let (mut tree, root, _handles) = setup();
    tree.send(root, Msg::Lower(Target::Removed)).unwrap();
    assert_eq!(result(&tree, root), Some(Err(NotOwnChild)));
}
