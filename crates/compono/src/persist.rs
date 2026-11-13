use crate::component::node::Node;
use crate::saved_key::is_unsaved_entry;
use crate::tree::Tree;
use crate::{Component, Cx, Handle, NodeId};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;
use core::ops::RangeInclusive;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub(crate) type Saver = fn(&dyn Node) -> Result<SavedEntry, SaveError>;

/// The format of the bytes [`Tree::save`] writes. Bytes of any other format
/// start clean.
const FORMAT: u32 = 0x00c0_0002;

/// One component's saved state, tagged with what saved it.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct SavedEntry {
    /// [`kind_hash`] of the saving component's [`Persist::KIND`].
    kind: u64,
    /// The saving component's [`Persist::VERSION`].
    version: u32,
    bytes: Vec<u8>,
}

/// The FNV-1a hash of a kind, or 0 for a component that names none.
const fn kind_hash(kind: &str) -> u64 {
    if kind.is_empty() {
        0
    } else {
        crate::tree::fnv1a(crate::tree::FNV_OFFSET, kind.as_bytes())
    }
}

/// Decodes `bytes` as exactly one `T`, refusing bytes left after it.
fn decode_exact<T: DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    let (value, rest) = postcard::take_from_bytes(bytes).ok()?;
    rest.is_empty().then_some(value)
}

/// A component whose state survives process death.
///
/// A router is saved as part of the component that owns it: the owner's
/// `Saved` holds the router's snapshot, and its constructor passes that
/// snapshot back to the router.
///
/// ```
/// use compono::router::stack::{SavedStack, Stack};
/// use compono::{Component, Cx, Handle, Persist, RestoreOutcome, Tree};
/// # use compono::{Commit, Host};
/// # struct Quiet;
/// # impl Host for Quiet {
/// #     fn commit(&mut self, _commit: &Commit<'_>) {}
/// # }
///
/// struct Note {
///     text: String,
/// }
///
/// impl Component for Note {
///     type Msg = String;
///     type Output = ();
///     fn update(&mut self, text: String, _cx: &mut Cx<'_, Self>) {
///         self.text = text;
///     }
/// }
///
/// impl Persist for Note {
///     type Saved = String;
///     fn save(&self) -> String {
///         self.text.clone()
///     }
/// }
///
/// struct Notes {
///     stack: Stack<u32, Handle<Note>, Notes>,
/// }
///
/// impl Component for Notes {
///     type Msg = u32;
///     type Output = ();
///     fn update(&mut self, note: u32, cx: &mut Cx<'_, Self>) {
///         self.stack.push(note, cx);
///     }
/// }
///
/// impl Persist for Notes {
///     type Saved = SavedStack<u32>;
///     fn save(&self) -> SavedStack<u32> {
///         self.stack.snapshot()
///     }
/// }
///
/// fn mount(tree: &mut Tree) -> Handle<Notes> {
///     tree.mount_persistent(|cx| {
///         let saved = cx.restored();
///         let stack = Stack::restore_or(saved, 0, cx, |_note, cx| {
///             cx.persistent_child(
///                 |cx| Note { text: cx.restored().unwrap_or_default() },
///                 |_notes, (), _cx| {},
///             )
///         });
///         Notes { stack }
///     })
/// }
///
/// /// Raised whenever a `Saved` type changes shape.
/// const SCHEMA: u32 = 1;
///
/// let mut tree = Tree::new(Quiet);
/// let notes = mount(&mut tree);
/// tree.send(notes, 7)?;
/// if let Some(note) = tree.get(notes).map(|notes| *notes.stack.active()) {
///     tree.send(note, "milk".to_owned())?;
/// }
/// let bytes = tree.save(SCHEMA)?;
///
/// // The process ends here and starts again.
/// let (mut tree, outcome) = Tree::with_saved(Quiet, &bytes, SCHEMA);
/// assert_eq!(outcome, RestoreOutcome::Restored);
/// let notes = mount(&mut tree);
/// let stack = tree.get(notes).map(|notes| &notes.stack);
/// assert_eq!(stack.map(Stack::depth), Some(2));
/// let note = stack.and_then(|stack| tree.get(*stack.active()));
/// assert_eq!(note.map(|note| note.text.as_str()), Some("milk"));
///
/// // Bytes of another schema are refused, and the application starts clean.
/// let (_tree, outcome) = Tree::with_saved(Quiet, &bytes, SCHEMA + 1);
/// assert_eq!(outcome, RestoreOutcome::SchemaChanged);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait Persist: Component {
    /// The stored form of this component.
    type Saved: Serialize + DeserializeOwned;

    /// A name for this kind of component, the same in every version and unique
    /// in the application, such as `"notes.editor"`.
    ///
    /// Saved state records it, and [`Cx::restored`] gives `None` for state that
    /// a component of another kind saved at the same place. The default, an
    /// empty name, checks no kind, and such a component reads whatever was saved
    /// at its place. Giving a component a kind later discards the state it saved
    /// without one.
    const KIND: &'static str = "";

    /// The version of [`Persist::Saved`]. Raise it when `Saved` changes shape,
    /// and read the older shapes in [`Persist::migrate`].
    const VERSION: u32 = 0;

    /// Captures the current state.
    fn save(&self) -> Self::Saved;

    /// Reads state saved by `version` of this component, which differs from
    /// [`Persist::VERSION`].
    ///
    /// `bytes` are the postcard encoding of that version's `Saved`.
    /// [`Cx::restored`] calls this when the kind agrees and the version does
    /// not. The default reads nothing, so the component starts clean.
    fn migrate(version: u32, bytes: &[u8]) -> Option<Self::Saved> {
        let _ignored = (version, bytes);
        None
    }
}

/// The tree could not be encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveError;

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("component state could not be encoded")
    }
}
impl core::error::Error for SaveError {}

/// What happened to previously saved bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestoreOutcome {
    /// The saved state was accepted.
    Restored,
    /// The schema version differs, so the application starts clean.
    SchemaChanged,
    /// The bytes could not be decoded, so the application starts clean.
    Corrupt,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: u32,
    schema: u32,
    nodes: Vec<(Vec<u64>, SavedEntry)>,
}

fn save_node<K: Persist>(node: &dyn Node) -> Result<SavedEntry, SaveError> {
    let value = node.as_any().downcast_ref::<K>().ok_or(SaveError)?;
    let bytes = postcard::to_allocvec(&value.save()).map_err(|_error| SaveError)?;
    Ok(SavedEntry {
        kind: kind_hash(K::KIND),
        version: K::VERSION,
        bytes,
    })
}

impl Tree {
    /// Creates a tree that restores from `bytes` when their schema matches.
    #[must_use]
    pub fn with_saved(
        host: impl crate::Host + 'static,
        bytes: &[u8],
        schema: u32,
    ) -> (Self, RestoreOutcome) {
        let mut tree = Self::new(host);
        let outcome = match decode_exact::<Envelope>(bytes) {
            Some(envelope) if envelope.format == FORMAT && envelope.schema == schema => {
                tree.restored = envelope.nodes.into_iter().collect::<BTreeMap<_, _>>();
                RestoreOutcome::Restored
            }
            Some(_) => RestoreOutcome::SchemaChanged,
            None => RestoreOutcome::Corrupt,
        };
        (tree, outcome)
    }

    /// Encodes every persistent component into one flat buffer.
    ///
    /// State restored for entries that were never rebuilt is carried forward.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError`] if a component cannot be encoded.
    pub fn save(&self, schema: u32) -> Result<Vec<u8>, SaveError> {
        let mut nodes = BTreeMap::new();
        for (path, entry) in &self.restored {
            nodes.insert(path.clone(), entry.clone());
        }
        if let Some(root) = self.root {
            self.save_subtree(root, &mut nodes)?;
        }
        let envelope = Envelope {
            format: FORMAT,
            schema,
            nodes: nodes.into_iter().collect(),
        };
        postcard::to_allocvec(&envelope).map_err(|_error| SaveError)
    }

    /// Saves a component and everything below it.
    ///
    /// The walk uses the tree's own links and no call stack, so its depth is not
    /// limited.
    fn save_subtree(
        &self,
        root: NodeId,
        nodes: &mut BTreeMap<Vec<u64>, SavedEntry>,
    ) -> Result<(), SaveError> {
        let mut path = Vec::new();
        let mut cursor = root;
        // Entries of dropped routers that a work limit left for a later
        // operation. They are on their way out, and the router that replaced
        // them may already save under the same keys.
        let leaving = self.leaving();
        loop {
            let Some(slot) = self.slot(cursor) else {
                return Ok(());
            };
            path.push(slot.key);
            path.extend(slot.child.map(u64::from));
            // An entry of a router past the id limit, and all below it, is not saved.
            let saved = !is_unsaved_entry(slot.key) && !leaving.contains(&cursor);
            if saved && let (Some(saver), Some(node)) = (slot.saver, slot.node.as_ref()) {
                nodes.insert(path.clone(), saver(node.as_ref())?);
            }
            if saved && let Some(child) = slot.first_child {
                cursor = child;
                continue;
            }
            loop {
                let Some(slot) = self.slot(cursor) else {
                    return Ok(());
                };
                let own = 1 + usize::from(slot.child.is_some());
                path.truncate(path.len().saturating_sub(own));
                if cursor == root {
                    return Ok(());
                }
                if let Some(next) = slot.next_sibling {
                    cursor = next;
                    break;
                }
                let Some(parent) = slot.parent else {
                    return Ok(());
                };
                cursor = parent;
            }
        }
    }

    /// Removes restored state of one component and everything below it.
    pub(crate) fn forget_subtree(&mut self, id: NodeId) {
        if self.restored.is_empty() {
            return;
        }
        let mut prefix = self.path_of(id);
        if let Some(last) = prefix.pop() {
            self.forget_restored(&prefix, last..=last, |_| true);
        }
    }

    /// Removes restored state of the entries of `owner`'s routers whose saved
    /// keys lie in `keys` and pass `doomed`, and of everything below them.
    pub(crate) fn forget_entries(
        &mut self,
        owner: NodeId,
        keys: RangeInclusive<u64>,
        doomed: impl Fn(u64) -> bool,
    ) {
        if self.restored.is_empty() {
            return;
        }
        let prefix = self.path_of(owner);
        self.forget_restored(&prefix, keys, doomed);
    }

    /// Removes restored state saved under `prefix` followed by a key in `keys`
    /// that passes `doomed`.
    ///
    /// The scan starts at the first key of `keys` and stops after the last, so
    /// it reads only state saved under those keys.
    fn forget_restored(
        &mut self,
        prefix: &[u64],
        keys: RangeInclusive<u64>,
        doomed: impl Fn(u64) -> bool,
    ) {
        let mut start = prefix.to_vec();
        start.push(*keys.start());
        let after_prefix = |path: &Vec<u64>| path.strip_prefix(prefix)?.first().copied();
        let doomed: Vec<Vec<u64>> = self
            .restored
            .range(start..)
            .map(|(path, _)| path)
            .take_while(|path| after_prefix(path).is_some_and(|key| key <= *keys.end()))
            .filter(|path| after_prefix(path).is_some_and(&doomed))
            .cloned()
            .collect();
        for path in doomed {
            self.restored.remove(&path);
        }
    }

    fn path_of(&self, id: NodeId) -> Vec<u64> {
        let mut path = Vec::new();
        let mut cursor = Some(id);
        while let Some(slot) = cursor.and_then(|id| self.slot(id)) {
            // The path is built leaf first and reversed at the end.
            path.extend(slot.child.map(u64::from));
            path.push(slot.key);
            cursor = slot.parent;
        }
        path.reverse();
        path
    }

    /// Creates a persistent root component.
    pub fn mount_persistent<K: Persist>(
        &mut self,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
    ) -> Handle<K> {
        self.operate(Handle::new(crate::NodeId::NONE), |tree| {
            let handle = tree.mount_now(init);
            if let Some(slot) = tree.slot_mut(handle.id()) {
                slot.saver = Some(save_node::<K>);
            }
            handle
        })
    }
}

impl<C: Component> Cx<'_, C> {
    /// Creates a child whose state is saved with the tree.
    ///
    /// Outside a router entry, the order in which this parent creates children
    /// identifies them in saved state, so create them in the constructor in a
    /// fixed order, or name them with [`Cx::persistent_child_keyed`].
    pub fn persistent_child<K: Persist>(
        &mut self,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
        on_output: impl Fn(&mut C, K::Output, &mut Cx<'_, C>) + 'static,
    ) -> Handle<K> {
        self.persistent_child_named(None, init, on_output)
    }

    /// Creates a child whose state is saved under `name`, whatever else this
    /// parent creates and in whatever order. See [`Cx::child_keyed`].
    ///
    /// ```
    /// use compono::{Component, Cx, Handle, Persist, Tree};
    /// # use compono::{Commit, Host};
    /// # struct Quiet;
    /// # impl Host for Quiet {
    /// #     fn commit(&mut self, _commit: &Commit<'_>) {}
    /// # }
    ///
    /// struct Draft(String);
    ///
    /// impl Component for Draft {
    ///     type Msg = String;
    ///     type Output = ();
    ///     fn update(&mut self, text: String, _cx: &mut Cx<'_, Self>) {
    ///         self.0 = text;
    ///     }
    /// }
    ///
    /// impl Persist for Draft {
    ///     type Saved = String;
    ///     fn save(&self) -> String {
    ///         self.0.clone()
    ///     }
    /// }
    ///
    /// struct Editor {
    ///     draft: Handle<Draft>,
    /// }
    ///
    /// impl Component for Editor {
    ///     type Msg = ();
    ///     type Output = ();
    ///     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    /// }
    ///
    /// impl Persist for Editor {
    ///     type Saved = ();
    ///     fn save(&self) {}
    /// }
    ///
    /// fn editor(cx: &mut Cx<'_, Editor>) -> Editor {
    ///     let draft = cx.persistent_child_keyed(
    ///         "draft",
    ///         |cx| Draft(cx.restored().unwrap_or_default()),
    ///         |_editor: &mut Editor, (), _cx| {},
    ///     );
    ///     Editor { draft }
    /// }
    ///
    /// let mut tree = Tree::new(Quiet);
    /// let root = tree.mount_persistent(editor);
    /// if let Some(draft) = tree.get(root).map(|editor| editor.draft) {
    ///     tree.send(draft, "milk".to_owned())?;
    /// }
    /// let bytes = tree.save(1)?;
    ///
    /// // A later version creates other children first. The draft is still found.
    /// let (mut tree, _outcome) = Tree::with_saved(Quiet, &bytes, 1);
    /// let root = tree.mount_persistent(|cx| {
    ///     let _toolbar = cx.child(|_cx| Draft(String::new()), |_editor: &mut Editor, (), _cx| {});
    ///     editor(cx)
    /// });
    /// let draft = tree.get(root).and_then(|editor| tree.get(editor.draft));
    /// assert_eq!(draft.map(|draft| draft.0.as_str()), Some("milk"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn persistent_child_keyed<K: Persist>(
        &mut self,
        name: &str,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
        on_output: impl Fn(&mut C, K::Output, &mut Cx<'_, C>) + 'static,
    ) -> Handle<K> {
        self.persistent_child_named(Some(name), init, on_output)
    }

    fn persistent_child_named<K: Persist>(
        &mut self,
        name: Option<&str>,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
        on_output: impl Fn(&mut C, K::Output, &mut Cx<'_, C>) + 'static,
    ) -> Handle<K> {
        let handle = self.child_named(name, init, on_output);
        if let Some(slot) = self.tree.slot_mut(handle.id()) {
            slot.saver = Some(save_node::<K>);
        }
        handle
    }
}

impl<C: Persist> Cx<'_, C> {
    /// Takes this component's saved state. Unreadable state counts as absent.
    ///
    /// State saved by a component of another [`Persist::KIND`] is absent, and so
    /// are bytes left over after the value. State of another
    /// [`Persist::VERSION`] goes through [`Persist::migrate`].
    pub fn restored(&mut self) -> Option<C::Saved> {
        if self.tree.restored.is_empty() {
            return None;
        }
        let path = self.tree.path_of(self.me);
        let entry = self.tree.restored.remove(&path)?;
        if !C::KIND.is_empty() && entry.kind != kind_hash(C::KIND) {
            return None;
        }
        if entry.version == C::VERSION {
            decode_exact(&entry.bytes)
        } else {
            C::migrate(entry.version, &entry.bytes)
        }
    }
}
