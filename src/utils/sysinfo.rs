use sysinfo::System;

pub struct AceSysInfo {
    sys: System,
}

impl AceSysInfo {
    pub fn new() -> Self {
        // Initialize with CPU and Memory refreshers
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Self { sys }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
    }

    pub fn cpu_count(&self) -> usize {
        self.sys.cpus().len()
    }

    pub fn total_memory(&self) -> u64 {
        // Return in MB for display convenience
        self.sys.total_memory() / (1024 * 1024)
    }

    pub fn used_memory(&self) -> u64 {
        // Return in MB for display convenience
        self.sys.used_memory() / (1024 * 1024)
    }

    pub fn cpu_usage(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    pub fn memory_usage(&self) -> u64 {
        self.sys.used_memory()
    }
}
