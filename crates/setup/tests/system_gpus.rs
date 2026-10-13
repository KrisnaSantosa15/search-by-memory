//! The graphics cards of this machine, from the operating system.

use sbm_setup::system_gpus;

#[cfg(windows)]
#[test]
fn windows_lists_this_machines_graphics_cards_but_not_software_renderers() {
    let gpus = system_gpus();

    assert!(!gpus.is_empty(), "this test expects a machine with a GPU");
    for gpu in &gpus {
        assert!(!gpu.name.trim().is_empty(), "{gpu:?}");
        assert!(!gpu.name.contains("Basic Render"), "{gpu:?}");
    }
}
