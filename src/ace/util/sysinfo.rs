pub struct AceSysInfo;

impl AceSysInfo {
    pub fn new() -> Self { Self }
    pub fn refresh(&mut self) {}
    pub fn cpu_count(&self) -> usize { 1 }
    pub fn total_memory(&self) -> u64 { 1024 * 1024 * 1024 }
    pub fn used_memory(&self) -> u64 { 0 }
    pub fn cpu_usage(&self) -> f32 { 0.0 }
    pub fn memory_usage(&self) -> u64 { 0 }
}
