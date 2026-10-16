//! Filter prefixes typed into the launcher: `ext:`, `in:`, `date:`, `code:`, `name:`.

use std::path::Path;

use sbm_app::Filters;

#[test]
fn ext_prefix_keeps_only_files_with_that_extension_and_leaves_the_rest_as_the_query() {
    let filters = Filters::parse("ext:pdf invoice");

    assert_eq!(filters.query(), "invoice");
    assert!(filters.keeps(Path::new(r"D:\Work\March invoice.pdf")));
    assert!(!filters.keeps(Path::new(r"D:\Work\March invoice.docx")));
}

#[test]
fn several_ext_prefixes_keep_files_with_any_of_them_written_with_or_without_the_dot() {
    let filters = Filters::parse("budget ext:.XLSX ext:csv");

    assert_eq!(filters.query(), "budget");
    assert!(filters.keeps(Path::new(r"D:\Work\Budget 2025.xlsx")));
    assert!(filters.keeps(Path::new(r"D:\Work\budget.CSV")));
    assert!(!filters.keeps(Path::new(r"D:\Work\budget.pdf")));
}

#[test]
fn in_prefix_keeps_files_inside_a_folder_whose_name_contains_it() {
    let filters = Filters::parse("in:tax receipt");

    assert_eq!(filters.query(), "receipt");
    assert!(filters.keeps(Path::new(r"D:\Documents\Taxes 2024\receipt.pdf")));
    assert!(!filters.keeps(Path::new(r"D:\Documents\receipt tax.pdf")));
}

#[test]
fn a_quoted_value_may_hold_spaces() {
    let filters = Filters::parse(r#"receipt in:"taxes 2024" march"#);

    assert_eq!(filters.query(), "receipt march");
    assert!(filters.keeps(Path::new(r"D:\Documents\Taxes 2024\receipt.pdf")));
    assert!(!filters.keeps(Path::new(r"D:\Documents\Taxes 2023\2024 receipt.pdf")));
}

#[test]
fn code_prefix_keeps_only_code_and_what_follows_it_stays_in_the_query() {
    for typed in ["code:parse config", "parse code: config", "CODE: parse config"] {
        let filters = Filters::parse(typed);

        assert_eq!(filters.query(), "parse config", "{typed}");
        assert!(filters.keeps(Path::new(r"D:\src\config.rs")), "{typed}");
        assert!(!filters.keeps(Path::new(r"D:\Docs\config.pdf")), "{typed}");
    }
}
