//! Onboarding: what the wizard offers, as facts the UI phrases for people.

use sbm_app::speed_choices;
use sbm_setup::{Gpu, GpuVendor, cuda_runtime};

fn gpu(name: &str, vendor: GpuVendor, gib: u64) -> Gpu {
    Gpu {
        name: name.into(),
        vendor,
        dedicated_memory: gib << 30,
    }
}

#[test]
fn nvidia_card_is_offered_first_with_the_runtime_download_it_needs() {
    let gpus = [gpu("NVIDIA GeForce RTX 4060", GpuVendor::Nvidia, 8)];

    let choices = speed_choices(&gpus);

    let ids: Vec<&str> = choices.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["cuda", "directml", "cpu"]);
    assert!(choices[0].recommended);
    assert_eq!(
        choices[0].graphics_card.as_deref(),
        Some("NVIDIA GeForce RTX 4060")
    );
    let runtime: u64 = cuda_runtime().iter().map(|file| file.size).sum();
    assert_eq!(choices[0].extra_download, runtime);
    assert_eq!(
        (choices[1].extra_download, choices[2].extra_download),
        (0, 0)
    );
    assert_eq!(choices[2].graphics_card, None);
}
