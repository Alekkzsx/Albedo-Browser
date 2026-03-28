//! # Alocador de MemÃ³ria ExecutÃ¡vel (W^X)
//!
//! Gerencia a alocaÃ§Ã£o de memÃ³ria virtual protegida para execuÃ§Ã£o de cÃ³digo JIT.
//! Implementa a polÃ­tica W^X (Write XOR Execute) para seguranÃ§a.

use std::fmt;
use std::ptr::{self, NonNull};

#[derive(Debug)]
pub enum MemoryError {
    AllocationFailed(String),
    ProtectionFailed(String),
    BufferOverflow,
    NotWritable,
    BudgetExceeded,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::AllocationFailed(msg) => {
                write!(f, "Falha na alocação de memória: {}", msg)
            }
            MemoryError::ProtectionFailed(msg) => {
                write!(f, "Falha ao mudar proteção de memória: {}", msg)
            }
            MemoryError::BufferOverflow => {
                write!(
                    f,
                    "A região de memória é muito pequena para a escrita solicitada"
                )
            }
            MemoryError::NotWritable => {
                write!(f, "Região de memória não é mais gravável (está em modo RX)")
            }
            MemoryError::BudgetExceeded => write!(f, "Budget de memória JIT excedido"),
        }
    }
}

impl std::error::Error for MemoryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionState {
    ReadWrite,
    ReadExecute,
}

/// RegiÃ£o de memÃ³ria virtual alocada do sistema operacional.
pub struct CodeRegion {
    ptr: NonNull<u8>,
    size: usize,
    used: usize,
    state: ProtectionState,
}

impl CodeRegion {
    /// Aloca uma nova regiÃ£o de memÃ³ria virtual com o tamanho especificado (page-aligned).
    pub fn allocate(size: usize) -> Result<Self, MemoryError> {
        let page_size = get_page_size();
        let aligned_size = (size + page_size - 1) & !(page_size - 1);

        let ptr = unsafe { platform_alloc(aligned_size)? };

        Ok(Self {
            ptr: NonNull::new(ptr)
                .ok_or_else(|| MemoryError::AllocationFailed("Nulo retornado".into()))?,
            size: aligned_size,
            used: 0,
            state: ProtectionState::ReadWrite,
        })
    }

    /// Escreve bytes de cÃ³digo na regiÃ£o. SÃ³ funciona se o estado for ReadWrite.
    pub fn write(&mut self, code: &[u8]) -> Result<usize, MemoryError> {
        if self.state != ProtectionState::ReadWrite {
            return Err(MemoryError::NotWritable);
        }

        if self.used + code.len() > self.size {
            return Err(MemoryError::BufferOverflow);
        }

        unsafe {
            ptr::copy_nonoverlapping(code.as_ptr(), self.ptr.as_ptr().add(self.used), code.len());
        }

        let offset = self.used;
        self.used += code.len();
        Ok(offset)
    }

    /// Muda a proteÃ§Ã£o da memÃ³ria para ReadExecute.
    pub fn make_executable(&mut self) -> Result<(), MemoryError> {
        if self.state == ProtectionState::ReadExecute {
            return Ok(());
        }

        unsafe {
            platform_protect_rx(self.ptr.as_ptr(), self.size)?;
        }
        self.state = ProtectionState::ReadExecute;
        Ok(())
    }

    /// Muda a proteÃ§Ã£o da memÃ³ria para ReadWrite.
    pub fn make_writable(&mut self) -> Result<(), MemoryError> {
        if self.state == ProtectionState::ReadWrite {
            return Ok(());
        }

        unsafe {
            platform_protect_rw(self.ptr.as_ptr(), self.size)?;
        }
        self.state = ProtectionState::ReadWrite;
        Ok(())
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn state(&self) -> ProtectionState {
        self.state
    }
}

impl Drop for CodeRegion {
    fn drop(&mut self) {
        unsafe {
            platform_free(self.ptr.as_ptr(), self.size);
        }
    }
}

// SAFETY: Ponteiros para memÃ³ria JIT sÃ£o Thread-safe se controlados por Mutex/RwLock
unsafe impl Send for CodeRegion {}
unsafe impl Sync for CodeRegion {}

// ---------------------------------------------------------
// CodePool
// ---------------------------------------------------------

#[derive(Default, Debug, Clone)]
pub struct CodePoolStats {
    pub total_allocated: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub allocations_count: u64,
}

pub struct CodePool {
    regions: Vec<CodeRegion>,
    budget: usize,
    stats: CodePoolStats,
}

impl CodePool {
    pub fn new(budget: usize) -> Self {
        Self {
            regions: Vec::new(),
            budget,
            stats: CodePoolStats::default(),
        }
    }

    pub fn allocate(&mut self, size: usize) -> Result<&mut CodeRegion, MemoryError> {
        if self.stats.current_usage + size > self.budget {
            return Err(MemoryError::BudgetExceeded);
        }

        let region = CodeRegion::allocate(size)?;
        let real_size = region.size();

        self.stats.total_allocated += real_size;
        self.stats.current_usage += real_size;
        self.stats.allocations_count += 1;

        if self.stats.current_usage > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.current_usage;
        }

        self.regions.push(region);
        Ok(self.regions.last_mut().unwrap())
    }

    pub fn stats(&self) -> &CodePoolStats {
        &self.stats
    }
}

// ---------------------------------------------------------
// ImplementaÃ§Ãµes de Plataforma (Windows)
// ---------------------------------------------------------

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};

#[cfg(target_os = "windows")]
fn get_page_size() -> usize {
    unsafe {
        let mut sys_info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut sys_info);
        sys_info.dwPageSize as usize
    }
}

#[cfg(target_os = "windows")]
unsafe fn platform_alloc(size: usize) -> Result<*mut u8, MemoryError> {
    let ptr = VirtualAlloc(ptr::null(), size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if ptr.is_null() {
        return Err(MemoryError::AllocationFailed(format!(
            "VirtualAlloc falhou com erro: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(ptr as *mut u8)
}

#[cfg(target_os = "windows")]
unsafe fn platform_protect_rx(ptr: *mut u8, size: usize) -> Result<(), MemoryError> {
    let mut old_protect = 0;
    if VirtualProtect(ptr as _, size, PAGE_EXECUTE_READ, &mut old_protect) == 0 {
        return Err(MemoryError::ProtectionFailed(format!(
            "VirtualProtect -> RX falhou: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn platform_protect_rw(ptr: *mut u8, size: usize) -> Result<(), MemoryError> {
    let mut old_protect = 0;
    if VirtualProtect(ptr as _, size, PAGE_READWRITE, &mut old_protect) == 0 {
        return Err(MemoryError::ProtectionFailed(format!(
            "VirtualProtect -> RW falhou: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn platform_free(ptr: *mut u8, _size: usize) {
    VirtualFree(ptr as _, 0, MEM_RELEASE);
}

// ---------------------------------------------------------
// ImplementaÃ§Ãµes de Plataforma (Unix)
// ---------------------------------------------------------

#[cfg(unix)]
use libc::*;

#[cfg(unix)]
fn get_page_size() -> usize {
    unsafe { sysconf(_SC_PAGESIZE) as usize }
}

#[cfg(unix)]
unsafe fn platform_alloc(size: usize) -> Result<*mut u8, MemoryError> {
    let ptr = mmap(
        ptr::null_mut(),
        size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if ptr == MAP_FAILED {
        return Err(MemoryError::AllocationFailed(format!(
            "mmap falhou: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(ptr as *mut u8)
}

#[cfg(unix)]
unsafe fn platform_protect_rx(ptr: *mut u8, size: usize) -> Result<(), MemoryError> {
    if mprotect(ptr as _, size, PROT_READ | PROT_EXEC) != 0 {
        return Err(MemoryError::ProtectionFailed(format!(
            "mprotect -> RX falhou: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(unix)]
unsafe fn platform_protect_rw(ptr: *mut u8, size: usize) -> Result<(), MemoryError> {
    if mprotect(ptr as _, size, PROT_READ | PROT_WRITE) != 0 {
        return Err(MemoryError::ProtectionFailed(format!(
            "mprotect -> RW falhou: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(unix)]
unsafe fn platform_free(ptr: *mut u8, size: usize) {
    munmap(ptr as _, size);
}

// ---------------------------------------------------------
// Testes
// ---------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_free() {
        let region = CodeRegion::allocate(4096).unwrap();
        assert!(!region.as_ptr().is_null());
        assert!(region.size() >= 4096);
        assert_eq!(region.state(), ProtectionState::ReadWrite);
    }

    #[test]
    fn test_write_and_execute_simple() {
        let mut region = CodeRegion::allocate(4096).unwrap();

        // CÃ³digo para retornar 42 (x86_64: mov eax, 42; ret)
        #[cfg(target_arch = "x86_64")]
        let code = [0xB8, 0x2A, 0x00, 0x00, 0x00, 0xC3];

        // CÃ³digo para retornar 42 (AArch64: mov w0, #42; ret)
        #[cfg(target_arch = "aarch64")]
        let code = [0x40, 0x05, 0x80, 0x52, 0xC0, 0x03, 0x5F, 0xD6];

        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            region.write(&code).unwrap();
            region.make_executable().unwrap();

            let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(region.as_ptr()) };
            assert_eq!(func(), 42);
        }
    }

    #[test]
    fn test_budget_enforcement() {
        let mut pool = CodePool::new(8192);
        pool.allocate(4096).unwrap();
        pool.allocate(4096).unwrap();

        let result = pool.allocate(1);
        assert!(matches!(result, Err(MemoryError::BudgetExceeded)));
    }

    #[test]
    fn test_wx_violation() {
        let mut region = CodeRegion::allocate(4096).unwrap();
        region.make_executable().unwrap();

        let result = region.write(&[0x90]); // NOP
        assert!(matches!(result, Err(MemoryError::NotWritable)));
    }
}
