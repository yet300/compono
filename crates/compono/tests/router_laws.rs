//! Laws of `Slot`, `Pages`, `Panels` and `Items` after any sequence of operations.

#![cfg(all(feature = "std", feature = "serde"))]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

mod common;

use common::NullHost;
use compono::router::items::{Items, SavedItems};
use compono::router::pages::{Pages, SavedPages};
use compono::router::panels::{PanelTypes, Panels, PanelsMode, SavedPanels};
use compono::router::slot::{SavedSlot, Slot};
use compono::{Component, Cx, Handle, Persist, Phase, Tree};
use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts live instances, so a leaked or doubly dropped child shows up.
struct Leaf {
    name: u8,
    live: Arc<AtomicUsize>,
}

impl Drop for Leaf {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Persist for Leaf {
    type Saved = u8;
    fn save(&self) -> u8 {
        self.name
    }
}

fn leaf<P: Component>(name: u8, live: &Arc<AtomicUsize>, cx: &mut Cx<'_, P>) -> Handle<Leaf> {
    live.fetch_add(1, Ordering::Relaxed);
    let live = Arc::clone(live);
    cx.persistent_child(move |_cx| Leaf { name, live }, |_parent, (), _cx| {})
}

fn build<P: Component>(
    live: &Arc<AtomicUsize>,
) -> impl Fn(&u8, &mut Cx<'_, P>) -> Handle<Leaf> + 'static {
    let live = Arc::clone(live);
    move |name, cx| leaf(*name, &live, cx)
}

fn phase() -> impl Strategy<Value = Phase> {
    prop_oneof![
        Just(Phase::Hidden),
        Just(Phase::Visible),
        Just(Phase::Focused)
    ]
}

fn focused_tree() -> Tree {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    tree
}

// ---- Slot ----

#[derive(Clone, Debug)]
enum SlotOp {
    Show(u8),
    Close,
    Phase(Phase),
}

struct Dialog {
    slot: Slot<u8, Handle<Leaf>, Self>,
}

impl Component for Dialog {
    type Msg = SlotOp;
    type Output = ();
    fn update(&mut self, op: SlotOp, cx: &mut Cx<'_, Self>) {
        match op {
            SlotOp::Show(route) => {
                self.slot.show(route, cx);
            }
            SlotOp::Close => {
                self.slot.close(cx);
            }
            SlotOp::Phase(phase) => self.slot.set_phase(phase, cx),
        }
    }
}

impl Persist for Dialog {
    type Saved = SavedSlot<u8>;
    fn save(&self) -> SavedSlot<u8> {
        self.slot.snapshot()
    }
}

fn dialog(tree: &mut Tree, live: &Arc<AtomicUsize>) -> Handle<Dialog> {
    let build = build(live);
    tree.mount_persistent(move |cx| {
        let saved = cx.restored();
        Dialog {
            slot: Slot::restore(saved, cx, build),
        }
    })
}

fn slot_op() -> impl Strategy<Value = SlotOp> {
    prop_oneof![
        any::<u8>().prop_map(SlotOp::Show),
        Just(SlotOp::Close),
        phase().prop_map(SlotOp::Phase)
    ]
}

proptest! {
    #[test]
    fn a_slot_holds_at_most_one_entry_at_its_granted_phase(
        ops in prop::collection::vec(slot_op(), 0..30),
    ) {
        let live = Arc::new(AtomicUsize::new(0));
        let mut tree = focused_tree();
        let root = dialog(&mut tree, &live);
        let mut route = None;
        let mut granted = Phase::Focused;
        for op in ops {
            match &op {
                SlotOp::Show(shown) => route = Some(*shown),
                SlotOp::Close => route = None,
                SlotOp::Phase(phase) => granted = *phase,
            }
            tree.send(root, op).unwrap();
            let slot = &tree.get(root).unwrap().slot;
            prop_assert_eq!(slot.route().copied(), route);
            prop_assert_eq!(slot.is_open(), route.is_some());
            prop_assert_eq!(slot.phase(), granted);
            prop_assert_eq!(live.load(Ordering::Relaxed), usize::from(route.is_some()));
            let phase = slot.active().and_then(|child| tree.phase(*child));
            prop_assert_eq!(phase, route.map(|_| granted));
        }
        let bytes = tree.save(1).unwrap();
        let again = Arc::new(AtomicUsize::new(0));
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let root = dialog(&mut tree, &again);
        prop_assert_eq!(tree.get(root).unwrap().slot.route().copied(), route);
    }
}

// ---- Pages ----

#[derive(Clone, Debug)]
enum PagesOp {
    Select(usize),
    Routes(Vec<u8>),
}

struct Book {
    pages: Pages<u8, Handle<Leaf>, Self>,
}

impl Component for Book {
    type Msg = PagesOp;
    type Output = ();
    fn update(&mut self, op: PagesOp, cx: &mut Cx<'_, Self>) {
        match op {
            PagesOp::Select(index) => {
                self.pages.select(index, cx);
            }
            PagesOp::Routes(routes) => self.pages.set_routes(routes, cx).unwrap(),
        }
    }
}

impl Persist for Book {
    type Saved = SavedPages<u8>;
    fn save(&self) -> SavedPages<u8> {
        self.pages.snapshot()
    }
}

const RETAIN: usize = 1;

fn book(tree: &mut Tree, live: &Arc<AtomicUsize>) -> Handle<Book> {
    let build = build(live);
    tree.mount_persistent(move |cx| {
        let saved = cx.restored();
        Book {
            pages: Pages::restore_or(saved, vec![0, 1, 2], RETAIN, cx, build).unwrap(),
        }
    })
}

fn unique_routes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::hash_set(0u8..12, 0..8).prop_map(|set| set.into_iter().collect())
}

fn pages_op() -> impl Strategy<Value = PagesOp> {
    prop_oneof![
        (0usize..10).prop_map(PagesOp::Select),
        unique_routes().prop_map(PagesOp::Routes)
    ]
}

/// Checks the pages against the plain list and selection they should show.
fn check_pages(
    tree: &Tree,
    root: Handle<Book>,
    routes: &[u8],
    selected: usize,
) -> Result<(), TestCaseError> {
    let pages = &tree.get(root).unwrap().pages;
    prop_assert_eq!(pages.routes(), routes);
    prop_assert_eq!(pages.selected(), selected);
    let near = routes
        .iter()
        .enumerate()
        .filter(|(index, _)| index.abs_diff(selected) <= RETAIN)
        .count();
    prop_assert_eq!(pages.live(), near);
    let active = pages.active().map(|child| tree.get(*child).unwrap().name);
    prop_assert_eq!(active, routes.get(selected).copied());
    let phase = pages.active().and_then(|child| tree.phase(*child));
    prop_assert_eq!(phase, routes.get(selected).map(|_| Phase::Focused));
    Ok(())
}

proptest! {
    #[test]
    fn pages_keep_only_the_neighbours_of_the_selection_alive(
        ops in prop::collection::vec(pages_op(), 0..30),
    ) {
        let live = Arc::new(AtomicUsize::new(0));
        let mut tree = focused_tree();
        let root = book(&mut tree, &live);
        let mut routes = vec![0, 1, 2];
        let mut selected = 0;
        for op in ops {
            match &op {
                PagesOp::Select(index) if *index < routes.len() => selected = *index,
                PagesOp::Select(_) => {}
                PagesOp::Routes(new) => {
                    selected = selected.min(new.len().saturating_sub(1));
                    routes.clone_from(new);
                }
            }
            tree.send(root, op).unwrap();
            check_pages(&tree, root, &routes, selected)?;
            prop_assert_eq!(live.load(Ordering::Relaxed), tree.get(root).unwrap().pages.live());
        }
        let bytes = tree.save(1).unwrap();
        let again = Arc::new(AtomicUsize::new(0));
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        tree.set_phase(Phase::Focused);
        let root = book(&mut tree, &again);
        check_pages(&tree, root, &routes, selected)?;
    }
}

// ---- Panels ----

struct Three;
impl PanelTypes for Three {
    type Main = u8;
    type MainChild = Handle<Leaf>;
    type Details = u8;
    type DetailsChild = Handle<Leaf>;
    type Extra = u8;
    type ExtraChild = Handle<Leaf>;
}

#[derive(Clone, Debug)]
enum PanelsOp {
    Main(u8),
    Details(Option<u8>),
    Extra(Option<u8>),
    Mode(PanelsMode),
    Pop,
}

struct Screen {
    panels: Panels<Three, Self>,
}

impl Component for Screen {
    type Msg = PanelsOp;
    type Output = ();
    fn update(&mut self, op: PanelsOp, cx: &mut Cx<'_, Self>) {
        match op {
            PanelsOp::Main(route) => self.panels.set_main(route, cx),
            PanelsOp::Details(route) => self.panels.set_details(route, cx),
            PanelsOp::Extra(route) => self.panels.set_extra(route, cx),
            PanelsOp::Mode(mode) => self.panels.set_mode(mode, cx),
            PanelsOp::Pop => {
                self.panels.pop(cx);
            }
        }
    }
}

impl Persist for Screen {
    type Saved = SavedPanels<u8, u8, u8>;
    fn save(&self) -> Self::Saved {
        self.panels.snapshot()
    }
}

fn screen(tree: &mut Tree, mode: PanelsMode, live: &Arc<AtomicUsize>) -> Handle<Screen> {
    let (main, details, extra) = (build(live), build(live), build(live));
    tree.mount_persistent(move |cx| {
        let saved = cx.restored();
        Screen {
            panels: Panels::restore_or(saved, 0, mode, cx, main, details, extra),
        }
    })
}

fn mode() -> impl Strategy<Value = PanelsMode> {
    prop_oneof![
        Just(PanelsMode::Single),
        Just(PanelsMode::Dual),
        Just(PanelsMode::Triple)
    ]
}

fn panels_op() -> impl Strategy<Value = PanelsOp> {
    prop_oneof![
        any::<u8>().prop_map(PanelsOp::Main),
        prop::option::of(any::<u8>()).prop_map(PanelsOp::Details),
        prop::option::of(any::<u8>()).prop_map(PanelsOp::Extra),
        mode().prop_map(PanelsOp::Mode),
        Just(PanelsOp::Pop)
    ]
}

/// The plain values the panels should show.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Model {
    main: u8,
    details: Option<u8>,
    extra: Option<u8>,
    mode: PanelsMode,
}

impl Model {
    fn apply(&mut self, op: &PanelsOp) {
        match op {
            PanelsOp::Main(route) => self.main = *route,
            PanelsOp::Details(None) => (self.details, self.extra) = (None, None),
            PanelsOp::Details(route) => self.details = *route,
            PanelsOp::Extra(route) => self.extra = *route,
            PanelsOp::Mode(mode) => self.mode = *mode,
            PanelsOp::Pop if self.extra.is_some() => self.extra = None,
            PanelsOp::Pop => self.details = None,
        }
    }

    /// The phase of main, details and extra: the deepest is focused, and as
    /// many of the next deepest as the mode has room for are visible.
    fn phases(&self) -> [Option<Phase>; 3] {
        let open = [true, self.details.is_some(), self.extra.is_some()];
        let room = match self.mode {
            PanelsMode::Single => 1,
            PanelsMode::Dual => 2,
            PanelsMode::Triple => 3,
        };
        let mut phases = [None; 3];
        let mut placed = 0;
        for index in (0..3).rev() {
            if open[index] {
                phases[index] = Some(match placed {
                    0 => Phase::Focused,
                    shown if shown < room => Phase::Visible,
                    _ => Phase::Hidden,
                });
                placed += 1;
            }
        }
        phases
    }
}

fn check_panels(tree: &Tree, root: Handle<Screen>, model: &Model) -> Result<(), TestCaseError> {
    let panels = &tree.get(root).unwrap().panels;
    prop_assert_eq!(panels.main_route().copied(), Some(model.main));
    prop_assert_eq!(panels.details_route().copied(), model.details);
    prop_assert_eq!(panels.extra_route().copied(), model.extra);
    prop_assert_eq!(panels.mode(), model.mode);
    let phases = [
        panels.main().and_then(|child| tree.phase(*child)),
        panels.details().and_then(|child| tree.phase(*child)),
        panels.extra().and_then(|child| tree.phase(*child)),
    ];
    prop_assert_eq!(phases, model.phases());
    let on_screen = phases
        .iter()
        .flatten()
        .filter(|phase| **phase >= Phase::Visible);
    prop_assert_eq!(panels.visible().count(), on_screen.count());
    prop_assert_eq!(
        panels.can_pop(),
        model.details.is_some() || model.extra.is_some()
    );
    Ok(())
}

proptest! {
    #[test]
    fn panels_focus_the_deepest_and_show_what_the_mode_has_room_for(
        first in mode(),
        ops in prop::collection::vec(panels_op(), 0..30),
    ) {
        let live = Arc::new(AtomicUsize::new(0));
        let mut tree = focused_tree();
        let root = screen(&mut tree, first, &live);
        let mut model = Model { main: 0, details: None, extra: None, mode: first };
        check_panels(&tree, root, &model)?;
        for op in ops {
            model.apply(&op);
            tree.send(root, op).unwrap();
            check_panels(&tree, root, &model)?;
            let open = 1 + usize::from(model.details.is_some()) + usize::from(model.extra.is_some());
            prop_assert_eq!(live.load(Ordering::Relaxed), open);
        }
        let bytes = tree.save(1).unwrap();
        let again = Arc::new(AtomicUsize::new(0));
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        tree.set_phase(Phase::Focused);
        let root = screen(&mut tree, model.mode, &again);
        check_panels(&tree, root, &model)?;
    }
}

// ---- Items ----

#[derive(Clone, Debug)]
enum ItemsOp {
    Window(usize, usize),
    Keys(Vec<u8>),
}

struct Feed {
    items: Items<u8, Handle<Leaf>, Self>,
}

impl Component for Feed {
    type Msg = ItemsOp;
    type Output = ();
    fn update(&mut self, op: ItemsOp, cx: &mut Cx<'_, Self>) {
        match op {
            ItemsOp::Window(start, end) => {
                self.items.set_window(start..end, cx).unwrap();
            }
            ItemsOp::Keys(keys) => self.items.set_keys(keys, cx).unwrap(),
        }
    }
}

impl Persist for Feed {
    type Saved = SavedItems<u8>;
    fn save(&self) -> SavedItems<u8> {
        self.items.snapshot()
    }
}

fn feed(tree: &mut Tree, keys: Vec<u8>, live: &Arc<AtomicUsize>) -> Handle<Feed> {
    let build = build(live);
    tree.mount_persistent(move |cx| {
        let saved = cx.restored();
        Feed {
            items: Items::restore_or(saved, keys, cx, build).unwrap(),
        }
    })
}

fn items_op() -> impl Strategy<Value = ItemsOp> {
    prop_oneof![
        (0usize..40, 0usize..40).prop_map(|(start, end)| ItemsOp::Window(start, end)),
        prop::collection::hash_set(any::<u8>(), 0..30)
            .prop_map(|set| ItemsOp::Keys(set.into_iter().collect()))
    ]
}

fn check_items(
    tree: &Tree,
    root: Handle<Feed>,
    keys: &[u8],
    window: &core::ops::Range<usize>,
) -> Result<(), TestCaseError> {
    let items = &tree.get(root).unwrap().items;
    prop_assert_eq!(items.len(), keys.len());
    prop_assert_eq!(items.window(), window.clone());
    prop_assert_eq!(items.live(), window.len());
    for (index, key) in keys.iter().enumerate() {
        prop_assert_eq!(items.key(index), Some(key));
        let row = items
            .child(index)
            .map(|child| tree.get(*child).unwrap().name);
        prop_assert_eq!(row, window.contains(&index).then_some(*key));
        if let Some(child) = items.child(index) {
            prop_assert_eq!(tree.phase(*child), Some(Phase::Visible));
        }
    }
    let shown: Vec<usize> = items.visible().map(|(index, _key, _child)| index).collect();
    prop_assert_eq!(shown, window.clone().collect::<Vec<_>>());
    Ok(())
}

proptest! {
    #[test]
    fn a_list_has_a_component_for_each_row_in_its_window_and_no_other(
        ops in prop::collection::vec(items_op(), 0..30),
    ) {
        let live = Arc::new(AtomicUsize::new(0));
        let mut tree = focused_tree();
        let mut keys: Vec<u8> = (0..20).collect();
        let root = feed(&mut tree, keys.clone(), &live);
        let mut window = 0..0;
        for op in ops {
            let before: HashSet<(u8, Handle<Leaf>)> = {
                let items = &tree.get(root).unwrap().items;
                items.visible().map(|(_index, key, child)| (*key, *child)).collect()
            };
            match &op {
                ItemsOp::Window(start, end) => {
                    let end = (*end).min(keys.len());
                    window = (*start).min(end)..end;
                }
                ItemsOp::Keys(new) => {
                    keys.clone_from(new);
                    let end = window.end.min(keys.len());
                    window = window.start.min(end)..end;
                }
            }
            tree.send(root, op).unwrap();
            check_items(&tree, root, &keys, &window)?;
            prop_assert_eq!(live.load(Ordering::Relaxed), window.len());
            let items = &tree.get(root).unwrap().items;
            for (_index, key, child) in items.visible() {
                let kept = before.iter().find(|(old, _)| old == key);
                if let Some((_key, old)) = kept {
                    prop_assert_eq!(old, child, "a row that stays keeps its component");
                }
            }
        }
        let bytes = tree.save(1).unwrap();
        let again = Arc::new(AtomicUsize::new(0));
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        tree.set_phase(Phase::Focused);
        let root = feed(&mut tree, keys.clone(), &again);
        check_items(&tree, root, &keys, &window)?;
    }
}
