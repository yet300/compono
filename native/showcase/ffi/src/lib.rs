//! The showcase as SwiftUI and Compose see it.
//!
//! [`ShowcaseApp`] owns the tree and has the standard methods the Swift package and
//! the Compose library call. Every screen is a component object, as in Decompose:
//! `RootComponent` gives `TabsComponent`, which gives `CountersComponent`, and so on.
//! Each has a `model` and its actions as methods. Component objects hold the tree
//! weakly and answer with nothing once their component or the application is gone.

mod component;
mod feature;

use boltffi::export;
use component::{Ctx, checked};
use compono::{Handle, Phase};
use compono_ffi::{NO_NODE, NativeTree, Platform, phase_from_code};
use compono_tokio::TokioSpawner;
use feature::root::{RootComponent, root_component};
use showcase::Services;
use showcase::feature::root::Root;
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Builder, Runtime};

boltffi::scaffolding!();

/// Bumped when saved state stops being readable.
const SCHEMA: u32 = 3;

/// What the native side receives from the tree.
#[export]
pub trait Listener: Send + Sync {
    /// One finished change. `dirty` holds the addresses of changed components.
    fn commit(&self, dirty: Vec<i64>, can_back: bool);
    /// Asks for `ShowcaseApp::pump` on the main thread. Called from any thread.
    fn schedule_pump(&self);
}

struct Forward(Arc<dyn Listener>);

impl Platform for Forward {
    fn commit(&self, dirty: &[u64], can_back: bool) {
        self.0
            .commit(dirty.iter().map(|id| id.cast_signed()).collect(), can_back);
    }

    fn schedule_pump(&self) {
        self.0.schedule_pump();
    }
}

fn runtime() -> Option<&'static Runtime> {
    static RUNTIME: OnceLock<Option<Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(1)
                .enable_time()
                .build()
                .ok()
        })
        .as_ref()
}

/// The application's tree. Create it and call it on the main thread.
#[export]
pub struct ShowcaseApp {
    tree: Arc<NativeTree>,
    root: Option<Handle<Root>>,
}

#[export]
impl ShowcaseApp {
    /// Builds the tree on this thread. `saved` is empty on a first start.
    pub fn new(listener: Arc<dyn Listener>, saved: Vec<u8>) -> Self {
        let platform: Arc<dyn Platform> = Arc::new(Forward(listener));
        let tree = if saved.is_empty() {
            NativeTree::new(platform)
        } else {
            // Unreadable state starts the application from the beginning.
            NativeTree::restore(platform, &saved, SCHEMA).0
        };
        let root = checked(tree.with(|tree| {
            if let Some(runtime) = runtime() {
                tree.set_spawner(TokioSpawner::new(runtime.handle().clone()));
            }
            // The timer is created inside the future, where the runtime is current.
            let services =
                Services::new(|delay| Box::pin(async move { tokio::time::sleep(delay).await }));
            tree.mount_persistent(move |cx| Root::new(cx, services))
        }));
        Self {
            tree: Arc::new(tree),
            root,
        }
    }

    /// The root component.
    pub fn root(&self) -> RootComponent {
        root_component(Ctx::new(&self.tree), self.root)
    }

    pub fn pump(&self) {
        checked(self.tree.pump());
    }

    /// Sets the phase: 0 for hidden, 1 for visible and 2 for focused.
    pub fn set_phase(&self, code: i32) {
        let phase = u8::try_from(code).ok().and_then(phase_from_code);
        debug_assert!(phase.is_some(), "unknown phase code {code}");
        checked(self.tree.set_phase(phase.unwrap_or(Phase::Hidden)));
    }

    pub fn back(&self) -> bool {
        checked(self.tree.back()).unwrap_or(false)
    }

    /// Starts a Back gesture and returns its target's address, or `NO_NODE`.
    pub fn back_start(&self) -> i64 {
        checked(self.tree.back_start())
            .flatten()
            .map_or(NO_NODE, u64::cast_signed)
    }

    pub fn back_cancel(&self) {
        checked(self.tree.back_cancel());
    }

    pub fn back_commit(&self) -> bool {
        checked(self.tree.back_commit()).unwrap_or(false)
    }

    /// Encodes navigation and persistent state. Empty if it could not be saved.
    pub fn save(&self) -> Vec<u8> {
        checked(self.tree.save(SCHEMA)).unwrap_or_default()
    }

    pub fn contains(&self, node: i64) -> bool {
        let node = compono::NodeId::from_bits(node.cast_unsigned());
        checked(self.tree.read(|tree| tree.contains(node))).unwrap_or(false)
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::{Listener, ShowcaseApp};
    use std::sync::Arc;

    struct Silent;
    impl Listener for Silent {
        fn commit(&self, _dirty: Vec<i64>, _can_back: bool) {}
        fn schedule_pump(&self) {}
    }

    fn app() -> ShowcaseApp {
        ShowcaseApp::new(Arc::new(Silent), Vec::new())
    }

    #[test]
    fn a_call_from_another_thread_stops_a_debug_build() {
        let app = app();
        let elsewhere = std::thread::scope(|scope| scope.spawn(|| app.back()).join());
        assert!(elsewhere.is_err(), "the wrong thread went unnoticed");
    }

    #[test]
    #[should_panic(expected = "phase code")]
    fn an_unknown_phase_code_stops_a_debug_build() {
        app().set_phase(7);
    }

    #[test]
    fn a_child_of_another_kind_gives_no_component() {
        let app = app();
        let root = app.root();
        let tabs = root.active().unwrap();
        assert!(root.book(tabs).is_none());
        assert!(root.tabs(tabs).is_some());
    }

    #[test]
    fn no_back_handler_is_reported_as_no_node() {
        let app = app();
        app.set_phase(2);
        assert_eq!(app.back_start(), compono_ffi::NO_NODE);
    }

    #[test]
    fn a_call_on_the_owning_thread_works() {
        let app = app();
        app.set_phase(2);
        app.pump();
        assert!(!app.back(), "the root screen handles no Back");
        assert!(!app.save().is_empty());
    }
}

#[cfg(test)]
mod kinds {
    use super::{Listener, ShowcaseApp};
    use crate::component::{ChildRef, Ctx};
    use crate::feature::articles::{ArticlesComponent, PanelChild, PanelKind};
    use crate::feature::root::{ScreenChild, ScreenKind};
    use crate::feature::tabs::{TabKind, TabsChild, TabsComponent};
    use std::sync::Arc;

    struct Silent;
    impl Listener for Silent {
        fn commit(&self, _dirty: Vec<i64>, _can_back: bool) {}
        fn schedule_pump(&self) {}
    }

    const CHILD: ChildRef = ChildRef {
        key: 0,
        node: 0,
        phase: 0,
    };

    #[test]
    fn a_screen_accessor_gives_nothing_for_another_kind() {
        let app = ShowcaseApp::new(Arc::new(Silent), Vec::new());
        let root = app.root();
        for kind in [ScreenKind::Tabs, ScreenKind::Book, ScreenKind::Gallery] {
            let child = ScreenChild { kind, child: CHILD };
            assert_eq!(root.tabs(child).is_some(), kind == ScreenKind::Tabs);
            assert_eq!(root.book(child).is_some(), kind == ScreenKind::Book);
            assert_eq!(root.gallery(child).is_some(), kind == ScreenKind::Gallery);
        }
    }

    #[test]
    fn a_tab_accessor_gives_nothing_for_another_kind() {
        let app = ShowcaseApp::new(Arc::new(Silent), Vec::new());
        let tabs = TabsComponent {
            ctx: Ctx::new(&app.tree),
            node: 0,
        };
        let all = [
            TabKind::Menu,
            TabKind::Counters,
            TabKind::Cards,
            TabKind::Articles,
            TabKind::Feed,
        ];
        for tab in all {
            let child = TabsChild { tab, child: CHILD };
            assert_eq!(tabs.menu(child).is_some(), tab == TabKind::Menu);
            assert_eq!(tabs.counters(child).is_some(), tab == TabKind::Counters);
            assert_eq!(tabs.cards(child).is_some(), tab == TabKind::Cards);
            assert_eq!(tabs.articles(child).is_some(), tab == TabKind::Articles);
            assert_eq!(tabs.feed(child).is_some(), tab == TabKind::Feed);
        }
    }

    #[test]
    fn a_panel_accessor_gives_nothing_for_another_kind() {
        let app = ShowcaseApp::new(Arc::new(Silent), Vec::new());
        let articles = ArticlesComponent {
            ctx: Ctx::new(&app.tree),
            node: 0,
        };
        for kind in [PanelKind::Main, PanelKind::Details, PanelKind::Extra] {
            let panel = PanelChild { kind, child: CHILD };
            assert_eq!(articles.list(panel).is_some(), kind == PanelKind::Main);
            assert_eq!(
                articles.details(panel).is_some(),
                kind == PanelKind::Details
            );
            assert_eq!(articles.author(panel).is_some(), kind == PanelKind::Extra);
        }
    }
}
