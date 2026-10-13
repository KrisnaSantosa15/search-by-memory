use std::time::Duration;

use sbm_core::Location;

fn lines(start: u32, end: u32) -> Location {
    Location::Lines { start, end }
}

fn time(start: u64, end: u64) -> Location {
    Location::Time {
        start: Duration::from_secs(start),
        end: Duration::from_secs(end),
    }
}

#[test]
fn same_page_overlaps_and_other_pages_do_not() {
    assert!(Location::Page(3).overlaps(&Location::Page(3)));
    assert!(!Location::Page(3).overlaps(&Location::Page(4)));
}

#[test]
fn ranges_overlap_when_they_share_any_line_or_moment() {
    assert!(lines(10, 24).overlaps(&lines(24, 30)));
    assert!(!lines(10, 24).overlaps(&lines(25, 30)));
    assert!(time(90, 130).overlaps(&time(120, 200)));
    assert!(!time(90, 130).overlaps(&time(131, 200)));
}

#[test]
fn whole_file_overlaps_any_location_in_the_file() {
    assert!(Location::WholeFile.overlaps(&Location::Page(7)));
    assert!(lines(1, 2).overlaps(&Location::WholeFile));
}

#[test]
fn locations_of_different_kinds_do_not_overlap() {
    assert!(!Location::Page(1).overlaps(&lines(1, 1)));
}
