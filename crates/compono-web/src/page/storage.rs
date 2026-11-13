use wasm_bindgen::JsValue;

/// Saved state in the tab's `sessionStorage`.
///
/// It survives a reload and Back into the page from another site, and is
/// dropped when the tab closes, which matches what a mobile system keeps.
#[derive(Clone, Debug)]
pub struct SessionStore {
    storage: web_sys::Storage,
    key: String,
}

impl SessionStore {
    /// Uses the item named `key`.
    ///
    /// # Errors
    ///
    /// Returns the browser's error when session storage is unavailable, as in
    /// some private modes.
    pub fn new(key: impl Into<String>) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let storage = window
            .session_storage()?
            .ok_or_else(|| JsValue::from_str("no session storage"))?;
        Ok(Self {
            storage,
            key: key.into(),
        })
    }

    /// Reads the saved bytes, or `None` if there are none or they are damaged.
    #[must_use]
    pub fn load(&self) -> Option<Vec<u8>> {
        let text = self.storage.get_item(&self.key).ok()??;
        text.chars()
            .map(|char| u8::try_from(u32::from(char)).ok())
            .collect()
    }

    /// Replaces the saved bytes.
    ///
    /// # Errors
    ///
    /// Returns the browser's error, such as a full quota.
    pub fn store(&self, bytes: &[u8]) -> Result<(), JsValue> {
        // Each byte becomes one character, which storage keeps exactly.
        let text: String = bytes.iter().copied().map(char::from).collect();
        self.storage.set_item(&self.key, &text)
    }

    /// Removes the saved bytes.
    ///
    /// # Errors
    ///
    /// Returns the browser's error.
    pub fn clear(&self) -> Result<(), JsValue> {
        self.storage.remove_item(&self.key)
    }
}
