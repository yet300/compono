//! A list reports its window with spare rows, so a fast scroll finds rows ready.

use compono::router::items::next_window;

#[test]
fn a_first_report_adds_spare_rows_on_each_side() {
    assert_eq!(next_window(20..30, 0..0, 10, 1000), Some(10..40));
}

#[test]
fn spare_rows_stop_at_the_ends_of_the_list() {
    assert_eq!(next_window(0..5, 0..0, 10, 12), Some(0..12));
    assert_eq!(next_window(995..1000, 0..0, 10, 1000), Some(985..1000));
    assert_eq!(
        next_window(0..0, 0..0, 10, 0),
        None,
        "an empty list has no window"
    );
}

#[test]
fn scrolling_inside_the_window_reports_nothing() {
    assert_eq!(next_window(15..25, 10..40, 10, 1000), None);
    assert_eq!(next_window(30..40, 10..40, 10, 1000), None);
}

#[test]
fn leaving_the_window_moves_it_with_the_spare_rows() {
    assert_eq!(next_window(35..45, 10..40, 10, 1000), Some(25..55));
    assert_eq!(next_window(5..15, 10..40, 10, 1000), Some(0..25));
}

#[test]
fn huge_values_do_not_overflow() {
    assert_eq!(
        next_window(usize::MAX - 5..usize::MAX, 0..0, usize::MAX, usize::MAX),
        Some(0..usize::MAX)
    );
}
