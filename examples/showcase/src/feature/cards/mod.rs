//! A pile of cards: a `ChildSet`, which keeps components while keys move around.

mod card;
pub use card::{Card, CardMsg};

use crate::Services;
use compono::router::set::ChildSet;
use compono::{Component, Cx, Handle, Phase};
use std::time::Duration;

const TICK: Duration = Duration::from_millis(500);
const MAX_CARDS: usize = 12;

/// Input to [`Cards`].
#[derive(Clone, Copy, Debug)]
pub enum CardsMsg {
    Add,
    Remove,
    SendTopToBack,
}

/// The cards tab.
#[derive(Debug)]
pub struct Cards {
    /// The cards. The last one is on top.
    pub cards: ChildSet<u32, Handle<Card>, Self>,
    order: Vec<u32>,
    next: u32,
}

impl Cards {
    /// Creates a pile of four cards.
    pub fn new(cx: &mut Cx<'_, Self>, services: Services) -> Self {
        let cards = ChildSet::new(cx, move |number: &u32, cx| {
            let (number, services) = (*number, services.clone());
            cx.child(
                move |_cx| Card {
                    number,
                    value: 0,
                    timer: None,
                    services,
                },
                |_cards: &mut Self, (), _cx| {},
            )
        });
        let mut screen = Self {
            cards,
            order: (1..=4).collect(),
            next: 5,
        };
        screen.apply(cx);
        screen
    }

    /// Writes the desired order. This is all a list-shaped navigation needs.
    fn apply(&mut self, cx: &mut Cx<'_, Self>) {
        let top = self.order.len().saturating_sub(1);
        let desired = self.order.iter().enumerate().map(|(position, number)| {
            let phase = if position == top {
                Phase::Focused
            } else {
                Phase::Visible
            };
            (*number, phase)
        });
        // The numbers come from one counter, so they cannot repeat.
        let _unique = self.cards.reconcile(desired, cx);
    }
}

impl Component for Cards {
    type Msg = CardsMsg;
    type Output = ();

    fn update(&mut self, msg: CardsMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            CardsMsg::Add if self.order.len() < MAX_CARDS => {
                self.order.push(self.next);
                self.next += 1;
            }
            CardsMsg::Remove if self.order.len() > 1 => {
                self.order.pop();
            }
            CardsMsg::SendTopToBack => self.order.rotate_right(1),
            CardsMsg::Add | CardsMsg::Remove => {
                cx.unchanged();
                return;
            }
        }
        self.apply(cx);
    }
}
