# iced

Views return `Element<'_, Action>`. An `Action` is a cloneable description of a
change to the tree, and the app applies it in `update`.

```rust,ignore
use compono::{Handle, Phase, Tree, WakeSignal};
use compono_iced::{Action, IcedHost, Presence, back_requests, presence, wakes};
use iced::{Element, Subscription, Task, window};

#[derive(Clone, Debug)]
pub enum Message {
    Tree(Action),
    Presence(Presence),
    Phase(Phase),
    Back,
}

pub struct App {
    tree: Tree,
    root: Handle<Root>,
    signal: WakeSignal,
}

impl App {
    pub fn new(saved: Option<&[u8]>) -> Self {
        let host = IcedHost::new();
        let signal = host.signal();
        let mut tree = match saved {
            Some(bytes) => Tree::with_saved(host, bytes, SCHEMA).0,
            None => Tree::new(host),
        };
        tree.set_spawner(spawner);
        let root = tree.mount_persistent(|cx| Root::new(cx));
        Self { tree, root, signal }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tree(action) => action.apply(&mut self.tree),
            Message::Presence(presence) => {
                self.tree.set_phase(presence.phase);
                // iced reports no minimise event, so ask after focus is lost.
                return presence.check_minimized().map(Message::Phase);
            }
            Message::Phase(phase) => self.tree.set_phase(phase),
            Message::Back => {
                let _handled = self.tree.back();
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        ui::root(&self.tree, self.root).map(Message::Tree)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            wakes(&self.signal).map(Message::Tree), // work from other threads
            presence().map(Message::Presence),
            back_requests().map(|_request| Message::Back),
        ])
    }
}
```

To save on close, set `.exit_on_close_request(false)` on the application, listen
to `window::close_requests()`, store `tree.save(SCHEMA)`, then return
`window::close(id)`.

## Views

```rust,ignore
use compono_iced::{RouterView, send};

pub type View<'a> = Element<'a, Action>;

pub fn view(tree: &Tree, handle: Handle<Counters>) -> View<'_> {
    let Some(counters) = tree.get(handle) else { return space().into() };
    column![
        text(format!("Stack depth {}", counters.stack.depth())),
        button("Back to first").on_press(send(handle, CountersMsg::PopToRoot)),
        counters.stack.view(|child| counter::view(tree, *child)),
    ]
    .into()
}
```

- `router.view(|child| ...)` returns `View` for `Stack` and `Panels`, and
  `Option<View>` for `Slot` and `Pages`.
- `ListView::new(&items, handle, Msg::Window).row_height(h).view(|index, child| ...)`.
- Router views animate with `compono_motion::Motion::desktop()`, a 150 ms fade;
  `compono_iced::set_motion(Motion::web())` changes it for the whole application. A `Stack` moves its
  new entry in, a `Slot` fades in, `Pages` slide by page order and `Panels` in
  `PanelsMode::Single` slide the deepest panel in. An entry that leaves disappears at
  once, because an `Element` cannot outlive the component it reads.
- `compono_iced::set_motion_preference(MotionPreference::Reduced)` passes the
  system's reduced-motion setting, which iced does not read.
- A widget callback that takes a value, such as `TabBar::new(on_select)` or
  `toggler(..).on_toggle(..)`, returns `send(handle, msg)` from its closure.
- `iced_aw` adds `TabBar`, `Card` and `Badge` for a standard look.
