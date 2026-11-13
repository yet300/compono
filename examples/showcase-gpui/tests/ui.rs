//! The showcase through real GPUI views, on GPUI's test platform.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests assert by failing")]

use compono::Phase;
use compono_gpui::update;
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};
use showcase::Services;
use showcase::feature::root::Child;
use showcase_gpui::ShowcaseView;

fn open(cx: &mut TestAppContext) -> (Entity<ShowcaseView>, &mut VisualTestContext) {
    resume(cx, None)
}

fn resume(
    cx: &mut TestAppContext,
    saved: Option<Vec<u8>>,
) -> (Entity<ShowcaseView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let executor = cx.executor();
    let services = Services::new(move |delay| Box::pin(executor.timer(delay)));
    let (view, cx) =
        cx.add_window_view(|window, cx| ShowcaseView::new(window, cx, services, saved.as_deref()));
    view.update(cx, |view, cx| {
        update(view, cx, |tree| tree.set_phase(Phase::Focused));
    });
    cx.run_until_parked();
    (view, cx)
}

fn click(cx: &mut VisualTestContext, name: &'static str) {
    let bounds = cx
        .debug_bounds(name)
        .unwrap_or_else(|| panic!("{name} is not on screen"));
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
}

fn drawn(cx: &mut VisualTestContext, name: &'static str) -> bool {
    cx.debug_bounds(name).is_some()
}

fn at_tabs(view: &Entity<ShowcaseView>, cx: &VisualTestContext) -> bool {
    view.read_with(cx, |view, _cx| {
        let root = view.tree().get(view.root()).unwrap();
        matches!(root.stack.active(), Child::Tabs(_))
    })
}

#[gpui::test]
fn every_navigation_model_can_be_used_from_the_screen(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    assert!(drawn(cx, "Showcase"));
    click(cx, "Show dialog");
    assert!(drawn(cx, "Slots hold zero or one child."));
    click(cx, "Dismiss");

    click(cx, "Counters");
    assert!(drawn(cx, "Counter 0"));
    click(cx, "Next");
    assert!(drawn(cx, "Counter 1"));
    click(cx, "Push 1000 screens");
    assert!(drawn(cx, "Stack depth 1002"));
    click(cx, "Back to first");

    click(cx, "Articles");
    click(cx, "article 3");
    click(cx, "Author");
    assert!(drawn(cx, "Author 3"));
    click(cx, "Close author");

    click(cx, "Menu");
    click(cx, "Open pages");
    assert!(drawn(cx, "Page 1"));
    click(cx, "Jump to 50");
    assert!(drawn(cx, "Page 50"));
    click(cx, "Close");
    assert!(at_tabs(&view, cx));

    click(cx, "Open custom navigation");
    assert!(drawn(
        cx,
        "A custom router: Filmstrip, written with only public API."
    ));
    click(cx, "Change mode");
    click(cx, "Close");
    assert!(at_tabs(&view, cx));
}

#[gpui::test]
fn the_feed_reports_its_real_window(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "Feed");
    cx.run_until_parked();
    let live = view.read_with(cx, |view, _cx| {
        let root = view.tree().get(view.root()).unwrap();
        let Child::Tabs(tabs) = *root.stack.active() else {
            return 0;
        };
        let showcase::feature::tabs::TabChild::Feed(feed) =
            *view.tree().get(tabs).unwrap().stack.active()
        else {
            return 0;
        };
        view.tree().get(feed).unwrap().items.live()
    });
    // One live row would be the height measurement reported as the window.
    assert!(live > 1 && live < 200, "{live} live rows");
}

#[gpui::test]
fn a_saved_session_resumes_on_the_same_screen(cx: &mut TestAppContext) {
    let saved = {
        let (view, cx) = open(cx);
        click(cx, "Counters");
        click(cx, "Next");
        assert!(drawn(cx, "Counter 1"));
        view.read_with(cx, |view, _cx| view.save()).unwrap()
    };
    let (_view, cx) = resume(cx, Some(saved));
    assert!(drawn(cx, "Counter 1"));
}

#[gpui::test]
fn the_system_back_shortcut_goes_back(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "Menu");
    click(cx, "Open pages");
    assert!(!at_tabs(&view, cx));
    let shortcut = if cfg!(target_os = "macos") {
        "cmd-["
    } else {
        "alt-left"
    };
    cx.simulate_keystrokes(shortcut);
    cx.run_until_parked();
    assert!(at_tabs(&view, cx));
}
