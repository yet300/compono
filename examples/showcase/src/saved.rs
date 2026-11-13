use compono::{Host, Tree};

/// Version of the saved-state layout. Raise it when a saved type changes.
pub const SCHEMA: u32 = 3;

/// Creates a tree that resumes from `saved` when its schema matches.
///
/// Anything else, including bytes from an older layout, starts fresh.
pub fn tree(host: impl Host + 'static, saved: Option<&[u8]>) -> Tree {
    match saved {
        Some(bytes) => Tree::with_saved(host, bytes, SCHEMA).0,
        None => Tree::new(host),
    }
}

/// Encodes the tree for [`tree`], or `None` if a component could not be encoded.
#[must_use]
pub fn save(tree: &Tree) -> Option<Vec<u8>> {
    tree.save(SCHEMA).ok()
}
