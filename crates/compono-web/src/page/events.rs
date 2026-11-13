use super::history::{Address, current_path, state_of};
use crate::history::EntryState;
use compono::Phase;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{Event, EventTarget, PopStateEvent, VisibilityState};

/// Something the page reports to the application.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageEvent {
    /// The page's presence changed. Save state when it becomes `Hidden`,
    /// because a hidden page may be discarded without further notice.
    Phase(Phase),
    /// The browser moved through history, for
    /// [`History::popped`](crate::history::History::popped).
    Popped {
        /// The state of the entry now shown.
        state: Option<EntryState>,
        /// The path of the entry now shown.
        path: String,
    },
}

/// Returns how present the page is.
///
/// | Page | Phase |
/// |---|---|
/// | hidden tab, minimized window | `Hidden` |
/// | visible without focus | `Visible` |
/// | visible with focus | `Focused` |
#[must_use]
pub fn page_phase() -> Phase {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return Phase::Hidden;
    };
    if document.visibility_state() == VisibilityState::Hidden {
        Phase::Hidden
    } else if document.has_focus().unwrap_or(false) {
        Phase::Focused
    } else {
        Phase::Visible
    }
}

struct Listener {
    target: EventTarget,
    name: &'static str,
    closure: Closure<dyn FnMut(Event)>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _absent = self
            .target
            .remove_event_listener_with_callback(self.name, self.closure.as_ref().unchecked_ref());
    }
}

/// Page lifecycle and history events, delivered while this value lives.
pub struct PageEvents {
    _listeners: Vec<Listener>,
}

impl std::fmt::Debug for PageEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageEvents").finish_non_exhaustive()
    }
}

type Handler = Rc<RefCell<dyn FnMut(PageEvent)>>;

fn deliver(handler: &Handler, event: PageEvent) {
    // An event raised from inside the handler is dropped rather than reentering it.
    if let Ok(mut handler) = handler.try_borrow_mut() {
        handler(event);
    }
}

impl PageEvents {
    /// Starts calling `handler` for focus, visibility and `popstate`.
    ///
    /// # Errors
    ///
    /// Returns the browser's error when the page has no window or document.
    pub fn listen(
        address: Address,
        handler: impl FnMut(PageEvent) + 'static,
    ) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;
        let handler: Handler = Rc::new(RefCell::new(handler));
        let mut listeners = Vec::new();
        let mut add = |target: &EventTarget,
                       name: &'static str,
                       on: Box<dyn FnMut(Event)>|
         -> Result<(), JsValue> {
            let closure = Closure::wrap(on);
            target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())?;
            listeners.push(Listener {
                target: target.clone(),
                name,
                closure,
            });
            Ok(())
        };
        for (target, name) in [
            (window.as_ref(), "focus"),
            (window.as_ref(), "blur"),
            (window.as_ref(), "pageshow"),
            (document.as_ref(), "visibilitychange"),
        ] {
            let handler = Rc::clone(&handler);
            add(
                target,
                name,
                Box::new(move |_event| deliver(&handler, PageEvent::Phase(page_phase()))),
            )?;
        }
        let hidden = Rc::clone(&handler);
        add(
            window.as_ref(),
            "pagehide",
            Box::new(move |_event| deliver(&hidden, PageEvent::Phase(Phase::Hidden))),
        )?;
        let location = window.location();
        add(
            window.as_ref(),
            "popstate",
            Box::new(move |event| {
                let state = event
                    .dyn_ref::<PopStateEvent>()
                    .and_then(|event| state_of(&event.state()));
                let path = current_path(&location, address);
                deliver(&handler, PageEvent::Popped { state, path });
            }),
        )?;
        Ok(Self {
            _listeners: listeners,
        })
    }

    /// Keeps the listeners for the rest of the page's life.
    pub fn forever(self) {
        std::mem::forget(self);
    }
}
