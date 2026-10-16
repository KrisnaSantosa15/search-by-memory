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

#[cfg(windows)]
#[test]
fn windows_graphics_cards_carry_the_adapter_id_their_usage_is_reported_under() {
    let usage = sbm_core::SystemUsage::default();
    usage.sample();
    let loads = usage.sample().gpus;

    for gpu in system_gpus() {
        assert!(
            loads.iter().any(|load| load.adapter == gpu.adapter),
            "{gpu:?} not in {loads:?}"
        );
    }
}
