//! Up to three panels shown side by side or one at a time.

use super::slot::Slot;
use super::{Router, RouterId};
use crate::{Component, Cx, Phase};
use core::fmt;

/// How many panels fit on screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PanelsMode {
    /// Only the deepest open panel.
    #[default]
    Single,
    /// The two deepest open panels.
    Dual,
    /// All open panels.
    Triple,
}

impl PanelsMode {
    const fn capacity(self) -> u8 {
        match self {
            Self::Single => 1,
            Self::Dual => 2,
            Self::Triple => 3,
        }
    }
}

/// Which panels the UI should draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownPanels {
    /// Whether the main panel is on screen.
    pub main: bool,
    /// Whether the details panel is on screen.
    pub details: bool,
    /// Whether the extra panel is on screen.
    pub extra: bool,
}

/// The routes and values of three panels.
pub trait PanelTypes: 'static {
    /// The main panel's route.
    type Main: 'static;
    /// The main panel's value.
    type MainChild: 'static;
    /// The details panel's route.
    type Details: 'static;
    /// The details panel's value.
    type DetailsChild: 'static;
    /// The extra panel's route.
    type Extra: 'static;
    /// The extra panel's value.
    type ExtraChild: 'static;
}

/// One panel on screen, as given by [`Panels::visible`].
pub enum Panel<'a, T: PanelTypes> {
    /// The main panel's value.
    Main(&'a T::MainChild),
    /// The details panel's value.
    Details(&'a T::DetailsChild),
    /// The extra panel's value.
    Extra(&'a T::ExtraChild),
}

impl<T: PanelTypes> fmt::Debug for Panel<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Main(_) => "Main",
            Self::Details(_) => "Details",
            Self::Extra(_) => "Extra",
        })
    }
}

/// Main, details and extra panels held as a field of their parent `P`.
///
/// The main panel always exists. The deepest open panel is `Focused`, other
/// panels that fit the mode are `Visible`, and the rest are `Hidden` but alive.
///
/// ```
/// use compono::router::panels::{Panel, PanelTypes, Panels, PanelsMode};
/// use compono::{Component, Cx, Handle, Phase, Tree};
/// # use compono::{Commit, Host};
/// # struct Quiet;
/// # impl Host for Quiet {
/// #     fn commit(&mut self, _commit: &Commit<'_>) {}
/// # }
/// # struct Screen;
/// # impl Component for Screen {
/// #     type Msg = ();
/// #     type Output = ();
/// #     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
/// # }
/// # fn screen<P: Component>(cx: &mut Cx<'_, P>) -> Handle<Screen> {
/// #     cx.child(|_cx| Screen, |_parent, (), _cx| {})
/// # }
///
/// /// The route and the value of each panel.
/// struct Mail;
///
/// impl PanelTypes for Mail {
///     type Main = ();
///     type MainChild = Handle<Screen>;
///     type Details = u32;
///     type DetailsChild = Handle<Screen>;
///     type Extra = u32;
///     type ExtraChild = Handle<Screen>;
/// }
///
/// struct Inbox {
///     panels: Panels<Mail, Inbox>,
/// }
///
/// enum Msg {
///     Open(u32),
///     Resized(PanelsMode),
///     Back,
/// }
///
/// impl Component for Inbox {
///     type Msg = Msg;
///     type Output = ();
///
///     fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
///         match msg {
///             Msg::Open(letter) => self.panels.set_details(Some(letter), cx),
///             Msg::Resized(mode) => self.panels.set_mode(mode, cx),
///             Msg::Back => {
///                 self.panels.pop(cx);
///             }
///         }
///     }
///
///     fn back(&self) -> Option<Msg> {
///         self.panels.can_pop().then_some(Msg::Back)
///     }
/// }
///
/// let mut tree = Tree::new(Quiet);
/// tree.set_phase(Phase::Focused);
/// let inbox = tree.mount(|cx| Inbox {
///     panels: Panels::new(
///         (),
///         PanelsMode::Single,
///         cx,
///         |(), cx| screen(cx),
///         |_letter, cx| screen(cx),
///         |_author, cx| screen(cx),
///     ),
/// });
///
/// // On a narrow screen the letter covers the list.
/// tree.send(inbox, Msg::Open(3))?;
/// let drawn = |tree: &Tree| {
///     let panels = &tree.get(inbox)?.panels;
///     Some(panels.visible().map(|panel| matches!(panel, Panel::Main(_))).collect::<Vec<_>>())
/// };
/// assert_eq!(drawn(&tree), Some(vec![false]));
///
/// // On a wide one both are drawn, and the letter keeps the focus.
/// tree.send(inbox, Msg::Resized(PanelsMode::Dual))?;
/// assert_eq!(drawn(&tree), Some(vec![true, false]));
/// let main = tree.get(inbox).and_then(|inbox| inbox.panels.main().copied());
/// assert_eq!(main.and_then(|main| tree.phase(main)), Some(Phase::Visible));
/// # Ok::<(), compono::StaleHandle>(())
/// ```
pub struct Panels<T: PanelTypes, P> {
    main: Slot<T::Main, T::MainChild, P>,
    details: Slot<T::Details, T::DetailsChild, P>,
    extra: Slot<T::Extra, T::ExtraChild, P>,
    mode: PanelsMode,
}

impl<T: PanelTypes, P> fmt::Debug for Panels<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Panels")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl<T: PanelTypes, P: Component> Panels<T, P> {
    /// Creates panels with only the main panel open.
    pub fn new(
        main: T::Main,
        mode: PanelsMode,
        cx: &mut Cx<'_, P>,
        build_main: impl Fn(&T::Main, &mut Cx<'_, P>) -> T::MainChild + 'static,
        build_details: impl Fn(&T::Details, &mut Cx<'_, P>) -> T::DetailsChild + 'static,
        build_extra: impl Fn(&T::Extra, &mut Cx<'_, P>) -> T::ExtraChild + 'static,
    ) -> Self {
        let mut panels = Self {
            main: Slot::new(cx, build_main),
            details: Slot::new(cx, build_details),
            extra: Slot::new(cx, build_extra),
            mode,
        };
        panels.main.show(main, cx);
        panels.apply_phases(cx);
        panels
    }

    /// Returns the panels to draw, from main to extra.
    pub fn visible(&self) -> impl Iterator<Item = Panel<'_, T>> {
        let shown = self.shown();
        [
            self.main().filter(|_| shown.main).map(Panel::Main),
            self.details().filter(|_| shown.details).map(Panel::Details),
            self.extra().filter(|_| shown.extra).map(Panel::Extra),
        ]
        .into_iter()
        .flatten()
    }

    /// Returns the value of the main panel.
    pub fn main(&self) -> Option<&T::MainChild> {
        self.main.active()
    }

    /// Returns the value of the details panel.
    pub fn details(&self) -> Option<&T::DetailsChild> {
        self.details.active()
    }

    /// Returns the value of the extra panel.
    pub fn extra(&self) -> Option<&T::ExtraChild> {
        self.extra.active()
    }

    /// Returns the route of the main panel.
    pub fn main_route(&self) -> Option<&T::Main> {
        self.main.route()
    }

    /// Returns the route of the details panel.
    pub fn details_route(&self) -> Option<&T::Details> {
        self.details.route()
    }

    /// Returns the route of the extra panel.
    pub fn extra_route(&self) -> Option<&T::Extra> {
        self.extra.route()
    }

    /// Returns the current mode.
    #[must_use]
    pub const fn mode(&self) -> PanelsMode {
        self.mode
    }

    /// Returns which panels fit on screen.
    #[must_use]
    pub const fn shown(&self) -> ShownPanels {
        let open = 1 + self.details.is_open() as u8 + self.extra.is_open() as u8;
        let hidden = open.saturating_sub(self.mode.capacity());
        ShownPanels {
            main: hidden == 0,
            details: self.details.is_open() && hidden <= 1,
            extra: self.extra.is_open(),
        }
    }

    /// Replaces the main panel. The other panels stay as they are.
    pub fn set_main(&mut self, route: T::Main, cx: &mut Cx<'_, P>) {
        // It starts hidden, so that it is raised in order with the others.
        self.main.set_phase(Phase::Hidden, cx);
        self.main.show(route, cx);
        self.apply_phases(cx);
    }

    /// Opens, replaces or closes the details panel. Closing also closes extra.
    pub fn set_details(&mut self, route: Option<T::Details>, cx: &mut Cx<'_, P>) {
        match route {
            Some(route) => {
                // It starts hidden, so focus leaves the old panel first.
                self.details.set_phase(Phase::Hidden, cx);
                self.details.show(route, cx);
            }
            None => {
                self.extra.close(cx);
                self.details.close(cx);
            }
        }
        self.apply_phases(cx);
    }

    /// Opens, replaces or closes the extra panel.
    pub fn set_extra(&mut self, route: Option<T::Extra>, cx: &mut Cx<'_, P>) {
        match route {
            Some(route) => {
                self.extra.set_phase(Phase::Hidden, cx);
                self.extra.show(route, cx);
            }
            None => {
                self.extra.close(cx);
            }
        }
        self.apply_phases(cx);
    }

    /// Changes how many panels fit on screen.
    pub fn set_mode(&mut self, mode: PanelsMode, cx: &mut Cx<'_, P>) {
        self.mode = mode;
        self.apply_phases(cx);
    }

    /// Closes the deepest open panel. Returns whether a panel was closed.
    pub fn pop(&mut self, cx: &mut Cx<'_, P>) -> bool {
        if self.extra.is_open() {
            self.set_extra(None, cx);
            true
        } else if self.details.is_open() {
            self.set_details(None, cx);
            true
        } else {
            false
        }
    }

    /// Returns whether [`Panels::pop`] would close a panel.
    #[must_use]
    pub const fn can_pop(&self) -> bool {
        self.details.is_open() || self.extra.is_open()
    }

    fn apply_phases(&mut self, cx: &mut Cx<'_, P>) {
        let shown = self.shown();
        let grant = |is_shown: bool, is_deepest: bool| match (is_shown, is_deepest) {
            (true, true) => Phase::Focused,
            (true, false) => Phase::Visible,
            (false, _) => Phase::Hidden,
        };
        let extra_open = self.extra.is_open();
        let details_open = self.details.is_open();
        // Lower before raising, so focus leaves one panel before reaching another.
        let mut main = Some(grant(shown.main, !details_open && !extra_open));
        let mut details = Some(grant(shown.details, details_open && !extra_open));
        let mut extra = Some(grant(shown.extra, extra_open));
        for raising in [false, true] {
            if let Some(phase) = main.filter(|phase| (self.main.phase() < *phase) == raising) {
                self.main.set_phase(phase, cx);
                main = None;
            }
            if let Some(phase) = details.filter(|phase| (self.details.phase() < *phase) == raising)
            {
                self.details.set_phase(phase, cx);
                details = None;
            }
            if let Some(phase) = extra.filter(|phase| (self.extra.phase() < *phase) == raising) {
                self.extra.set_phase(phase, cx);
                extra = None;
            }
        }
    }
}

impl<T: PanelTypes, P: Component> Router<P> for Panels<T, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        let details = self.details.router_ids();
        let extra = self.extra.router_ids();
        self.main
            .router_ids()
            .into_iter()
            .chain(details)
            .chain(extra)
    }

    fn destroy(self, cx: &mut Cx<'_, P>) {
        self.extra.destroy(cx);
        self.details.destroy(cx);
        self.main.destroy(cx);
    }
}

#[cfg(feature = "serde")]
mod saved {
    use super::{PanelTypes, Panels, PanelsMode};
    use crate::router::slot::{SavedSlot, Slot};
    use crate::{Component, Cx, Phase};

    /// The persistable form of [`Panels`]: the route of each open panel.
    ///
    /// The mode is not saved, because it follows the size of the screen.
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct SavedPanels<M, D, E> {
        main: SavedSlot<M>,
        details: SavedSlot<D>,
        extra: SavedSlot<E>,
    }

    impl<T: PanelTypes, P: Component> Panels<T, P>
    where
        T::Main: Clone,
        T::Details: Clone,
        T::Extra: Clone,
    {
        /// Captures the open panels. Component state is saved by the tree.
        pub fn snapshot(&self) -> SavedPanels<T::Main, T::Details, T::Extra> {
            SavedPanels {
                main: self.main.snapshot(),
                details: self.details.snapshot(),
                extra: self.extra.snapshot(),
            }
        }

        /// Rebuilds panels, or creates them with only `main` open if nothing
        /// was saved.
        pub fn restore_or(
            saved: Option<SavedPanels<T::Main, T::Details, T::Extra>>,
            main: T::Main,
            mode: PanelsMode,
            cx: &mut Cx<'_, P>,
            build_main: impl Fn(&T::Main, &mut Cx<'_, P>) -> T::MainChild + 'static,
            build_details: impl Fn(&T::Details, &mut Cx<'_, P>) -> T::DetailsChild + 'static,
            build_extra: impl Fn(&T::Extra, &mut Cx<'_, P>) -> T::ExtraChild + 'static,
        ) -> Self {
            let Some(saved) = saved else {
                return Self::new(main, mode, cx, build_main, build_details, build_extra);
            };
            // Every panel starts hidden and is then raised once, in order.
            let hidden = Phase::Hidden;
            let mut panels = Self {
                main: Slot::restore_at(Some(saved.main), hidden, cx, build_main),
                details: Slot::restore_at(Some(saved.details), hidden, cx, build_details),
                extra: Slot::restore_at(Some(saved.extra), hidden, cx, build_extra),
                mode,
            };
            if !panels.main.is_open() {
                panels.main.show(main, cx);
            }
            panels.apply_phases(cx);
            panels
        }
    }
}

#[cfg(feature = "serde")]
pub use saved::SavedPanels;
