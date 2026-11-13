//! The checked-in golden table is what the presets give today.

#[path = "../examples/golden/table.rs"]
mod table;

#[test]
fn the_golden_table_is_current() {
    let expected = table::table();
    let actual = include_str!("../golden/motion.csv");
    assert!(
        actual == expected,
        "golden/motion.csv is stale: run `cargo run -p compono-motion --example golden > crates/compono-motion/golden/motion.csv`"
    );
}

#[test]
fn every_row_has_every_column() {
    let columns = table::HEADER.split(',').count();
    for line in table::table().lines() {
        assert_eq!(line.split(',').count(), columns, "{line}");
    }
}
