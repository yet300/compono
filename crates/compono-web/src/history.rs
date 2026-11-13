//! Browser history that mirrors one [`Stack`].
//!
//! Each stack entry is one browser history entry, so the browser's Back and
//! Forward buttons move through the stack and a link opens the screens its
//! path names. Routes map to paths through
//! [`Paths`], from the core. The mapping is plain
//! logic over the [`Browser`] trait, so it runs and is tested without a browser.
//!
//! ```
//! use compono::router::link::Paths;
//! use compono::router::stack::Stack;
//! use compono::{Component, Cx, Tree};
//! use compono_web::history::{Browser, EntryState, History};
//! # use compono::{Commit, Host};
//! # struct Quiet;
//! # impl Host for Quiet {
//! #     fn commit(&mut self, _commit: &Commit<'_>) {}
//! # }
//!
//! /// `/` is the list and `/7` is article 7, opened above the list.
//! struct Addresses;
//!
//! impl Paths for Addresses {
//!     type Route = u32;
//!
//!     fn path(&self, route: &u32) -> String {
//!         if *route == 0 { "/".to_owned() } else { format!("/{route}") }
//!     }
//!
//!     fn routes(&self, path: &str) -> Vec<u32> {
//!         match path.trim_start_matches('/') {
//!             "" => vec![0],
//!             article => article.parse().map(|article| vec![0, article]).unwrap_or_default(),
//!         }
//!     }
//! }
//!
//! /// Stands in for the page's history. On `wasm32` this is `BrowserHistory`.
//! #[derive(Default)]
//! struct Recorded(Vec<String>);
//!
//! impl Browser for Recorded {
//!     fn push(&mut self, path: &str, _state: EntryState) {
//!         self.0.push(format!("push {path}"));
//!     }
//!     fn replace(&mut self, path: &str, _state: EntryState) {
//!         self.0.push(format!("replace {path}"));
//!     }
//!     fn go(&mut self, delta: i32) {
//!         self.0.push(format!("go {delta}"));
//!     }
//!     fn new_epoch(&mut self) -> u32 {
//!         1
//!     }
//! }
//!
//! struct Site {
//!     stack: Stack<u32, (), Site>,
//! }
//!
//! impl Component for Site {
//!     type Msg = u32;
//!     type Output = ();
//!     fn update(&mut self, article: u32, cx: &mut Cx<'_, Self>) {
//!         self.stack.push(article, cx);
//!     }
//! }
//!
//! let mut tree = Tree::new(Quiet);
//! let site = tree.mount(|cx| Site { stack: Stack::new(0, cx, |_route, _cx| ()) });
//! let mut browser = Recorded::default();
//! let mut history = History::new(Addresses, 1);
//!
//! // The page opened at `/` with no state of ours in its entry.
//! if let Some(site) = tree.get(site) {
//!     let change = history.start(&mut browser, &site.stack, None, "/");
//!     assert_eq!(change, None, "the stack already shows what the address names");
//!     history.sync(&mut browser, &site.stack);
//! }
//!
//! // After each change to the tree, the browser is brought in line.
//! tree.send(site, 7)?;
//! if let Some(site) = tree.get(site) {
//!     history.sync(&mut browser, &site.stack);
//! }
//! assert_eq!(browser.0, ["replace /", "push /7"]);
//! # Ok::<(), compono::StaleHandle>(())
//! ```

use compono::Component;
use compono::router::link::{Navigation, Paths};
use compono::router::stack::Stack;

/// What the application stores in each history entry it creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryState {
    /// The application's saved-state schema when the entry was made.
    pub schema: u32,
    /// Names one run of entries, so entries from an earlier page load or schema
    /// are told apart from the current ones.
    pub epoch: u32,
    /// The entry's position in the stack, from zero at the root.
    pub index: u32,
}

/// The browser's history, as [`History`] uses it.
pub trait Browser {
    /// Adds an entry after the current one, dropping any forward entries.
    fn push(&mut self, path: &str, state: EntryState);
    /// Changes the current entry.
    fn replace(&mut self, path: &str, state: EntryState);
    /// Moves `delta` entries. The browser reports arrival later, through
    /// [`History::popped`].
    fn go(&mut self, delta: i32);
    /// Returns a number unlikely to name any epoch already in the history.
    fn new_epoch(&mut self) -> u32;
}

#[derive(Debug)]
struct Entry {
    /// The stack entry's key, or `None` until a matching entry is seen.
    key: Option<u64>,
    path: String,
}

/// Keeps browser history and one [`Stack`] in step.
///
/// Call [`History::start`] once, [`History::sync`] after each change to the
/// tree, and [`History::popped`] when the browser reports a `popstate`.
#[derive(Debug)]
pub struct History<P> {
    paths: P,
    schema: u32,
    epoch: u32,
    entries: Vec<Entry>,
    current: usize,
    traversal: Option<usize>,
}

fn to_index(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

impl<P: Paths> History<P>
where
    P::Route: 'static,
{
    /// Creates history for routes mapped by `paths`, tagging entries with `schema`.
    pub const fn new(paths: P, schema: u32) -> Self {
        Self {
            paths,
            schema,
            epoch: 0,
            entries: Vec::new(),
            current: 0,
            traversal: None,
        }
    }

    /// Returns the path mapping.
    pub const fn paths(&self) -> &P {
        &self.paths
    }

    /// Returns whether `state` belongs to the current run of entries.
    fn ours(&self, state: Option<EntryState>) -> Option<usize> {
        let state = state?;
        let index = usize::try_from(state.index).ok()?;
        (state.schema == self.schema && state.epoch == self.epoch && index <= self.entries.len())
            .then_some(index)
    }

    /// Takes over the current browser entry, at `path` with `state`.
    ///
    /// After a reload the entry still holds the state written before it, and a
    /// stack restored to the same depth is adopted as it is. Otherwise `path`
    /// decides: a `Navigation` is returned when it names other screens than the
    /// stack shows. Either way the next [`History::sync`] writes the stack.
    pub fn start<H: 'static, C: Component>(
        &mut self,
        browser: &mut impl Browser,
        stack: &Stack<P::Route, H, C>,
        state: Option<EntryState>,
        path: &str,
    ) -> Option<Navigation<P::Route>> {
        self.traversal = None;
        if let Some(state) = state
            && state.schema == self.schema
            && usize::try_from(state.index)
                .is_ok_and(|index| index.checked_add(1) == Some(stack.depth()))
            && self.paths.path(stack.route()) == path
        {
            self.epoch = state.epoch;
            self.entries = stack
                .entries()
                .map(|(key, route)| Entry {
                    key: Some(key),
                    path: self.paths.path(route),
                })
                .collect();
            self.current = stack.depth() - 1;
            return None;
        }
        self.rebase(browser);
        Navigation::toward(&self.paths, path, stack)
    }

    /// Starts a new run of entries at the browser's current entry.
    fn rebase(&mut self, browser: &mut impl Browser) {
        self.epoch = browser.new_epoch();
        self.entries.clear();
        self.current = 0;
    }

    /// Answers a `popstate`: the browser now shows `path`, holding `state`.
    ///
    /// Returns the change the application should make to its stack, and then
    /// call [`History::sync`] as after any other change.
    pub fn popped<H: 'static, C: Component>(
        &mut self,
        browser: &mut impl Browser,
        stack: &Stack<P::Route, H, C>,
        state: Option<EntryState>,
        path: &str,
    ) -> Option<Navigation<P::Route>> {
        if self.traversal.take().is_some() {
            if let Some(index) = self.ours(state) {
                self.current = index.min(self.entries.len().saturating_sub(1));
            }
            self.sync(browser, stack);
            return None;
        }
        match self.ours(state) {
            Some(index) if index <= self.current => {
                let moved = index < self.current;
                self.current = index;
                moved.then_some(Navigation {
                    keep: index + 1,
                    open: Vec::new(),
                })
            }
            Some(index) => {
                if let Some(navigation) = self.forward(index, path) {
                    return Some(navigation);
                }
                self.rebase(browser);
                Navigation::toward(&self.paths, path, stack)
            }
            None => {
                self.rebase(browser);
                Navigation::toward(&self.paths, path, stack)
            }
        }
    }

    /// Moves forward to `index`, using the paths of the entries passed over.
    fn forward(&mut self, index: usize, path: &str) -> Option<Navigation<P::Route>> {
        let mut open = Vec::new();
        for position in self.current + 1..=index {
            let passed = match self.entries.get(position) {
                Some(entry) => entry.path.as_str(),
                None if position == index => path,
                None => return None,
            };
            open.push(self.paths.routes(passed).pop()?);
        }
        for position in self.current + 1..=index {
            match self.entries.get_mut(position) {
                Some(entry) => entry.key = None,
                None => self.entries.push(Entry {
                    key: None,
                    path: path.to_owned(),
                }),
            }
        }
        let keep = self.current + 1;
        self.current = index;
        Some(Navigation { keep, open })
    }

    /// Brings the browser in line with `stack`. Cheap when nothing changed.
    ///
    /// While the browser is still moving back from an earlier call, this does
    /// nothing, and [`History::popped`] finishes the work on arrival.
    pub fn sync<H: 'static, C: Component>(
        &mut self,
        browser: &mut impl Browser,
        stack: &Stack<P::Route, H, C>,
    ) {
        if self.traversal.is_some() {
            return;
        }
        let depth = stack.depth();
        let mut kept = 0;
        for (key, route) in stack.entries().take(self.current + 1) {
            let Some(entry) = self.entries.get_mut(kept) else {
                break;
            };
            let same = match entry.key {
                Some(known) => known == key,
                None if entry.path == self.paths.path(route) => {
                    entry.key = Some(key);
                    true
                }
                None => false,
            };
            if !same {
                break;
            }
            kept += 1;
        }
        if kept > self.current {
            if depth > kept {
                self.write(browser, stack, kept);
            }
            return;
        }
        let target = if depth > kept { kept } else { kept - 1 };
        if target == self.current {
            self.write(browser, stack, target);
        } else {
            let delta = i32::try_from(self.current - target).unwrap_or(i32::MAX);
            self.traversal = Some(target);
            browser.go(-delta);
        }
    }

    /// Writes stack entries from `from` on, replacing the current entry when
    /// `from` is the current one and pushing the rest.
    fn write<H: 'static, C: Component>(
        &mut self,
        browser: &mut impl Browser,
        stack: &Stack<P::Route, H, C>,
        from: usize,
    ) {
        for (index, (key, route)) in stack.entries().enumerate().skip(from) {
            let path = self.paths.path(route);
            let state = EntryState {
                schema: self.schema,
                epoch: self.epoch,
                index: to_index(index),
            };
            let entry = Entry {
                key: Some(key),
                path,
            };
            if index == self.current && from == self.current {
                browser.replace(&entry.path, state);
                // Replacing an entry leaves the browser's forward entries alone.
                match self.entries.get_mut(index) {
                    Some(slot) => *slot = entry,
                    None => self.entries.push(entry),
                }
            } else {
                browser.push(&entry.path, state);
                self.entries.truncate(index);
                self.entries.push(entry);
            }
        }
        self.current = stack.depth() - 1;
    }
}
