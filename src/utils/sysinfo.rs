use sysinfo::System;

pub struct AceSysInfo {
    sys: System,
}

impl AceSysInfo {
    /// TODO: add docs
    pub fn new() -> Self {
        // Initialize with CPU and Memory refreshers
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Self { sys }
    }

    /// TODO: add docs
    pub fn refresh(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
    }

    /// TODO: add docs
    pub fn cpu_count(&self) -> usize {
        self.sys.cpus().len()
    }

    /// TODO: add docs
    pub fn total_memory(&self) -> u64 {
        // Return in MB for display convenience
        self.sys.total_memory() / (1024 * 1024)
    }

    /// TODO: add docs
    pub fn used_memory(&self) -> u64 {
        // Return in MB for display convenience
        self.sys.used_memory() / (1024 * 1024)
    }

    /// TODO: add docs
    pub fn cpu_usage(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    /// TODO: add docs
    pub fn memory_usage(&self) -> u64 {
        self.sys.used_memory()
    }
}
