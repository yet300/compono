//! The showcase through real iced views, on virtual time.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::Phase;
use compono_test::VirtualTime;
use iced::Size;
use iced_test::simulator::Simulator;
use showcase::Services;
use showcase::feature::root::Child;
use showcase_iced::{App, Message};

struct Session {
    app: App,
    time: VirtualTime,
}

fn session() -> Session {
    resumed(None)
}

fn resumed(saved: Option<&[u8]>) -> Session {
    let time = VirtualTime::new();
    let clock = time.clone();
    let services = Services::new(move |delay| Box::pin(clock.sleep(delay)));
    let mut app = App::new(services, time.clone(), saved);
    drop(app.update(Message::Phase(Phase::Focused)));
    time.settle(app.tree_mut()).unwrap();
    Session { app, time }
}

impl Session {
    fn ui(&self) -> Simulator<'_, Message> {
        Simulator::with_size(
            Default::default(),
            Size::new(1000.0, 720.0),
            self.app.view(),
        )
    }

    fn click(&mut self, label: &str) {
        let mut ui = self.ui();
        ui.click(label).unwrap();
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(!messages.is_empty(), "{label} produced no message");
        for message in messages {
            drop(self.app.update(message));
        }
        self.time.settle(self.app.tree_mut()).unwrap();
    }

    fn shows(&self, label: &str) -> bool {
        self.ui().find(label).is_ok()
    }

    fn at_tabs(&self) -> bool {
        let root = self.app.tree().get(self.app.root()).unwrap();
        matches!(root.stack.active(), Child::Tabs(_))
    }
}

#[test]
fn every_navigation_model_can_be_used_from_the_screen() {
    let mut app = session();
    assert!(app.shows("Showcase"));
    app.click("Show dialog");
    assert!(app.shows("Slots hold zero or one child."));
    app.click("Dismiss");

    app.click("Counters");
    assert!(app.shows("Counter 0"));
    app.click("Next");
    assert!(app.shows("Counter 1"));
    app.click("Push 1000 screens");
    assert!(app.shows("Stack depth 1002"));
    app.click("Back to first");

    app.click("Cards");
    assert!(app.shows("Card 4: 0 (Focused)"));
    app.click("Send top to back");
    assert!(app.shows("Card 3: 0 (Focused)"));

    app.click("Articles");
    app.click("Article 3: router");
    assert!(app.shows("Author"));
    app.click("Author");
    assert!(app.shows("Author 3"));
    app.click("Close author");

    app.click("Feed");
    assert!(app.shows("Row 0  value 0"));

    app.click("Menu");
    app.click("Open pages");
    assert!(app.shows("Page 1"));
    app.click("Jump to 50");
    assert!(app.shows("Page 50"));
    assert!(app.shows("50 of 100 pages, 3 alive"));
    app.click("Close");
    assert!(app.at_tabs());

    app.click("Open custom navigation");
    assert!(app.shows("A custom router: Filmstrip, written with only public API."));
    app.click("Change mode");
    app.click("Close");
    assert!(app.at_tabs());
}

#[test]
fn a_saved_session_resumes_on_the_same_screen() {
    let mut app = session();
    app.click("Counters");
    app.click("Next");
    assert!(app.shows("Counter 1"));
    let saved = app.app.save().unwrap();

    let again = resumed(Some(&saved));
    assert!(again.shows("Counter 1"));
}
