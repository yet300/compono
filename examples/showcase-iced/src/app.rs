use crate::ui;
use compono::{Handle, Phase, Spawner, Tree, WakeSignal};
use compono_iced::{Action, IcedHost, Presence, back_requests, presence, wakes};
use iced::{Element, Subscription, Task, window};
use showcase::Services;
use showcase::feature::root::Root;

/// Everything the iced update loop can receive.
#[derive(Clone, Debug)]
pub enum Message {
    Tree(Action),
    Presence(Presence),
    Phase(Phase),
    Back,
    /// The window asked to close. The application saves before closing it.
    CloseRequested(window::Id),
}

/// A component tree connected to an iced application.
#[derive(Debug)]
pub struct App {
    tree: Tree,
    root: Handle<Root>,
    signal: WakeSignal,
}

impl App {
    /// Builds the tree, resuming `saved` from [`App::save`] when it is given.
    pub fn new(services: Services, spawner: impl Spawner + 'static, saved: Option<&[u8]>) -> Self {
        let host = IcedHost::new();
        let signal = host.signal();
        let mut tree = showcase::saved::tree(host, saved);
        tree.set_spawner(spawner);
        let root = tree.mount_persistent(move |cx| Root::new(cx, services));
        Self { tree, root, signal }
    }

    /// Encodes the tree for the next launch.
    #[must_use]
    pub fn save(&self) -> Option<Vec<u8>> {
        showcase::saved::save(&self.tree)
    }

    /// Returns the tree, for tests.
    pub const fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }

    /// Returns the tree, for tests.
    #[must_use]
    pub const fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Returns the root component.
    #[must_use]
    pub const fn root(&self) -> Handle<Root> {
        self.root
    }

    /// Applies one message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tree(action) => action.apply(&mut self.tree),
            Message::Presence(presence) => {
                self.tree.set_phase(presence.phase);
                return presence.check_minimized().map(Message::Phase);
            }
            Message::Phase(phase) => self.tree.set_phase(phase),
            Message::Back => {
                let _handled = self.tree.back();
            }
            Message::CloseRequested(window) => return window::close(window),
        }
        Task::none()
    }

    /// Builds the current screen.
    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        ui::root(&self.tree, self.root).map(Message::Tree)
    }

    /// Listens for background work, focus, Back and closing.
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            wakes(&self.signal).map(Message::Tree),
            presence().map(Message::Presence),
            back_requests().map(|_request| Message::Back),
            window::close_requests().map(Message::CloseRequested),
        ])
    }
}
