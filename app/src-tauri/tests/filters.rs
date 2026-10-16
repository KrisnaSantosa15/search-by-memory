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
