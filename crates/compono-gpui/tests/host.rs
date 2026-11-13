//! `GpuiHost` under the host conformance suite.

use compono::{Component, Cx, Tree, WakeSignal};
use compono_gpui::GpuiHost;
use compono_test::check_host;
use std::sync::{Arc, Mutex, PoisonError};

#[test]
fn gpui_host_conforms() {
    let latest: Arc<Mutex<Option<WakeSignal>>> = Arc::default();
    let writer = Arc::clone(&latest);
    let result = check_host(
        move || {
            let host = GpuiHost::new();
            *writer.lock().unwrap_or_else(PoisonError::into_inner) = Some(host.signal());
            host
        },
        move || {
            let signal = latest.lock().unwrap_or_else(PoisonError::into_inner);
            usize::from(signal.as_ref().is_some_and(WakeSignal::is_pending))
        },
    );
    assert_eq!(result, Ok(()));
}

struct Quiet;
impl Component for Quiet {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.unchanged();
    }
}

struct Loud;
impl Component for Loud {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

#[test]
fn only_real_changes_are_recorded() {
    let host = GpuiHost::new();
    let changes = host.changes();
    let mut tree = Tree::new(host);
    let quiet = tree.mount(|_cx| Quiet);
    let _mounted = changes.take();
    assert_eq!(tree.send(quiet, ()), Ok(()));
    assert!(changes.take().is_empty());
    let loud = tree.mount(|_cx| Loud);
    let _mounted = changes.take();
    assert_eq!(tree.send(loud, ()), Ok(()));
    assert_eq!(changes.take(), [loud.id()]);
    assert!(changes.take().is_empty(), "taking resets the record");
}
