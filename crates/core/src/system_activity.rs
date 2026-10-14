use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{Activity, ThrottleReason};

/// Share of all CPU time that other processes must use for the computer to be busy.
const BUSY_SHARE: f64 = 0.5;
/// Shortest time between two CPU samples; asked sooner, the last answer stands.
const SAMPLE_EVERY: Duration = Duration::from_secs(1);

/// What the computer is doing, as the operating system reports it.
#[derive(Default)]
pub struct SystemActivity {
    cpu: Mutex<Option<CpuReading>>,
}

/// CPU times at one moment, and whether other work kept the CPU busy before it.
struct CpuReading {
    at: Instant,
    times: platform::CpuTimes,
    busy: bool,
}

impl SystemActivity {
    /// Whether other processes used more than `BUSY_SHARE` of the CPU since the
    /// last sample. This process does not count: indexing must not hold itself back.
    fn busy(&self) -> bool {
        let mut cpu = self.cpu.lock().unwrap();
        if let Some(last) = cpu.as_ref().filter(|last| last.at.elapsed() < SAMPLE_EVERY) {
            return last.busy;
        }
        let Some(times) = platform::cpu_times() else {
            return false;
        };
        let busy = cpu.as_ref().is_some_and(|last| {
            let all = times.all.saturating_sub(last.times.all);
            let idle = times.idle.saturating_sub(last.times.idle);
            let own = times.own.saturating_sub(last.times.own);
            let others = all.saturating_sub(idle).saturating_sub(own);
            all > 0 && others as f64 / all as f64 > BUSY_SHARE
        });
        *cpu = Some(CpuReading {
            at: Instant::now(),
            times,
            busy,
        });
        busy
    }
}

impl Activity for SystemActivity {
    fn throttle_reason(&self) -> Option<ThrottleReason> {
        if platform::full_screen() {
            Some(ThrottleReason::FullScreen)
        } else if platform::on_battery() {
            Some(ThrottleReason::Battery)
        } else {
            self.busy().then_some(ThrottleReason::Busy)
        }
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetProcessTimes, GetSystemTimes,
    };
    use windows_sys::Win32::UI::Shell::{
        QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        SHQueryUserNotificationState,
    };

    /// A full-screen game, video, or presentation: the moments Windows itself
    /// keeps notifications quiet for.
    pub fn full_screen() -> bool {
        let mut state = 0;
        let read = unsafe { SHQueryUserNotificationState(&mut state) } >= 0;
        read && [
            QUNS_BUSY,
            QUNS_RUNNING_D3D_FULL_SCREEN,
            QUNS_PRESENTATION_MODE,
        ]
        .contains(&state)
    }

    /// Unplugged; a desktop without a battery reports mains power.
    pub fn on_battery() -> bool {
        let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        unsafe { GetSystemPowerStatus(&mut status) != 0 && status.ACLineStatus == 0 }
    }

    /// CPU time in 100 ns units, summed over all processors.
    pub struct CpuTimes {
        /// Time spent by all processors, idle included.
        pub all: u64,
        pub idle: u64,
        /// Time spent by this process.
        pub own: u64,
    }

    pub fn cpu_times() -> Option<CpuTimes> {
        let zero = || FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut idle, mut kernel, mut user) = (zero(), zero(), zero());
        let (mut created, mut exited, mut own_kernel, mut own_user) =
            (zero(), zero(), zero(), zero());
        let read = unsafe {
            GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0
                && GetProcessTimes(
                    GetCurrentProcess(),
                    &mut created,
                    &mut exited,
                    &mut own_kernel,
                    &mut own_user,
                ) != 0
        };
        let ticks =
            |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        // Kernel time includes idle time.
        read.then(|| CpuTimes {
            all: ticks(kernel) + ticks(user),
            idle: ticks(idle),
            own: ticks(own_kernel) + ticks(own_user),
        })
    }
}

#[cfg(not(windows))]
mod platform {
    pub struct CpuTimes {
        pub all: u64,
        pub idle: u64,
        pub own: u64,
    }

    /// Not detected outside Windows yet.
    pub fn full_screen() -> bool {
        false
    }

    pub fn on_battery() -> bool {
        false
    }

    /// Not measured outside Windows yet: never busy.
    pub fn cpu_times() -> Option<CpuTimes> {
        None
    }
}
