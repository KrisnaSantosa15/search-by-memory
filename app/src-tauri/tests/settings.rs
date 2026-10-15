//! Settings: what onboarding decided, kept so the next start skips it.

use sbm_app::Settings;

#[test]
fn saved_settings_come_back_on_the_next_start() {
    let data = tempfile::tempdir().unwrap();
    let settings = Settings {
        accelerator: "cuda".into(),
        hotkey: "Alt+Space".into(),
    };

    settings.save(data.path()).unwrap();

    assert_eq!(Settings::load(data.path()), Some(settings));
}

#[test]
fn without_saved_settings_onboarding_is_not_finished() {
    let data = tempfile::tempdir().unwrap();

    assert_eq!(Settings::load(data.path()), None);
}

#[test]
fn forgotten_settings_send_the_next_start_back_to_onboarding() {
    let data = tempfile::tempdir().unwrap();
    let settings = Settings {
        accelerator: "cuda".into(),
        hotkey: "Alt+Space".into(),
    };
    settings.save(data.path()).unwrap();

    Settings::forget(data.path()).unwrap();
    Settings::forget(data.path()).unwrap();

    assert_eq!(Settings::load(data.path()), None);
}
