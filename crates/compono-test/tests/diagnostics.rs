//! A router or child that saves nothing is reported to the host once.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Component, Cx, Tree};
use compono_test::{DiagnosticRecord, headless};

struct Leaf;

impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

/// Runs `build` while mounting a root, and returns the root's id and what the
/// host received.
fn mounted(build: impl FnOnce(&mut Cx<'_, Leaf>)) -> (compono::NodeId, Vec<DiagnosticRecord>) {
    let (host, log) = headless();
    let mut tree = Tree::new(host);
    let root = tree.mount(|cx| {
        build(cx);
        Leaf
    });
    (root.id(), log.diagnostics())
}

#[test]
fn a_numbered_router_past_the_limit_is_reported() {
    let mut last = None;
    let (owner, diagnostics) = mounted(|cx| {
        for _ in 0..32_768 {
            last = Some(cx.router());
        }
    });
    assert!(
        !last.unwrap().is_saved(),
        "the 32 768th router saves nothing"
    );
    assert_eq!(diagnostics, [DiagnosticRecord::RouterLimit { owner }]);
}

#[test]
fn a_router_name_given_twice_is_reported() {
    let (owner, diagnostics) = mounted(|cx| {
        let first = cx.named_routers("tabs", Cx::router);
        let second = cx.named_routers("tabs", Cx::router);
        assert!(first.is_saved());
        assert!(!second.is_saved());
    });
    assert_eq!(
        diagnostics,
        [DiagnosticRecord::RepeatedRouterName { owner }]
    );
}

#[test]
fn a_child_name_a_live_sibling_uses_is_reported() {
    let (owner, diagnostics) = mounted(|cx| {
        cx.child_keyed("draft", |_cx| Leaf, |_root, (), _cx| {});
        cx.child_keyed("draft", |_cx| Leaf, |_root, (), _cx| {});
    });
    assert_eq!(
        diagnostics,
        [DiagnosticRecord::RepeatedChildName {
            owner,
            name: "draft".to_owned(),
        }]
    );
}
