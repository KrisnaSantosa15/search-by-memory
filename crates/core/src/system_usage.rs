use std::collections::BTreeMap;

/// One instance of a Windows performance counter: its instance name and value.
#[derive(Debug, Clone, PartialEq)]
pub struct CounterValue {
    pub instance: String,
    pub value: f64,
}

/// The Windows "GPU ..." performance counters, one value per instance.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuCounters {
    /// `GPU Engine(*)\Utilization Percentage`: one per process and engine.
    pub engines: Vec<CounterValue>,
}

/// How busy one graphics card is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuLoad {
    /// The adapter's LUID, as DXGI reports it.
    pub adapter: u64,
    /// Share of its busiest engine in use, from 0 to 100.
    pub percent: f64,
    /// The share of that used by this app's process.
    pub own_percent: f64,
}

/// How busy each graphics card is, as Task Manager counts it: the busiest of
/// its engines, each summed over every process.
pub fn gpu_loads(counters: &GpuCounters, own_pid: u32) -> Vec<GpuLoad> {
    // Per adapter, per engine: (all processes, this process).
    let mut engines: BTreeMap<u64, BTreeMap<&str, (f64, f64)>> = BTreeMap::new();
    for CounterValue { instance, value } in &counters.engines {
        let Some((pid, adapter, engine)) = engine_instance(instance) else {
            continue;
        };
        let load = engines.entry(adapter).or_default().entry(engine).or_default();
        load.0 += value;
        if pid == own_pid {
            load.1 += value;
        }
    }
    engines
        .into_iter()
        .map(|(adapter, engines)| GpuLoad {
            adapter,
            percent: engines.values().map(|load| load.0).fold(0.0, f64::max),
            own_percent: engines.values().map(|load| load.1).fold(0.0, f64::max),
        })
        .collect()
}

/// The process, adapter LUID and engine of an instance such as
/// `pid_1234_luid_0x00000000_0x0000C2F1_phys_0_eng_0_engtype_3D`.
fn engine_instance(instance: &str) -> Option<(u32, u64, &str)> {
    let rest = instance.strip_prefix("pid_")?;
    let (pid, rest) = rest.split_once('_')?;
    let rest = rest.strip_prefix("luid_0x")?;
    let (high, rest) = rest.split_once("_0x")?;
    let (low, rest) = rest.split_once('_')?;
    let engine = rest.split("_engtype_").next()?;
    let luid = (u64::from_str_radix(high, 16).ok()? << 32) | u64::from_str_radix(low, 16).ok()?;
    Some((pid.parse().ok()?, luid, engine))
}
