//! The Engine monitor: how busy this computer is, and how much of that is the app.

use sbm_app::{CpuUsageRow, GpuUsageRow, ProcessorRow, device_usage};
use sbm_core::{CpuUsage, GpuLoad, Processor, Usage};
use sbm_setup::{Gpu, GpuVendor};

const GIB: u64 = 1 << 30;

fn rtx(adapter: u64) -> Gpu {
    Gpu {
        name: "NVIDIA GeForce RTX 4060".into(),
        vendor: GpuVendor::Nvidia,
        dedicated_memory: 8 * GIB,
        adapter,
    }
}

#[test]
fn names_each_graphics_card_beside_its_load_and_video_memory() {
    let usage = Usage {
        cpu: None,
        memory: None,
        gpus: vec![GpuLoad {
            adapter: 7,
            percent: 76.0,
            own_percent: 40.0,
            memory_used: 4 * GIB,
            own_memory: GIB,
        }],
    };

    let row = device_usage(&usage, None, &[rtx(7)]);

    assert_eq!(
        row.gpus,
        vec![GpuUsageRow {
            name: "NVIDIA GeForce RTX 4060".into(),
            percent: 76.0,
            own_percent: 40.0,
            memory_used: 4 * GIB,
            memory_total: 8 * GIB,
            own_memory: GIB,
        }]
    );
}

#[test]
fn names_the_processor_with_its_cores_threads_and_speed_in_gigahertz() {
    let usage = Usage {
        cpu: Some(CpuUsage {
            percent: 12.5,
            own_percent: 4.0,
        }),
        memory: None,
        gpus: Vec::new(),
    };
    let processor = Processor {
        name: "12th Gen Intel(R) Core(TM) i5-12450H".into(),
        cores: 8,
        threads: 12,
        mhz: 2_496,
    };

    let row = device_usage(&usage, Some(&processor), &[]);

    assert_eq!(
        row.cpu,
        Some(CpuUsageRow {
            percent: 12.5,
            own_percent: 4.0,
        })
    );
    assert_eq!(
        row.processor,
        Some(ProcessorRow {
            name: "12th Gen Intel(R) Core(TM) i5-12450H".into(),
            cores: 8,
            threads: 12,
            ghz: 2.5,
        })
    );
}
