//! The load test: 100 000 rows, with live components only on screen.

mod row;
pub use row::{Row, RowMsg};

use crate::Services;
use compono::router::items::Items;
use compono::{Component, Cx, Handle, Phase, TaskId};
use core::ops::Range;
use std::time::Duration;

/// How many rows the feed has.
pub const ROWS: u64 = 100_000;
/// Rows update at about this interval while on screen.
pub const FRAME: Duration = Duration::from_millis(16);

const INITIAL_ROWS: usize = 24;

/// Input to [`Feed`].
#[derive(Clone, Debug)]
pub enum FeedMsg {
    Frame,
    Window(Range<usize>),
    ToggleLive,
}

/// The feed tab.
#[derive(Debug)]
pub struct Feed {
    /// The rows.
    pub items: Items<u64, Handle<Row>, Self>,
    /// Whether rows update every frame.
    pub live: bool,
    /// How many frames have been delivered.
    pub frames: u64,
    timer: Option<TaskId>,
    services: Services,
}

impl Feed {
    /// Creates the feed.
    pub fn new(cx: &mut Cx<'_, Self>, services: Services) -> Self {
        let mut items = Items::new((0..ROWS).collect(), cx, |id: &u64, cx| {
            let id = *id;
            cx.child(
                move |_cx| Row {
                    id,
                    value: 0,
                    liked: false,
                },
                |_feed: &mut Self, (), _cx| {},
            )
        });
        // The UI reports the real window on its first frame.
        let _unique = items.set_window(0..INITIAL_ROWS, cx);
        Self {
            items,
            live: true,
            frames: 0,
            timer: None,
            services,
        }
    }

    fn arm(&mut self, cx: &mut Cx<'_, Self>) {
        let sleep = self.services.sleep(FRAME);
        self.timer = cx.perform(sleep, |()| FeedMsg::Frame).ok();
    }

    fn stop(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(task) = self.timer.take() {
            cx.cancel(task);
        }
    }
}

impl Component for Feed {
    type Msg = FeedMsg;
    type Output = ();

    fn update(&mut self, msg: FeedMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            FeedMsg::Frame => {
                self.frames += 1;
                let rows: Vec<Handle<Row>> = self
                    .items
                    .visible()
                    .map(|(_index, _key, row)| *row)
                    .collect();
                for row in rows {
                    // Rows in the window are live children of the feed.
                    let _queued = cx.send(row, RowMsg::Advance);
                }
                self.arm(cx);
                // Only the rows changed. The list around them did not.
                cx.unchanged();
            }
            FeedMsg::Window(window) => {
                if !matches!(self.items.set_window(window, cx), Ok(true)) {
                    cx.unchanged();
                }
            }
            FeedMsg::ToggleLive => {
                self.live = !self.live;
                self.stop(cx);
                if self.live && cx.phase() == Phase::Focused {
                    self.arm(cx);
                }
            }
        }
    }

    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        cx.unchanged();
        self.stop(cx);
        if phase == Phase::Focused && self.live {
            self.arm(cx);
        }
    }
}
