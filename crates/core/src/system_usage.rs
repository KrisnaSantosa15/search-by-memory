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
    /// `GPU Adapter Memory(*)\Dedicated Usage`: one per adapter, in bytes.
    pub adapter_memory: Vec<CounterValue>,
    /// `GPU Process Memory(*)\Dedicated Usage`: one per process and adapter, in bytes.
    pub process_memory: Vec<CounterValue>,
}

/// How busy one graphics card is.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GpuLoad {
    /// The adapter's LUID, as DXGI reports it.
    pub adapter: u64,
    /// Share of its busiest engine in use, from 0 to 100.
    pub percent: f64,
    /// The share of that used by this app's process.
    pub own_percent: f64,
    /// Bytes of its own video memory in use by every process.
    pub memory_used: u64,
    /// Bytes of that used by this app's process.
    pub own_memory: u64,
}

/// How busy each graphics card is, as Task Manager counts it: the busiest of
/// its engines, each summed over every process.
pub fn gpu_loads(counters: &GpuCounters, own_pid: u32) -> Vec<GpuLoad> {
    // Per adapter, per engine: (all processes, this process).
    let mut engines: BTreeMap<u64, BTreeMap<&str, (f64, f64)>> = BTreeMap::new();
    for CounterValue { instance, value } in &counters.engines {
        let Some((pid, rest)) = process_instance(instance) else {
            continue;
        };
        let Some((adapter, rest)) = adapter_instance(rest) else {
            continue;
        };
        let engine = rest.split("_engtype_").next().unwrap_or(rest);
        let load = engines.entry(adapter).or_default().entry(engine).or_default();
        load.0 += value;
        if pid == own_pid {
            load.1 += value;
        }
    }
    let mut loads: BTreeMap<u64, GpuLoad> = BTreeMap::new();
    for (adapter, engines) in engines {
        let load = load_of(&mut loads, adapter);
        // Samples taken a moment apart can add up past full.
        let busiest = |share: fn(&(f64, f64)) -> f64| {
            engines.values().map(share).fold(0.0, f64::max).min(100.0)
        };
        load.percent = busiest(|load| load.0);
        load.own_percent = busiest(|load| load.1);
    }
    for CounterValue { instance, value } in &counters.adapter_memory {
        if let Some((adapter, _)) = adapter_instance(instance) {
            load_of(&mut loads, adapter).memory_used = *value as u64;
        }
    }
    for CounterValue { instance, value } in &counters.process_memory {
        let Some((pid, rest)) = process_instance(instance) else {
            continue;
        };
        if let Some((adapter, _)) = adapter_instance(rest) {
            let load = load_of(&mut loads, adapter);
            if pid == own_pid {
                load.own_memory += *value as u64;
            }
        }
    }
    loads.into_values().collect()
}

fn load_of(loads: &mut BTreeMap<u64, GpuLoad>, adapter: u64) -> &mut GpuLoad {
    loads.entry(adapter).or_insert_with(|| GpuLoad {
        adapter,
        ..GpuLoad::default()
    })
}

/// The process of an instance such as `pid_1234_luid_...`, and the rest of it.
fn process_instance(instance: &str) -> Option<(u32, &str)> {
    let (pid, rest) = instance.strip_prefix("pid_")?.split_once('_')?;
    Some((pid.parse().ok()?, rest))
}

/// The adapter LUID of an instance such as `luid_0x00000000_0x0000C2F1_phys_0...`,
/// and the rest of it after the physical adapter.
fn adapter_instance(instance: &str) -> Option<(u64, &str)> {
    let rest = instance.strip_prefix("luid_0x")?;
    let (high, rest) = rest.split_once("_0x")?;
    let (low, rest) = rest.split_once('_').unwrap_or((rest, ""));
    let luid = (u64::from_str_radix(high, 16).ok()? << 32) | u64::from_str_radix(low, 16).ok()?;
    let rest = rest.strip_prefix("phys_").map_or(rest, |phys| {
        phys.split_once('_').map_or("", |(_, rest)| rest)
    });
    Some((luid, rest))
}
