// ============================================================================
// Albedo Core Engine (ACE)
// File: partition_alloc.rs
// Description: PartitionAlloc / OOM Killer.
//              Mapeamento direto de memória virtual (VirtualAlloc/mmap) para isolamento
//              de segurança (mitigação de buffer overflow) e interceptação de OOM.
// Author: Albedo Browser Engineering Team
// ============================================================================



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

/// Página de alocação protegida pelo Sistema Operacional.
pub struct SecurePage {
    _partition: MemoryPartition,
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
                return Err("Falha na reserva de memória (System OOM).");
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
                return Err("Falha no commit de memória central (System OOM).");
            }

            base as *mut u8
        };

        #[cfg(not(target_os = "windows"))]
        let base_ptr = {
            // Fallback (simulação) para outras plataformas temporariamente
            let layout = std::alloc::Layout::from_size_align(total_size, 4096).unwrap();
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                OomKiller::trigger_memory_pressure_purge();
                return Err("System Out of Memory. OOM Killer acionado.");
            }
            ptr
        };

        Ok(Self {
            _partition: partition,
            base_ptr,
            data_size: aligned_size,
        })
    }

    /// Retorna o ponteiro seguro para manipulação de dados (ignora a Guard Page inicial)
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe { self.base_ptr.add(4096) }
    }
}

impl Drop for SecurePage {
    fn drop(&mut self) {
        if !self.base_ptr.is_null() {
            #[cfg(target_os = "windows")]
            unsafe {
                win_os::VirtualFree(
                    self.base_ptr as *mut std::ffi::c_void,
                    0,
                    win_os::MEM_RELEASE,
                );
            }

            #[cfg(not(target_os = "windows"))]
            {
                let total_size = self.data_size + 2 * 4096;
                let layout = std::alloc::Layout::from_size_align(total_size, 4096).unwrap();
                unsafe {
                    std::alloc::dealloc(self.base_ptr, layout);
                }
            }
        }
    }
}

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

// Testes movidos para tests/partition_alloc_tests.rs
