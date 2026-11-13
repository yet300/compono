//! The `Row` component.

use compono::{Component, Cx};

/// Input to [`Row`].
#[derive(Clone, Copy, Debug)]
pub enum RowMsg {
    Advance,
    Like,
}

/// One live row.
#[derive(Debug)]
pub struct Row {
    /// Its key.
    pub id: u64,
    /// Advances every frame while on screen.
    pub value: u64,
    /// Whether the user liked it.
    pub liked: bool,
}

impl Component for Row {
    type Msg = RowMsg;
    type Output = ();

    fn update(&mut self, msg: RowMsg, _cx: &mut Cx<'_, Self>) {
        match msg {
            RowMsg::Advance => self.value = self.value.wrapping_add(self.id % 7 + 1),
            RowMsg::Like => self.liked = !self.liked,
        }
    }
}
