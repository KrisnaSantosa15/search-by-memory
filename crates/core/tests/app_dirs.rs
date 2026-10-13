use std::ffi::OsString;
use std::path::PathBuf;

use sbm_core::{AppDirs, Platform};

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let vars: Vec<(String, String)> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| {
        vars.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| OsString::from(v))
    }
}

#[test]
fn windows_data_and_cache_live_in_local_app_data() {
    let dirs = AppDirs::from_env(
        Platform::Windows,
        env(&[(r"LOCALAPPDATA", r"C:\Users\ana\AppData\Local")]),
    );

    let root = PathBuf::from(r"C:\Users\ana\AppData\Local").join("search-by-memory");
    assert_eq!(dirs.data, root);
    assert_eq!(dirs.cache, root.join("cache"));
}

#[test]
fn unix_data_and_cache_follow_the_xdg_defaults_under_home() {
    let dirs = AppDirs::from_env(Platform::Unix, env(&[("HOME", "/home/ana")]));

    assert_eq!(dirs.data, PathBuf::from("/home/ana/.local/share/search-by-memory"));
    assert_eq!(dirs.cache, PathBuf::from("/home/ana/.cache/search-by-memory"));
}
