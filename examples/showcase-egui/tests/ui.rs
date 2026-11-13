//! The showcase through real egui frames, on virtual time.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono_test::VirtualTime;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use showcase::Services;
use showcase::feature::root::Child;
use showcase_egui::App;

struct State {
    app: Option<App>,
    time: VirtualTime,
}

fn harness<'a>() -> Harness<'a, State> {
    resumed(None)
}

fn resumed<'a>(saved: Option<Vec<u8>>) -> Harness<'a, State> {
    let time = VirtualTime::new();
    let state = State {
        app: None,
        time: time.clone(),
    };
    Harness::builder()
        .with_size(egui::vec2(1000.0, 720.0))
        .build_ui_state(
            move |ui, state: &mut State| {
                let time = time.clone();
                let app = state.app.get_or_insert_with(|| {
                    let clock = time.clone();
                    let services = Services::new(move |delay| Box::pin(clock.sleep(delay)));
                    App::new(ui.ctx(), services, time.clone(), saved.as_deref())
                });
                app.frame(ui);
            },
            state,
        )
}

fn settle(harness: &mut Harness<'_, State>) {
    harness.run_steps(3);
    let state = harness.state_mut();
    let time = state.time.clone();
    time.settle(state.app.as_mut().unwrap().tree_mut()).unwrap();
    harness.run_steps(3);
}

fn click(harness: &mut Harness<'_, State>, label: &str) {
    harness.get_by_label(label).click();
    settle(harness);
}

fn at_tabs(harness: &Harness<'_, State>) -> bool {
    let app = harness.state().app.as_ref().unwrap();
    let root = app.tree().get(app.root()).unwrap();
    matches!(root.stack.active(), Child::Tabs(_))
}

#[test]
fn every_navigation_model_can_be_used_from_the_screen() {
    let mut app = harness();
    settle(&mut app);
    app.get_by_label("Showcase");
    click(&mut app, "Show dialog");
    app.get_by_label("Slots hold zero or one child.");
    click(&mut app, "Dismiss");

    click(&mut app, "Counters");
    app.get_by_label("Counter 0");
    click(&mut app, "Next");
    app.get_by_label("Counter 1");
    click(&mut app, "Push 1000 screens");
    app.get_by_label("Stack depth 1002");
    click(&mut app, "Back to first");

    click(&mut app, "Cards");
    app.get_by_label("Card 4: 0 (Focused)");
    click(&mut app, "Send top to back");
    app.get_by_label("Card 3: 0 (Focused)");

    click(&mut app, "Articles");
    click(&mut app, "Article 3: router");
    click(&mut app, "Author");
    app.get_by_label("Author 3");
    click(&mut app, "Close author");

    click(&mut app, "Feed");
    app.get_by_label("Row 0  value 0");

    click(&mut app, "Menu");
    click(&mut app, "Open pages");
    app.get_by_label("Page 1");
    click(&mut app, "Jump to 50");
    app.get_by_label("Page 50");
    app.get_by_label("50 of 100 pages, 3 alive");
    click(&mut app, "Close");
    assert!(at_tabs(&app));

    click(&mut app, "Open custom navigation");
    app.get_by_label("A custom router: Filmstrip, written with only public API.");
    click(&mut app, "Change mode");
    click(&mut app, "Close");
    assert!(at_tabs(&app));
}

#[test]
fn a_saved_session_resumes_on_the_same_screen() {
    let mut app = harness();
    settle(&mut app);
    click(&mut app, "Counters");
    click(&mut app, "Next");
    app.get_by_label("Counter 1");
    let saved = app.state().app.as_ref().unwrap().save().unwrap();

    let mut again = resumed(Some(saved));
    settle(&mut again);
    again.get_by_label("Counter 1");
}
