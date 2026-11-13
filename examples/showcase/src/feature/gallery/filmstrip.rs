//! `Filmstrip`, a custom router written with only the public API.

use compono::router::{Router, RouterId, Scope};
use compono::{Component, Cx, Phase};

/// How the strip presents its frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StripMode {
    /// Every frame is on screen.
    #[default]
    Contact,
    /// Only the selected frame is on screen.
    Single,
}

struct Frame<H> {
    key: u64,
    child: H,
    scope: Scope,
}

/// The custom router. Generic like the built-in ones, and held as a field.
pub struct Filmstrip<H, P> {
    frames: Vec<Frame<H>>,
    selected: usize,
    mode: StripMode,
    router: RouterId,
    marker: core::marker::PhantomData<fn() -> P>,
}

impl<H, P> std::fmt::Debug for Filmstrip<H, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filmstrip")
            .field("frames", &self.frames.len())
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl<H: 'static, P: Component> Filmstrip<H, P> {
    /// Creates one child per key.
    pub fn new(
        keys: impl IntoIterator<Item = u64>,
        cx: &mut Cx<'_, P>,
        build: impl Fn(u64, &mut Cx<'_, P>) -> H,
    ) -> Self {
        let router = cx.router();
        cx.forget_router(router);
        let frames = keys
            .into_iter()
            .map(|key| {
                let (child, scope) = cx.scoped(router, key, |cx| build(key, cx));
                Frame { key, child, scope }
            })
            .collect();
        let strip = Self {
            frames,
            selected: 0,
            mode: StripMode::default(),
            router,
            marker: core::marker::PhantomData,
        };
        strip.grant(cx);
        strip
    }

    /// Returns each frame's key, value and whether it is selected.
    pub fn frames(&self) -> impl Iterator<Item = (u64, &H, bool)> {
        self.frames
            .iter()
            .enumerate()
            .map(|(index, frame)| (frame.key, &frame.child, index == self.selected))
    }

    /// Returns the selected frame's value.
    #[must_use]
    pub fn selected(&self) -> Option<&H> {
        self.frames.get(self.selected).map(|frame| &frame.child)
    }

    /// Returns how frames are presented.
    #[must_use]
    pub const fn mode(&self) -> StripMode {
        self.mode
    }

    /// Moves the selection, wrapping at the ends.
    pub fn advance(&mut self, step: isize, cx: &mut Cx<'_, P>) {
        let count = self.frames.len().max(1).cast_signed();
        self.selected = (self.selected.cast_signed() + step)
            .rem_euclid(count)
            .cast_unsigned();
        self.grant(cx);
    }

    /// Changes the presentation.
    pub fn set_mode(&mut self, mode: StripMode, cx: &mut Cx<'_, P>) {
        self.mode = mode;
        self.grant(cx);
    }

    /// Removes the selected frame and its components.
    pub fn remove_selected(&mut self, cx: &mut Cx<'_, P>) {
        if self.selected < self.frames.len() {
            let frame = self.frames.remove(self.selected);
            cx.destroy_scope(&frame.scope);
            self.selected = self.selected.min(self.frames.len().saturating_sub(1));
            self.grant(cx);
        }
    }

    /// Returns the identity of this router in saved state.
    #[must_use]
    pub const fn router(&self) -> RouterId {
        self.router
    }

    fn grant(&self, cx: &mut Cx<'_, P>) {
        let unselected = match self.mode {
            StripMode::Contact => Phase::Visible,
            StripMode::Single => Phase::Hidden,
        };
        // Lower first, so focus leaves one frame before reaching another.
        for (index, frame) in self.frames.iter().enumerate() {
            if index != self.selected {
                cx.assign_scope(&frame.scope, unselected);
            }
        }
        if let Some(frame) = self.frames.get(self.selected) {
            cx.assign_scope(&frame.scope, Phase::Focused);
        }
    }
}

impl<H: 'static, P: Component> Router<P> for Filmstrip<H, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        [self.router]
    }

    fn destroy(self, cx: &mut Cx<'_, P>) {
        // Newest first, as a component's children are removed.
        for frame in self.frames.iter().rev() {
            cx.destroy_scope(&frame.scope);
        }
    }
}
