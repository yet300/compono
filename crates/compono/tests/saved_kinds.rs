//! Saved state carries the kind and version of the component that saved it.

#![cfg(feature = "serde")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Commit, Component, Cx, Host, Persist, RestoreOutcome, Tree};

struct Quiet;
impl Host for Quiet {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

const SCHEMA: u32 = 1;

/// A component saving `S`, whose kind and version are set by the test.
struct Saving<S, const K: u8, const V: u32> {
    saved: Option<S>,
}

impl<S: 'static, const K: u8, const V: u32> Component for Saving<S, K, V> {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

const fn kind(k: u8) -> &'static str {
    match k {
        1 => "note",
        2 => "counter",
        _ => "",
    }
}

impl<S, const K: u8, const V: u32> Persist for Saving<S, K, V>
where
    S: serde::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    Saving<S, K, V>: Migrate<S>,
{
    type Saved = S;
    const KIND: &'static str = kind(K);
    const VERSION: u32 = V;
    fn save(&self) -> S {
        self.saved.clone().unwrap()
    }
    fn migrate(version: u32, bytes: &[u8]) -> Option<S> {
        <Self as Migrate<S>>::from_older(version, bytes)
    }
}

/// How a test component reads older state.
trait Migrate<S> {
    fn from_older(_version: u32, _bytes: &[u8]) -> Option<S> {
        None
    }
}

impl<const K: u8, const V: u32> Migrate<u32> for Saving<u32, K, V> {}
impl<const K: u8, const V: u32> Migrate<(u32, u32)> for Saving<(u32, u32), K, V> {}

/// Version 2 of a note keeps text, where version 1 kept a number.
impl<const K: u8> Migrate<String> for Saving<String, K, 2> {
    fn from_older(version: u32, bytes: &[u8]) -> Option<String> {
        (version == 1)
            .then(|| postcard::from_bytes::<u32>(bytes).ok())
            .flatten()
            .map(|number| format!("note {number}"))
    }
}

fn saved_by<S, const K: u8, const V: u32>(value: S) -> Vec<u8>
where
    Saving<S, K, V>: Persist<Saved = S>,
    S: 'static,
{
    let mut tree = Tree::new(Quiet);
    tree.mount_persistent(|_cx| Saving::<S, K, V> { saved: Some(value) });
    tree.save(SCHEMA).unwrap()
}

fn restored_by<S, const K: u8, const V: u32>(bytes: &[u8]) -> Option<S>
where
    Saving<S, K, V>: Persist<Saved = S>,
    S: 'static,
{
    let (mut tree, outcome) = Tree::with_saved(Quiet, bytes, SCHEMA);
    assert_eq!(outcome, RestoreOutcome::Restored);
    let mut restored = None;
    tree.mount_persistent(|cx| {
        restored = cx.restored();
        Saving::<S, K, V> { saved: None }
    });
    restored
}

#[test]
fn state_of_another_kind_at_the_same_path_is_not_given() {
    let bytes = saved_by::<u32, 1, 0>(7);
    assert_eq!(
        restored_by::<u32, 1, 0>(&bytes),
        Some(7),
        "the same kind reads it"
    );
    assert_eq!(
        restored_by::<u32, 2, 0>(&bytes),
        None,
        "another kind does not"
    );
}

#[test]
fn bytes_left_after_the_value_are_refused() {
    let bytes = saved_by::<(u32, u32), 1, 0>((7, 9));
    assert_eq!(restored_by::<u32, 1, 0>(&bytes), None);
}

#[test]
fn a_newer_version_reads_older_state_through_migrate() {
    let bytes = saved_by::<u32, 1, 1>(7);
    assert_eq!(
        restored_by::<String, 1, 2>(&bytes),
        Some("note 7".to_owned())
    );
}

#[test]
fn a_component_without_a_kind_restores_as_before() {
    let bytes = saved_by::<u32, 0, 0>(7);
    assert_eq!(restored_by::<u32, 0, 0>(&bytes), Some(7));
}

#[test]
fn bytes_in_the_format_before_tags_start_clean() {
    // The envelope as it was before entries carried a kind and a version.
    let entry: (Vec<u64>, Vec<u8>) = (vec![0], postcard::to_allocvec(&7_u32).unwrap());
    let old = postcard::to_allocvec(&(SCHEMA, vec![entry])).unwrap();
    let (_tree, outcome) = Tree::with_saved(Quiet, &old, SCHEMA);
    assert!(
        matches!(
            outcome,
            RestoreOutcome::Corrupt | RestoreOutcome::SchemaChanged
        ),
        "{outcome:?}"
    );
}
