//! Generated content, so the screen has something to show.

const WORDS: [&str; 8] = [
    "ownership",
    "tree",
    "phase",
    "router",
    "handle",
    "commit",
    "message",
    "panel",
];

/// An article.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Article {
    /// Its id.
    pub id: u32,
    /// Its title.
    pub title: String,
    /// Its text.
    pub body: String,
    /// Who wrote it.
    pub author: u32,
}

/// An author.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Author {
    /// Their id.
    pub id: u32,
    /// Their name.
    pub name: String,
}

fn word(seed: u32) -> &'static str {
    WORDS
        .get(usize::try_from(seed).unwrap_or(0) % WORDS.len())
        .copied()
        .unwrap_or("word")
}

/// Returns the article with `id`.
#[must_use]
pub fn article(id: u32) -> Article {
    let body: Vec<&str> = (0..40)
        .map(|index| word(id.wrapping_mul(31) ^ index))
        .collect();
    Article {
        id,
        title: format!("Article {id}: {}", word(id)),
        body: body.join(" "),
        author: id % 50,
    }
}

/// Returns the author with `id`.
#[must_use]
pub fn author(id: u32) -> Author {
    Author {
        id,
        name: format!("Author {id}"),
    }
}
