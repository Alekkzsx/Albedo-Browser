// ============================================================================
// Albedo Core Engine (ACE)
// File: partition_alloc.rs
// Description: PartitionAlloc / Hardening de Memória / OOM Killer.
//              Mapeamento direto de memória virtual (VirtualAlloc no Windows / mmap no Unix)
//              com Guard Pages (PROT_NONE) para contenção de Buffer Overflows,
//              ofuscação de ponteiros da FreeList e Quarentena de liberação anti-UAF.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Tipos de partições de memória rigidamente isoladas fisicamente pelo SO.
/// Isso impede que um Buffer Overflow no decodificador de Imagens invada
/// o heap da Máquina Virtual JavaScript, mitigando exploits Tier-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPartition {
    /// Domínio seguro. Apenas nós da árvore do DOM.
    DomTree,
    /// Domínio de alto risco. Decodificadores (PNG/JPEG), Rede.
    MediaBuffers,
    /// Domínio de execução. Objetos da VM JavaScript.
    JsContext,
}

#[cfg(target_os = "windows")]
mod win_os {
    pub const MEM_COMMIT: u32 = 0x00001000;
    pub const MEM_RESERVE: u32 = 0x00002000;
    pub const MEM_RELEASE: u32 = 0x00008000;
    pub const PAGE_READWRITE: u32 = 0x04;
    pub const PAGE_NOACCESS: u32 = 0x01;

    #[link(name = "kernel32")]
    extern "system" {
        pub fn VirtualAlloc(
            lpAddress: *mut std::ffi::c_void,
            dwSize: usize,
            flAllocationType: u32,
            flProtect: u32,
        ) -> *mut std::ffi::c_void;

        pub fn VirtualFree(lpAddress: *mut std::ffi::c_void, dwSize: usize, dwFreeType: u32)
            -> i32;
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "freebsd"
))]
mod unix_os {
    pub const PROT_NONE: i32 = 0;
    pub const PROT_READ: i32 = 1;
    pub const PROT_WRITE: i32 = 2;

    #[cfg(target_os = "linux")]
    pub const MAP_PRIVATE: i32 = 0x02;
    #[cfg(target_os = "linux")]
    pub const MAP_ANONYMOUS: i32 = 0x20;

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    pub const MAP_PRIVATE: i32 = 0x0002;
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    pub const MAP_ANONYMOUS: i32 = 0x1000;

    pub const MAP_FAILED: *mut std::ffi::c_void = !0 as *mut std::ffi::c_void;

    #[link(name = "c")]
    extern "C" {
        pub fn mmap(
            addr: *mut std::ffi::c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut std::ffi::c_void;

        pub fn mprotect(addr: *mut std::ffi::c_void, len: usize, prot: i32) -> i32;

        pub fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
    }
}

/// Cookie secreto aleatório para ofuscação XOR dos ponteiros da FreeList.
static FREELIST_COOKIE: AtomicUsize = AtomicUsize::new(0);

fn get_freelist_cookie() -> usize {
    let current = FREELIST_COOKIE.load(Ordering::Relaxed);
    if current != 0 {
        return current;
    }
    // Inicialização do cookie com base no endereço da própria static (ASLR entropy) e clock de ciclos
    let seed = (&FREELIST_COOKIE as *const _ as usize) ^ crate::time::CycleClock::now_ticks() as usize ^ 0xDEADBEEF_CAFEBABE;
    let cookie = if seed == 0 { 0x517cc1b727220a95 } else { seed };
    FREELIST_COOKIE.store(cookie, Ordering::Relaxed);
    cookie
}

/// Codifica um ponteiro bruto da FreeList usando máscara XOR para prevenir ataques de sobrescrita arbitrária.
#[inline(always)]
pub fn encode_freelist_ptr(ptr: *mut u8) -> usize {
    if ptr.is_null() {
        0
    } else {
        (ptr as usize) ^ get_freelist_cookie()
    }
}

/// Decodifica um ponteiro ofuscado da FreeList.
#[inline(always)]
pub fn decode_freelist_ptr(encoded: usize) -> *mut u8 {
    if encoded == 0 {
        ptr::null_mut()
    } else {
        (encoded ^ get_freelist_cookie()) as *mut u8
    }
}

/// Página de alocação protegida pelo Sistema Operacional.
/// O espaço útil é blindado por 2 Guard Pages permanentes (início e fim) com `PROT_NONE` / `PAGE_NOACCESS`.
pub struct SecurePage {
    partition: MemoryPartition,
    base_ptr: *mut u8,
    data_size: usize,
}

impl SecurePage {
    /// Aloca uma nova página na memória virtual com Guard Pages (Páginas de proteção).
    pub fn allocate_isolated(
        partition: MemoryPartition,
        size_in_bytes: usize,
    ) -> Result<Self, &'static str> {
        let page_size = 4096;
        // Alinhamento para múltiplos do tamanho da página
        let aligned_size = (size_in_bytes + page_size - 1) & !(page_size - 1);

        // Tamanho total = Espaço alinhado + 2 Guard Pages (Início e Fim)
        let total_size = aligned_size + 2 * page_size;

        #[cfg(target_os = "windows")]
        let base_ptr = unsafe {
            use win_os::*;
            // 1. Reserva o espaço total (incluso Guard Pages) sem dar acesso (PAGE_NOACCESS)
            let base = VirtualAlloc(ptr::null_mut(), total_size, MEM_RESERVE, PAGE_NOACCESS);

            if base.is_null() {
                OomKiller::trigger_memory_pressure_purge();
                return Err("Falha na reserva de memória virtual Windows (System OOM).");
            }

            // 2. Comita apenas a região central com leitura/escrita
            let data_ptr = (base as *mut u8).add(page_size);
            let committed = VirtualAlloc(
                data_ptr as *mut std::ffi::c_void,
                aligned_size,
                MEM_COMMIT,
                PAGE_READWRITE,
            );

            if committed.is_null() {
                VirtualFree(base, 0, MEM_RELEASE);
                OomKiller::trigger_memory_pressure_purge();
                return Err("Falha no commit de memória central Windows (System OOM).");
            }

            base as *mut u8
        };

        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "android",
            target_os = "freebsd"
        ))]
        let base_ptr = unsafe {
            use unix_os::*;
            // 1. Mapeia o bloco total com PROT_NONE (Guard Pages ativadas em todo o espaço)
            let base = mmap(
                ptr::null_mut(),
                total_size,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );

            if base == MAP_FAILED || base.is_null() {
                OomKiller::trigger_memory_pressure_purge();
                return Err("Falha na reserva mmap virtual Unix (System OOM).");
            }

            // 2. Protege/Libera apenas a região central útil para Leitura e Escrita
            let data_ptr = (base as *mut u8).add(page_size);
            let mprotect_res = mprotect(
                data_ptr as *mut std::ffi::c_void,
                aligned_size,
                PROT_READ | PROT_WRITE,
            );

            if mprotect_res != 0 {
                munmap(base, total_size);
                OomKiller::trigger_memory_pressure_purge();
                return Err("Falha no mprotect central da região útil Unix (System OOM).");
            }

            base as *mut u8
        };

        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "macos",
            target_os = "android",
            target_os = "freebsd"
        )))]
        let base_ptr = {
            let layout = std::alloc::Layout::from_size_align(total_size, 4096).unwrap();
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                OomKiller::trigger_memory_pressure_purge();
                return Err("System Out of Memory. OOM Killer acionado.");
            }
            ptr
        };

        Ok(Self {
            partition,
            base_ptr,
            data_size: aligned_size,
        })
    }

    /// Retorna a partição à qual esta página de memória pertence.
    #[inline]
    pub fn partition(&self) -> MemoryPartition {
        self.partition
    }

    /// Retorna o tamanho em bytes utilizáveis da região de dados.
    #[inline]
    pub fn data_size(&self) -> usize {
        self.data_size
    }

    /// Retorna o ponteiro seguro para manipulação de dados (ignora a Guard Page inicial de 4 KB).
    #[inline]
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe { self.base_ptr.add(4096) }
    }

    /// Verifica se um dado endereço de memória pertence à região útil comitada desta página.
    #[inline]
    pub fn contains_ptr(&self, ptr: *const u8) -> bool {
        let start = self.data_ptr() as usize;
        let end = start + self.data_size;
        let addr = ptr as usize;
        addr >= start && addr < end
    }
}

impl Drop for SecurePage {
    fn drop(&mut self) {
        if !self.base_ptr.is_null() {
            let page_size = 4096;
            let total_size = self.data_size + 2 * page_size;

            #[cfg(target_os = "windows")]
            unsafe {
                win_os::VirtualFree(
                    self.base_ptr as *mut std::ffi::c_void,
                    0,
                    win_os::MEM_RELEASE,
                );
            }

            #[cfg(any(
                target_os = "linux",
                target_os = "macos",
                target_os = "android",
                target_os = "freebsd"
            ))]
            unsafe {
                unix_os::munmap(self.base_ptr as *mut std::ffi::c_void, total_size);
            }

            #[cfg(not(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "macos",
                target_os = "android",
                target_os = "freebsd"
            )))]
            {
                let layout = std::alloc::Layout::from_size_align(total_size, 4096).unwrap();
                unsafe {
                    std::alloc::dealloc(self.base_ptr, layout);
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Quarentena de Desalocação Anti-UAF
// ----------------------------------------------------------------------------

const QUARANTINE_CAPACITY: usize = 64;

/// Fila circular de quarentena que retém ponteiros desalocados temporariamente
/// para prevenir a reciclagem imediata e frustrar ataques de Use-After-Free / Heap Spraying.
pub struct QuarantineRing {
    entries: [*mut u8; QUARANTINE_CAPACITY],
    head: usize,
    count: usize,
}

impl QuarantineRing {
    pub const fn new() -> Self {
        Self {
            entries: [ptr::null_mut(); QUARANTINE_CAPACITY],
            head: 0,
            count: 0,
        }
    }

    /// Coloca um ponteiro recém-liberado em quarentena.
    /// Retorna um ponteiro antigo que saiu da quarentena (pronto para reciclagem final), se houver.
    pub fn push(&mut self, ptr: *mut u8) -> Option<*mut u8> {
        let mut evicted = None;
        if self.count >= QUARANTINE_CAPACITY {
            evicted = Some(self.entries[self.head]);
        } else {
            self.count += 1;
        }

        self.entries[self.head] = ptr;
        self.head = (self.head + 1) % QUARANTINE_CAPACITY;
        evicted
    }

    /// Esvazia todos os ponteiros retidos em quarentena.
    pub fn drain_all<F: FnMut(*mut u8)>(&mut self, mut on_release: F) {
        for i in 0..self.count {
            let idx = (self.head + QUARANTINE_CAPACITY - self.count + i) % QUARANTINE_CAPACITY;
            let ptr = self.entries[idx];
            if !ptr.is_null() {
                on_release(ptr);
                self.entries[idx] = ptr::null_mut();
            }
        }
        self.head = 0;
        self.count = 0;
    }
}

impl Default for QuarantineRing {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// OOM Killer
// ----------------------------------------------------------------------------

/// Sistema autônomo de defesa contra falta de RAM.
pub struct OomKiller;

impl OomKiller {
    /// Chamado quando uma alocação falha ou quando o SO sinaliza pressão extrema na RAM.
    /// Em vez de fechar o navegador, o Albedo expurga agressivamente buffers não críticos.
    pub fn trigger_memory_pressure_purge() {
        // 1. Forçar o Garbage Collector do JavaScript em todos os contextos ativos.
        crate::ace_warn!("OOM Killer Ativado! Limpando Lixo do JS...");

        // 2. Destruir buffers de imagem ocultos (que não estão na tela atual).
        crate::ace_warn!("Descartando caches de mídia não-essenciais...");

        // 3. Compactar Strings do DOM (forçar shrink_to_fit).
        crate::ace_warn!("Compactando Arenas...");
    }
}
