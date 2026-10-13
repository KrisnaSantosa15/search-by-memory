//! Which Accelerator first-run setup offers and recommends for this machine.

use sbm_embed_onnx::Accelerator;
use sbm_setup::{Gpu, GpuVendor, SpeedOption, speed_options};

fn gpu(name: &str, vendor: GpuVendor, memory_gb: u64) -> Gpu {
    Gpu {
        name: name.to_string(),
        vendor,
        dedicated_memory: memory_gb << 30,
    }
}

fn accelerators(options: &[SpeedOption]) -> Vec<(Accelerator, bool)> {
    options
        .iter()
        .map(|option| (option.accelerator, option.recommended))
        .collect()
}

#[test]
fn nvidia_gpu_is_recommended_through_cuda_with_directml_and_cpu_as_alternatives() {
    let gpus = [gpu("NVIDIA GeForce RTX 4060", GpuVendor::Nvidia, 8)];

    let options = speed_options(&gpus);

    assert_eq!(
        accelerators(&options),
        [
            (Accelerator::Cuda, true),
            (Accelerator::DirectMl, false),
            (Accelerator::Cpu, false)
        ]
    );
    assert!(options[0].needs_cuda_runtime);
    assert_eq!(options[0].gpu.as_ref(), Some(&gpus[0]));
}

#[test]
fn other_graphics_card_is_recommended_through_directml() {
    let gpus = [gpu("AMD Radeon RX 7600", GpuVendor::Amd, 8)];

    let options = speed_options(&gpus);

    assert_eq!(
        accelerators(&options),
        [(Accelerator::DirectMl, true), (Accelerator::Cpu, false)]
    );
}
