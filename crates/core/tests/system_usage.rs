//! How busy this computer and this app are, for the Engine monitor.

use sbm_core::{CounterValue, GpuCounters, GpuLoad, gpu_loads};

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
