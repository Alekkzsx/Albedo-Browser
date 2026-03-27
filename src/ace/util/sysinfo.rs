use std::time::Instant;

#[derive(Clone, Copy, Debug)]
struct CpuSample {
    total: u64,
    idle: u64,
}

pub struct AceSysInfo {
    cpu_count: usize,
    cpu_usage: f32,
    total_memory: u64,
    used_memory: u64,
    prev_cpu_sample: Option<CpuSample>,
    _last_refresh: Option<Instant>,
}

impl AceSysInfo {
    pub fn new() -> Self {
        let mut s = Self {
            cpu_count: cpu_count_platform(),
            cpu_usage: 0.0,
            total_memory: 0,
            used_memory: 0,
            prev_cpu_sample: None,
            _last_refresh: None,
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        self.cpu_count = cpu_count_platform();
        self.cpu_usage = cpu_usage_platform(&mut self.prev_cpu_sample);
        let (total, used) = memory_platform();
        self.total_memory = total;
        self.used_memory = used;
        self._last_refresh = Some(Instant::now());
    }

    pub fn cpu_count(&self) -> usize {
        self.cpu_count
    }

    pub fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub fn total_memory(&self) -> u64 {
        self.total_memory
    }

    pub fn used_memory(&self) -> u64 {
        self.used_memory
    }
}

#[cfg(target_os = "windows")]
fn cpu_count_platform() -> usize {
    #[allow(non_snake_case)]
    #[repr(C)]
    struct SYSTEM_INFO {
        wProcessorArchitecture: u16,
        wReserved: u16,
        dwPageSize: u32,
        lpMinimumApplicationAddress: *mut core::ffi::c_void,
        lpMaximumApplicationAddress: *mut core::ffi::c_void,
        dwActiveProcessorMask: usize,
        dwNumberOfProcessors: u32,
        dwProcessorType: u32,
        dwAllocationGranularity: u32,
        wProcessorLevel: u16,
        wProcessorRevision: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemInfo(lpSystemInfo: *mut SYSTEM_INFO);
    }

    let mut info = SYSTEM_INFO {
        wProcessorArchitecture: 0,
        wReserved: 0,
        dwPageSize: 0,
        lpMinimumApplicationAddress: core::ptr::null_mut(),
        lpMaximumApplicationAddress: core::ptr::null_mut(),
        dwActiveProcessorMask: 0,
        dwNumberOfProcessors: 0,
        dwProcessorType: 0,
        dwAllocationGranularity: 0,
        wProcessorLevel: 0,
        wProcessorRevision: 0,
    };

    unsafe { GetSystemInfo(&mut info as *mut SYSTEM_INFO) };
    if info.dwNumberOfProcessors == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        info.dwNumberOfProcessors as usize
    }
}

#[cfg(target_os = "windows")]
fn cpu_usage_platform(prev: &mut Option<CpuSample>) -> f32 {
    #[allow(non_snake_case)]
    #[repr(C)]
    struct FILETIME {
        dwLowDateTime: u32,
        dwHighDateTime: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimes(
            lpIdleTime: *mut FILETIME,
            lpKernelTime: *mut FILETIME,
            lpUserTime: *mut FILETIME,
        ) -> i32;
    }

    fn ft_to_u64(ft: &FILETIME) -> u64 {
        ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
    }

    let mut idle = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return 0.0;
    }

    let idle_t = ft_to_u64(&idle);
    let total = ft_to_u64(&kernel) + ft_to_u64(&user);
    let current = CpuSample {
        total,
        idle: idle_t,
    };

    let usage = if let Some(prev_sample) = prev {
        let total_delta = current.total.saturating_sub(prev_sample.total);
        let idle_delta = current.idle.saturating_sub(prev_sample.idle);
        if total_delta == 0 {
            0.0
        } else {
            ((total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64 * 100.0) as f32
        }
    } else {
        0.0
    };

    *prev = Some(current);
    usage.clamp(0.0, 100.0)
}

#[cfg(target_os = "windows")]
fn memory_platform() -> (u64, u64) {
    #[allow(non_snake_case)]
    #[repr(C)]
    struct MEMORYSTATUSEX {
        dwLength: u32,
        dwMemoryLoad: u32,
        ullTotalPhys: u64,
        ullAvailPhys: u64,
        ullTotalPageFile: u64,
        ullAvailPageFile: u64,
        ullTotalVirtual: u64,
        ullAvailVirtual: u64,
        ullAvailExtendedVirtual: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
    }

    let mut mem = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    let ok = unsafe { GlobalMemoryStatusEx(&mut mem as *mut MEMORYSTATUSEX) };
    if ok == 0 || mem.ullTotalPhys == 0 {
        return (0, 0);
    }

    let total_mb = mem.ullTotalPhys / (1024 * 1024);
    let used_mb = (mem.ullTotalPhys.saturating_sub(mem.ullAvailPhys)) / (1024 * 1024);
    (total_mb, used_mb)
}

#[cfg(target_os = "linux")]
fn cpu_count_platform() -> usize {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        let count = cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        if count > 0 {
            return count;
        }
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(target_os = "linux")]
fn cpu_usage_platform(prev: &mut Option<CpuSample>) -> f32 {
    let Ok(stat) = std::fs::read_to_string("/proc/stat") else {
        return 0.0;
    };
    let Some(first) = stat.lines().next() else {
        return 0.0;
    };
    if !first.starts_with("cpu ") {
        return 0.0;
    }

    let nums: Vec<u64> = first
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    if nums.len() < 4 {
        return 0.0;
    }

    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total: u64 = nums.iter().copied().sum();
    let current = CpuSample { total, idle };

    let usage = if let Some(prev_sample) = prev {
        let total_delta = current.total.saturating_sub(prev_sample.total);
        let idle_delta = current.idle.saturating_sub(prev_sample.idle);
        if total_delta == 0 {
            0.0
        } else {
            ((total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64 * 100.0) as f32
        }
    } else {
        0.0
    };

    *prev = Some(current);
    usage.clamp(0.0, 100.0)
}

#[cfg(target_os = "linux")]
fn memory_platform() -> (u64, u64) {
    let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") else {
        return (0, 0);
    };

    let mut mem_total_kb = 0u64;
    let mut mem_available_kb = None;
    let mut mem_free_kb = 0u64;
    let mut buffers_kb = 0u64;
    let mut cached_kb = 0u64;

    for line in meminfo.lines() {
        if let Some((key, value_part)) = line.split_once(':') {
            let value_kb = value_part
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            match key {
                "MemTotal" => mem_total_kb = value_kb,
                "MemAvailable" => mem_available_kb = Some(value_kb),
                "MemFree" => mem_free_kb = value_kb,
                "Buffers" => buffers_kb = value_kb,
                "Cached" => cached_kb = value_kb,
                _ => {}
            }
        }
    }

    if mem_total_kb == 0 {
        return (0, 0);
    }

    let available_kb = mem_available_kb.unwrap_or(mem_free_kb + buffers_kb + cached_kb);
    let used_kb = mem_total_kb.saturating_sub(available_kb);
    (mem_total_kb / 1024, used_kb / 1024)
}

#[cfg(target_os = "macos")]
fn cpu_count_platform() -> usize {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_void};

    extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> c_int;
    }

    let name = CString::new("hw.ncpu").unwrap();
    let mut value: u32 = 0;
    let mut len = std::mem::size_of::<u32>();
    let rc = unsafe {
        sysctlbyname(
            name.as_ptr(),
            &mut value as *mut _ as *mut c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc == 0 && value > 0 {
        value as usize
    } else {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }
}

#[cfg(target_os = "macos")]
fn cpu_usage_platform(prev: &mut Option<CpuSample>) -> f32 {
    use std::os::raw::{c_int, c_uint};

    type MachPort = c_uint;
    type KernReturn = c_int;
    type Integer = c_int;
    type Natural = c_uint;
    type HostFlavor = c_int;
    type MachMsgTypeNumber = Natural;

    const HOST_CPU_LOAD_INFO: HostFlavor = 3;
    const HOST_CPU_LOAD_INFO_COUNT: MachMsgTypeNumber = 4;
    const CPU_STATE_USER: usize = 0;
    const CPU_STATE_SYSTEM: usize = 1;
    const CPU_STATE_IDLE: usize = 2;
    const CPU_STATE_NICE: usize = 3;

    #[repr(C)]
    struct HostCpuLoadInfo {
        cpu_ticks: [Natural; 4],
    }

    extern "C" {
        fn mach_host_self() -> MachPort;
        fn host_statistics(
            host_priv: MachPort,
            flavor: HostFlavor,
            host_info_out: *mut Integer,
            host_info_out_cnt: *mut MachMsgTypeNumber,
        ) -> KernReturn;
    }

    let mut info = HostCpuLoadInfo { cpu_ticks: [0; 4] };
    let mut count = HOST_CPU_LOAD_INFO_COUNT;
    let kr = unsafe {
        host_statistics(
            unsafe { mach_host_self() },
            HOST_CPU_LOAD_INFO,
            &mut info as *mut _ as *mut Integer,
            &mut count,
        )
    };
    if kr != 0 {
        return 0.0;
    }

    let user = info.cpu_ticks[CPU_STATE_USER] as u64;
    let system = info.cpu_ticks[CPU_STATE_SYSTEM] as u64;
    let idle = info.cpu_ticks[CPU_STATE_IDLE] as u64;
    let nice = info.cpu_ticks[CPU_STATE_NICE] as u64;
    let current = CpuSample {
        total: user + system + idle + nice,
        idle,
    };

    let usage = if let Some(prev_sample) = prev {
        let total_delta = current.total.saturating_sub(prev_sample.total);
        let idle_delta = current.idle.saturating_sub(prev_sample.idle);
        if total_delta == 0 {
            0.0
        } else {
            ((total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64 * 100.0) as f32
        }
    } else {
        0.0
    };

    *prev = Some(current);
    usage.clamp(0.0, 100.0)
}

#[cfg(target_os = "macos")]
fn memory_platform() -> (u64, u64) {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_uint, c_void};

    type MachPort = c_uint;
    type KernReturn = c_int;
    type Integer = c_int;
    type Natural = c_uint;
    type HostFlavor = c_int;
    type MachMsgTypeNumber = Natural;

    const HOST_VM_INFO64: HostFlavor = 4;

    #[repr(C)]
    struct VmStatistics64 {
        free_count: Natural,
        active_count: Natural,
        inactive_count: Natural,
        wire_count: Natural,
        zero_fill_count: u64,
        reactivations: u64,
        pageins: u64,
        pageouts: u64,
        faults: u64,
        cow_faults: u64,
        lookups: u64,
        hits: u64,
        purges: u64,
        purgeable_count: Natural,
        speculative_count: Natural,
        decompressions: u64,
        compressions: u64,
        swapins: u64,
        swapouts: u64,
        compressor_page_count: Natural,
        throttled_count: Natural,
        external_page_count: Natural,
        internal_page_count: Natural,
        total_uncompressed_pages_in_compressor: u64,
    }

    extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> c_int;
        fn mach_host_self() -> MachPort;
        fn host_statistics64(
            host_priv: MachPort,
            flavor: HostFlavor,
            host_info_out: *mut Integer,
            host_info_out_cnt: *mut MachMsgTypeNumber,
        ) -> KernReturn;
    }

    let mem_name = CString::new("hw.memsize").unwrap();
    let mut total_bytes: u64 = 0;
    let mut total_len = std::mem::size_of::<u64>();
    let total_rc = unsafe {
        sysctlbyname(
            mem_name.as_ptr(),
            &mut total_bytes as *mut _ as *mut c_void,
            &mut total_len,
            std::ptr::null_mut(),
            0,
        )
    };
    if total_rc != 0 || total_bytes == 0 {
        return (0, 0);
    }

    let mut vm = VmStatistics64 {
        free_count: 0,
        active_count: 0,
        inactive_count: 0,
        wire_count: 0,
        zero_fill_count: 0,
        reactivations: 0,
        pageins: 0,
        pageouts: 0,
        faults: 0,
        cow_faults: 0,
        lookups: 0,
        hits: 0,
        purges: 0,
        purgeable_count: 0,
        speculative_count: 0,
        decompressions: 0,
        compressions: 0,
        swapins: 0,
        swapouts: 0,
        compressor_page_count: 0,
        throttled_count: 0,
        external_page_count: 0,
        internal_page_count: 0,
        total_uncompressed_pages_in_compressor: 0,
    };
    let mut count =
        (std::mem::size_of::<VmStatistics64>() / std::mem::size_of::<Integer>()) as Natural;

    let kr = unsafe {
        host_statistics64(
            unsafe { mach_host_self() },
            HOST_VM_INFO64,
            &mut vm as *mut _ as *mut Integer,
            &mut count,
        )
    };
    if kr != 0 {
        return (total_bytes / (1024 * 1024), 0);
    }

    let page_size = 4096u64;
    let free_bytes = (vm.free_count as u64 + vm.inactive_count as u64) * page_size;
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    (total_bytes / (1024 * 1024), used_bytes / (1024 * 1024))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cpu_count_platform() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cpu_usage_platform(_prev: &mut Option<CpuSample>) -> f32 {
    0.0
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn memory_platform() -> (u64, u64) {
    (0, 0)
}
