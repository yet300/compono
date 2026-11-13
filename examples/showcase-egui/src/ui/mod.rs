//! Views, one folder per feature. The same screens exist under the same names in
//! every UI of the showcase, and the components they draw are in `showcase::feature`.

mod articles;
mod book;
mod cards;
mod counters;
mod feed;
mod gallery;
mod menu;
mod root;
mod tabs;
mod widgets;

pub use root::root;
