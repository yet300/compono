use crate::history::{Browser, EntryState};
use js_sys::Array;
use wasm_bindgen::JsValue;

/// Where the address bar carries the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Address {
    /// In the path itself, as `/book`. The server must answer every such path
    /// with the application's page.
    Path,
    /// After `#`, as `#/book`. Works from any static file server.
    Hash,
}

const TAG: &str = "compono";

fn encode(state: EntryState) -> JsValue {
    Array::of4(
        &JsValue::from_str(TAG),
        &JsValue::from(state.schema),
        &JsValue::from(state.epoch),
        &JsValue::from(state.index),
    )
    .into()
}

fn number(value: &JsValue) -> Option<u32> {
    let number = value.as_f64()?;
    let whole = number.trunc();
    (whole == number && (0.0..=f64::from(u32::MAX)).contains(&whole))
        .then(|| u32::try_from(whole as u64).ok())
        .flatten()
}

fn decode(value: &JsValue) -> Option<EntryState> {
    if !Array::is_array(value) {
        return None;
    }
    let array = Array::from(value);
    if array.length() != 4 || array.get(0).as_string().as_deref() != Some(TAG) {
        return None;
    }
    Some(EntryState {
        schema: number(&array.get(1))?,
        epoch: number(&array.get(2))?,
        index: number(&array.get(3))?,
    })
}

/// The page's real history, for [`History`](crate::history::History).
///
/// Errors from the browser, such as a refused `pushState`, leave history as it
/// was. The next [`History::sync`](crate::history::History::sync) does not
/// retry them.
#[derive(Clone, Debug)]
pub struct BrowserHistory {
    history: web_sys::History,
    location: web_sys::Location,
    address: Address,
}

impl BrowserHistory {
    /// Binds to the page's history.
    ///
    /// # Errors
    ///
    /// Returns the browser's error when the page has no window or history.
    pub fn new(address: Address) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        Ok(Self {
            history: window.history()?,
            location: window.location(),
            address,
        })
    }

    /// Returns the current entry's state and path, for
    /// [`History::start`](crate::history::History::start).
    #[must_use]
    pub fn current(&self) -> (Option<EntryState>, String) {
        let state = self.history.state().ok().as_ref().and_then(decode);
        (state, current_path(&self.location, self.address))
    }

    fn url(&self, path: &str) -> String {
        match self.address {
            Address::Path => path.to_owned(),
            Address::Hash => format!("#{path}"),
        }
    }
}

pub(crate) fn current_path(location: &web_sys::Location, address: Address) -> String {
    let path = match address {
        Address::Path => location.pathname().unwrap_or_default(),
        Address::Hash => {
            let hash = location.hash().unwrap_or_default();
            hash.strip_prefix('#').unwrap_or(&hash).to_owned()
        }
    };
    if path.is_empty() {
        "/".to_owned()
    } else {
        path
    }
}

pub(crate) fn state_of(value: &JsValue) -> Option<EntryState> {
    decode(value)
}

impl Browser for BrowserHistory {
    fn push(&mut self, path: &str, state: EntryState) {
        let url = self.url(path);
        let _refused = self
            .history
            .push_state_with_url(&encode(state), "", Some(&url));
    }

    fn replace(&mut self, path: &str, state: EntryState) {
        let url = self.url(path);
        let _refused = self
            .history
            .replace_state_with_url(&encode(state), "", Some(&url));
    }

    fn go(&mut self, delta: i32) {
        let _refused = self.history.go_with_delta(delta);
    }

    fn new_epoch(&mut self) -> u32 {
        let random = js_sys::Math::random() * f64::from(u32::MAX);
        u32::try_from(random as u64).unwrap_or_default()
    }
}
