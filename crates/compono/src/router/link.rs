//! Links: routes named by a path, such as `/articles/3`.
//!
//! [`Paths`] maps a [`Stack`]'s routes to paths and back. [`Navigation::toward`]
//! turns a path into the change that shows it, which an Android intent, an iOS
//! universal link or browser history then applies with [`Navigation::apply`] in
//! the component that owns the stack.
//!
//! ```
//! use compono::router::link::{Navigation, Paths};
//! use compono::router::stack::Stack;
//! use compono::{Component, Cx, Tree};
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
//! struct Site {
//!     stack: Stack<u32, (), Site>,
//! }
//!
//! impl Component for Site {
//!     type Msg = Navigation<u32>;
//!     type Output = ();
//!     fn update(&mut self, navigation: Navigation<u32>, cx: &mut Cx<'_, Self>) {
//!         navigation.apply(&mut self.stack, cx);
//!     }
//! }
//!
//! let mut tree = Tree::new(Quiet);
//! let site = tree.mount(|cx| Site { stack: Stack::new(0, cx, |_route, _cx| ()) });
//!
//! // A link to `/7` arrives, for example from an Android intent.
//! let change = tree
//!     .get(site)
//!     .and_then(|site| Navigation::toward(&Addresses, "/7", &site.stack));
//! assert_eq!(change, Some(Navigation { keep: 1, open: vec![7] }), "the list stays");
//! if let Some(change) = change {
//!     tree.send(site, change)?;
//! }
//! let routes: Vec<u32> = tree
//!     .get(site)
//!     .map(|site| site.stack.entries().map(|(_key, route)| *route).collect())
//!     .unwrap_or_default();
//! assert_eq!(routes, [0, 7]);
//! # Ok::<(), compono::StaleHandle>(())
//! ```

use crate::router::stack::Stack;
use crate::{Component, Cx};
use alloc::string::String;
use alloc::vec::Vec;

/// Maps routes to paths and back.
pub trait Paths {
    /// The stack's route type.
    type Route;

    /// Returns the path that names `route`, such as `/articles/3`.
    fn path(&self, route: &Self::Route) -> String;

    /// Returns the screens a link to `path` opens, oldest first.
    ///
    /// The last one is the route `path` names, and the ones before it are what
    /// Back from it should reveal. An empty list means the path is not one of
    /// the application's, and the application keeps what it shows.
    fn routes(&self, path: &str) -> Vec<Self::Route>;
}

/// A change to a stack: keep its oldest entries and open routes above them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navigation<R> {
    /// How many of the oldest entries to keep. Zero replaces every entry.
    pub keep: usize,
    /// Routes to open above the kept entries, oldest first.
    pub open: Vec<R>,
}

impl<R: 'static> Navigation<R> {
    /// Returns the change that makes `stack` show what `path` names.
    ///
    /// Entries the stack already shows at the bottom are kept with their state.
    /// Returns `None` when the stack already shows exactly those screens, or when
    /// `paths` does not know `path`.
    pub fn toward<P, H, C>(paths: &P, path: &str, stack: &Stack<R, H, C>) -> Option<Self>
    where
        P: Paths<Route = R> + ?Sized,
        H: 'static,
        C: Component,
    {
        let mut routes = paths.routes(path);
        if routes.is_empty() {
            return None;
        }
        let keep = routes
            .iter()
            .zip(stack.entries())
            .take_while(|(wanted, (_key, shown))| paths.path(wanted) == paths.path(shown))
            .count();
        if keep == routes.len() && keep == stack.depth() {
            return None;
        }
        let open = routes.split_off(keep);
        Some(Self { keep, open })
    }

    /// Applies the change to `stack`, from inside the component that owns it.
    pub fn apply<H: 'static, P: Component>(self, stack: &mut Stack<R, H, P>, cx: &mut Cx<'_, P>) {
        if self.keep == 0 {
            let _empty = stack.replace_all(self.open, cx);
            return;
        }
        stack.pop_to(self.keep, cx);
        for route in self.open {
            stack.push(route, cx);
        }
    }
}
