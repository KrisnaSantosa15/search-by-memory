use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::system_activity::platform::{CpuTimes, cpu_times};

/// How busy this computer is, and how much of that is this app, sampled on
/// demand. Processor shares cover the time since the previous sample.
#[derive(Default)]
pub struct SystemUsage {
    last: Mutex<Last>,
}

#[derive(Default)]
struct Last {
    cpu: Option<CpuTimes>,
    gpu: Option<platform::GpuQuery>,
}

/// One sample of [`SystemUsage`]; a part the system did not report is `None` or empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    /// `None` on the first sample, which has nothing to compare with.
    pub cpu: Option<CpuUsage>,
    pub memory: Option<MemoryUsage>,
    pub gpus: Vec<GpuLoad>,
}

/// Share of all processors' time in use since the previous sample, from 0 to 100.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuUsage {
    pub percent: f64,
    /// The share of that used by this app's process.
    pub own_percent: f64,
}

/// Physical memory, in bytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryUsage {
    pub used: u64,
    pub total: u64,
    /// Used by this app's process (its working set).
    pub own: u64,
}

impl SystemUsage {
    pub fn sample(&self) -> Usage {
        let mut last = self.last.lock().unwrap();
        let times = cpu_times();
        let cpu = match (&last.cpu, &times) {
            (Some(before), Some(after)) => cpu_usage(before, after),
            _ => None,
        };
        last.cpu = times;
        let counters = last
            .gpu
            .get_or_insert_with(platform::GpuQuery::open)
            .collect();
        Usage {
            cpu,
            memory: platform::memory(),
            gpus: gpu_loads(&counters, std::process::id()),
        }
    }
}

fn cpu_usage(before: &CpuTimes, after: &CpuTimes) -> Option<CpuUsage> {
    let all = after.all.saturating_sub(before.all);
    if all == 0 {
        return None;
    }
    let idle = after.idle.saturating_sub(before.idle);
    let own = after.own.saturating_sub(before.own);
    let share = |time: u64| (time as f64 * 100.0 / all as f64).clamp(0.0, 100.0);
    let percent = share(all.saturating_sub(idle));
    Some(CpuUsage {
        percent,
        own_percent: share(own).min(percent),
    })
}

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

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::System::Performance::{
        PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA,
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    use super::{CounterValue, GpuCounters, MemoryUsage};

    pub fn memory() -> Option<MemoryUsage> {
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        let mut own: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
        own.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let read = unsafe {
            GlobalMemoryStatusEx(&mut status) != 0
                && K32GetProcessMemoryInfo(GetCurrentProcess(), &mut own, own.cb) != 0
        };
        read.then(|| MemoryUsage {
            used: status.ullTotalPhys.saturating_sub(status.ullAvailPhys),
            total: status.ullTotalPhys,
            own: own.WorkingSetSize as u64,
        })
    }

    /// An open PDH query over the GPU counters. Utilization is a rate, so it
    /// reads from the second collection on.
    pub struct GpuQuery {
        query: isize,
        engines: isize,
        adapter_memory: isize,
        process_memory: isize,
    }

    // A PDH query handle may be used from any thread, one at a time.
    unsafe impl Send for GpuQuery {}

    impl GpuQuery {
        pub fn open() -> Self {
            let mut query = 0;
            if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) } != 0 {
                query = 0;
            }
            let counter = |path: &str| {
                let path: Vec<u16> = path.encode_utf16().chain([0]).collect();
                let mut counter = 0;
                if query == 0
                    || unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) } != 0
                {
                    return 0;
                }
                counter
            };
            Self {
                engines: counter(r"\GPU Engine(*)\Utilization Percentage"),
                adapter_memory: counter(r"\GPU Adapter Memory(*)\Dedicated Usage"),
                process_memory: counter(r"\GPU Process Memory(*)\Dedicated Usage"),
                query,
            }
        }

        pub fn collect(&self) -> GpuCounters {
            if self.query == 0 || unsafe { PdhCollectQueryData(self.query) } != 0 {
                return GpuCounters::default();
            }
            GpuCounters {
                engines: values(self.engines),
                adapter_memory: values(self.adapter_memory),
                process_memory: values(self.process_memory),
            }
        }
    }

    impl Drop for GpuQuery {
        fn drop(&mut self) {
            if self.query != 0 {
                unsafe { PdhCloseQuery(self.query) };
            }
        }
    }

    /// Every instance of `counter` with a valid value.
    fn values(counter: isize) -> Vec<CounterValue> {
        if counter == 0 {
            return Vec::new();
        }
        let (mut size, mut count) = (0u32, 0u32);
        let asked = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            )
        };
        if asked != PDH_MORE_DATA || size == 0 {
            return Vec::new();
        }
        // Items first, then the names they point to; u64 keeps the items aligned.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let items = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
        let read =
            unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) };
        if read != 0 {
            return Vec::new();
        }
        let items = unsafe { std::slice::from_raw_parts(items, count as usize) };
        items
            .iter()
            .filter(|item| item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA)
            .map(|item| CounterValue {
                instance: unsafe { wide(item.szName) },
                value: unsafe { item.FmtValue.Anonymous.doubleValue },
            })
            .collect()
    }

    unsafe fn wide(text: *const u16) -> String {
        if text.is_null() {
            return String::new();
        }
        let length = (0..).take_while(|&i| unsafe { *text.add(i) } != 0).count();
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) })
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{GpuCounters, MemoryUsage};

    /// Not measured outside Windows yet.
    pub fn memory() -> Option<MemoryUsage> {
        None
    }

    pub struct GpuQuery;

    impl GpuQuery {
        pub fn open() -> Self {
            GpuQuery
        }

        pub fn collect(&self) -> GpuCounters {
            GpuCounters::default()
        }
    }
}
