use core::fmt;
use core::marker::PhantomData;
use core::num::NonZeroU32;

/// Untyped address of a live or former node.
///
/// `Option<NodeId>` is as small as `NodeId`, because the generation is kept
/// plus one and zero is never a stored value.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub(crate) index: u32,
    /// The slot's generation plus one. A slot retires before its generation
    /// reaches `u32::MAX`, so every generation issued fits.
    stored_generation: NonZeroU32,
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeId")
            .field("index", &self.index)
            .field("generation", &self.generation())
            .finish()
    }
}

impl NodeId {
    /// An id no component ever has: its index is past any slot a tree can hold.
    pub(crate) const NONE: Self = Self {
        index: u32::MAX,
        stored_generation: NonZeroU32::MAX,
    };

    /// The id of slot `index` at `generation`.
    ///
    /// `u32::MAX` is never issued as a generation and gives [`NodeId::NONE`].
    pub(crate) const fn new(index: u32, generation: u32) -> Self {
        match NonZeroU32::new(generation.wrapping_add(1)) {
            Some(stored_generation) => Self {
                index,
                stored_generation,
            },
            None => Self::NONE,
        }
    }

    /// The generation of the slot this id was issued for.
    pub(crate) const fn generation(self) -> u32 {
        // The stored value is at least one.
        self.stored_generation.get().wrapping_sub(1)
    }

    /// Packs the id into one integer that can cross a language boundary directly.
    ///
    /// It never returns `u64::MAX`. A slot retires before its generation reaches
    /// `u32::MAX`, so no id has that generation, and `u64::MAX` (`-1` as an `i64`)
    /// can stand for no component.
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        ((self.generation() as u64) << 32) | self.index as u64
    }

    /// Rebuilds an id from [`NodeId::to_bits`].
    ///
    /// Bits whose generation is `u32::MAX`, such as `u64::MAX`, were never
    /// returned by `to_bits`, and give an id that matches no component.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(bits as u32, (bits >> 32) as u32)
    }
}

// The architecture promises 8-byte addresses, which cross language boundaries as
// one `u64`. Growing either type must be a deliberate change.
const _: () = assert!(core::mem::size_of::<NodeId>() == 8);
const _: () = assert!(core::mem::size_of::<Handle<()>>() == 8);
// A missing address costs nothing more, as in each link of a tree slot.
const _: () = assert!(core::mem::size_of::<Option<NodeId>>() == 8);

/// Typed address of a component. It stays safe to hold after the component is gone.
pub struct Handle<C> {
    id: NodeId,
    marker: PhantomData<fn() -> C>,
}

impl<C> Handle<C> {
    pub(crate) const fn new(id: NodeId) -> Self {
        Self {
            id,
            marker: PhantomData,
        }
    }

    /// Returns the untyped address.
    #[must_use]
    pub const fn id(self) -> NodeId {
        self.id
    }
}

impl<C> Clone for Handle<C> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<C> Copy for Handle<C> {}
impl<C> PartialEq for Handle<C> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<C> Eq for Handle<C> {}
impl<C> core::hash::Hash for Handle<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<C> fmt::Debug for Handle<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Handle").field(&self.id).finish()
    }
}

/// The addressed component no longer exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaleHandle;

impl fmt::Display for StaleHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("component no longer exists")
    }
}
impl core::error::Error for StaleHandle {}

/// A message went neither to the sending component nor to one of its live
/// children, so it was not queued.
///
/// [`Cx::send`](crate::Cx::send) reaches only those, so that a child cannot
/// message its parent even when it holds the parent's handle. A grandchild is
/// reached through its own parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotOwnChild;

impl fmt::Display for NotOwnChild {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the target is neither this component nor one of its children")
    }
}
impl core::error::Error for NotOwnChild {}

#[cfg(test)]
mod tests {
    use super::NodeId;

    #[test]
    fn bits_are_the_same_for_every_generation_issued() {
        // Generation in the high half and index in the low half, as always.
        for (index, generation) in [(0, 0), (7, 1 << 31), (u32::MAX - 1, u32::MAX - 1)] {
            let id = NodeId::new(index, generation);
            let bits = (u64::from(generation) << 32) | u64::from(index);
            assert_eq!(id.to_bits(), bits);
            assert_eq!(NodeId::from_bits(bits), id);
            assert_eq!(id.generation(), generation);
        }
    }

    #[test]
    fn bits_of_a_generation_never_issued_give_no_component() {
        assert_eq!(NodeId::from_bits(u64::MAX), NodeId::NONE);
    }
}
