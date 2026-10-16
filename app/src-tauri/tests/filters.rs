//! Filter prefixes typed into the launcher: `ext:`, `in:`, `date:`, `code:`, `name:`.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use jiff::Zoned;
use sbm_app::Filters;

/// A Friday afternoon in Jakarta.
fn now() -> Zoned {
    "2026-10-16T15:00[Asia/Jakarta]".parse().unwrap()
}

#[test]
fn ext_prefix_keeps_only_files_with_that_extension_and_leaves_the_rest_as_the_query() {
    let filters = Filters::parse("ext:pdf invoice", &now());

    assert_eq!(filters.query(), "invoice");
    assert!(filters.keeps(Path::new(r"D:\Work\March invoice.pdf")));
    assert!(!filters.keeps(Path::new(r"D:\Work\March invoice.docx")));
}

#[test]
fn several_ext_prefixes_keep_files_with_any_of_them_written_with_or_without_the_dot() {
    let filters = Filters::parse("budget ext:.XLSX ext:csv", &now());

    assert_eq!(filters.query(), "budget");
    assert!(filters.keeps(Path::new(r"D:\Work\Budget 2025.xlsx")));
    assert!(filters.keeps(Path::new(r"D:\Work\budget.CSV")));
    assert!(!filters.keeps(Path::new(r"D:\Work\budget.pdf")));
}

#[test]
fn in_prefix_keeps_files_inside_a_folder_whose_name_contains_it() {
    let filters = Filters::parse("in:tax receipt", &now());

    assert_eq!(filters.query(), "receipt");
    assert!(filters.keeps(Path::new(r"D:\Documents\Taxes 2024\receipt.pdf")));
    assert!(!filters.keeps(Path::new(r"D:\Documents\receipt tax.pdf")));
}

#[test]
fn a_quoted_value_may_hold_spaces() {
    let filters = Filters::parse(r#"receipt in:"taxes 2024" march"#, &now());

    assert_eq!(filters.query(), "receipt march");
    assert!(filters.keeps(Path::new(r"D:\Documents\Taxes 2024\receipt.pdf")));
    assert!(!filters.keeps(Path::new(r"D:\Documents\Taxes 2023\2024 receipt.pdf")));
}

#[test]
fn a_prefix_with_nothing_after_it_yet_filters_nothing() {
    for typed in [
        "invoice ext:",
        "invoice in:",
        r#"invoice in:"""#,
        "invoice date:",
    ] {
        let filters = Filters::parse(typed, &now());

        assert_eq!(filters.query(), "invoice", "{typed}");
        assert!(filters.keeps(Path::new(r"D:\Work\invoice.docx")), "{typed}");
    }
}

#[test]
fn unknown_prefixes_stay_in_the_query() {
    let typed = r"note: see https://example.org and C:\Temp\x";

    assert_eq!(Filters::parse(typed, &now()).query(), typed);
}

#[test]
fn code_prefix_keeps_only_code_and_what_follows_it_stays_in_the_query() {
    for typed in [
        "code:parse config",
        "parse code: config",
        "CODE: parse config",
    ] {
        let filters = Filters::parse(typed, &now());

        assert_eq!(filters.query(), "parse config", "{typed}");
        assert!(filters.keeps(Path::new(r"D:\src\config.rs")), "{typed}");
        assert!(!filters.keeps(Path::new(r"D:\Docs\config.pdf")), "{typed}");
    }
}

/// A file in `folder` last modified at `time`.
fn modified_at(folder: &Path, name: &str, time: &str) -> PathBuf {
    let path = folder.join(name);
    let file = File::create(&path).unwrap();
    let time: Zoned = time.parse().unwrap();
    file.set_modified(SystemTime::from(time.timestamp()))
        .unwrap();
    path
}

#[test]
fn date_today_keeps_files_modified_since_midnight() {
    let folder = tempfile::tempdir().unwrap();
    let this_morning = modified_at(folder.path(), "a.txt", "2026-10-16T09:00[Asia/Jakarta]");
    let last_night = modified_at(folder.path(), "b.txt", "2026-10-15T23:30[Asia/Jakarta]");

    let filters = Filters::parse("date:today notes", &now());

    assert_eq!(filters.query(), "notes");
    assert!(filters.keeps(&this_morning));
    assert!(!filters.keeps(&last_night));
}

#[test]
fn date_yesterday_keeps_files_modified_the_day_before() {
    let folder = tempfile::tempdir().unwrap();
    let this_morning = modified_at(folder.path(), "a.txt", "2026-10-16T09:00[Asia/Jakarta]");
    let last_night = modified_at(folder.path(), "b.txt", "2026-10-15T23:30[Asia/Jakarta]");
    let two_days_ago = modified_at(folder.path(), "c.txt", "2026-10-14T23:30[Asia/Jakarta]");

    let filters = Filters::parse("date:Yesterday", &now());

    assert!(filters.keeps(&last_night));
    assert!(!filters.keeps(&this_morning));
    assert!(!filters.keeps(&two_days_ago));
}

#[test]
fn date_week_month_and_year_keep_files_modified_in_the_last_7_30_and_365_days() {
    let folder = tempfile::tempdir().unwrap();
    for (word, inside, outside) in [
        ("week", "2026-10-09T08:00", "2026-10-08T23:00"),
        ("month", "2026-09-16T08:00", "2026-09-15T23:00"),
        ("year", "2025-10-16T08:00", "2025-10-15T23:00"),
    ] {
        let inside = modified_at(folder.path(), "in.txt", &format!("{inside}[Asia/Jakarta]"));
        let outside = modified_at(
            folder.path(),
            "out.txt",
            &format!("{outside}[Asia/Jakarta]"),
        );

        let filters = Filters::parse(&format!("date:{word}"), &now());

        assert!(filters.keeps(&inside), "{word}");
        assert!(!filters.keeps(&outside), "{word}");
    }
}

#[test]
fn date_with_a_year_or_a_month_keeps_files_modified_in_that_calendar_span() {
    let folder = tempfile::tempdir().unwrap();
    for (value, inside, outside) in [
        ("2025", "2025-12-31T23:00", "2026-01-01T01:00"),
        ("2025", "2025-01-01T01:00", "2024-12-31T23:00"),
        ("2025-03", "2025-03-31T23:00", "2025-04-01T01:00"),
        ("2025-03", "2025-03-01T01:00", "2025-02-28T23:00"),
    ] {
        let inside = modified_at(folder.path(), "in.txt", &format!("{inside}[Asia/Jakarta]"));
        let outside = modified_at(
            folder.path(),
            "out.txt",
            &format!("{outside}[Asia/Jakarta]"),
        );

        let filters = Filters::parse(&format!("report date:{value}"), &now());

        assert_eq!(filters.query(), "report", "{value}");
        assert!(filters.keeps(&inside), "{value}");
        assert!(!filters.keeps(&outside), "{value}");
    }
}
