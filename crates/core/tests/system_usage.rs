//! How busy this computer and this app are, for the Engine monitor.

use sbm_core::{CounterValue, GpuCounters, GpuLoad, SystemUsage, gpu_loads};

const ADAPTER: &str = "luid_0x00000000_0x0000C2F1_phys_0";

fn value(instance: &str, value: f64) -> CounterValue {
    CounterValue {
        instance: instance.into(),
        value,
    }
}

fn engine(pid: u32, engine: u32, kind: &str, percent: f64) -> CounterValue {
    value(
        &format!("pid_{pid}_{ADAPTER}_eng_{engine}_engtype_{kind}"),
        percent,
    )
}

#[test]
fn a_gpus_load_is_its_busiest_engine_summed_over_processes_as_task_manager_counts_it() {
    let counters = GpuCounters {
        engines: vec![
            engine(100, 0, "3D", 20.0),
            engine(200, 0, "3D", 15.0),
            engine(200, 1, "Compute", 30.0),
        ],
        ..GpuCounters::default()
    };

    let loads = gpu_loads(&counters, 200);

    assert_eq!(loads.len(), 1);
    let GpuLoad { adapter, percent, own_percent, .. } = loads[0];
    assert_eq!(adapter, 0xC2F1);
    assert_eq!(percent, 35.0);
    assert_eq!(own_percent, 30.0);
}

#[test]
fn a_gpus_video_memory_in_use_counts_every_process_and_this_apps_own_share() {
    let counters = GpuCounters {
        adapter_memory: vec![value(ADAPTER, 4_000.0)],
        process_memory: vec![
            value(&format!("pid_100_{ADAPTER}"), 3_000.0),
            value(&format!("pid_200_{ADAPTER}"), 1_000.0),
        ],
        ..GpuCounters::default()
    };

    let loads = gpu_loads(&counters, 200);

    assert_eq!(loads.len(), 1);
    assert_eq!(loads[0].memory_used, 4_000);
    assert_eq!(loads[0].own_memory, 1_000);
    assert_eq!(loads[0].percent, 0.0);
}

#[test]
fn a_gpus_load_never_reads_above_full() {
    let counters = GpuCounters {
        engines: vec![engine(100, 0, "3D", 70.0), engine(200, 0, "3D", 45.0)],
        ..GpuCounters::default()
    };

    assert_eq!(gpu_loads(&counters, 200)[0].percent, 100.0);
}

#[cfg(windows)]
#[test]
fn windows_reports_how_busy_the_processor_and_memory_are_and_this_apps_share() {
    let usage = SystemUsage::default();
    usage.sample();
    // Some work of its own between the two samples.
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(300) {
        std::hint::black_box((0..1_000).sum::<u64>());
    }

    let now = usage.sample();

    let cpu = now.cpu.expect("a second sample measures the processor");
    assert!((0.0..=100.0).contains(&cpu.percent), "{cpu:?}");
    assert!(cpu.own_percent > 0.0 && cpu.own_percent <= cpu.percent, "{cpu:?}");
    let memory = now.memory.expect("memory is measured");
    assert!(memory.used > 0 && memory.used <= memory.total, "{memory:?}");
    assert!(memory.own > 0 && memory.own <= memory.used, "{memory:?}");
}

#[test]
fn the_first_sample_cannot_say_how_busy_the_processor_was() {
    assert_eq!(SystemUsage::default().sample().cpu, None);
}
