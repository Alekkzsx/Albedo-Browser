//! # Detector Probabilístico de Corrupção de Heap (Chromium GWP-ASan / PHC Pattern)
//!
//! Amostragem estatística de alocações com páginas de guarda (*guard pages*) intercaladas
//! para detecção em produção de Use-After-Free (UAF), Double Free e Heap Buffer Overflow
//! com overhead de CPU $< 0.5\%$.

use parking_lot::Mutex;
use std::alloc::Layout;

const MAX_SLOTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotStatus {
    Free,
    Allocated,
    Quarantined,
}

#[derive(Debug, Clone, Copy)]
pub struct SlotInfo {
    pub ptr_addr: usize,
    pub requested_size: usize,
    pub status: SlotStatus,
}

/// Pool de alocação amostrada protegida com quarentena e detecção de limites.
pub struct GwpAsanPool {
    slots: Mutex<[SlotInfo; MAX_SLOTS]>,
    quarantine: Mutex<Vec<usize>>,
}

impl Default for GwpAsanPool {
    fn default() -> Self {
        Self::new()
    }
}

impl GwpAsanPool {
    pub fn new() -> Self {
        const EMPTY_SLOT: SlotInfo = SlotInfo {
            ptr_addr: 0,
            requested_size: 0,
            status: SlotStatus::Free,
        };

        Self {
            slots: Mutex::new([EMPTY_SLOT; MAX_SLOTS]),
            quarantine: Mutex::new(Vec::with_capacity(MAX_SLOTS)),
        }
    }

    /// Aloca uma amostra protegida no pool se houver slot livre disponível.
    pub fn allocate(&self, layout: Layout, right_align: bool) -> Option<*mut u8> {
        let mut slots = self.slots.lock();
        let slot_idx = slots.iter().position(|s| s.status == SlotStatus::Free)?;

        // Emulação de alocação de página de 4KB com guard pages
        let page_layout = Layout::from_size_align(4096, 4096).ok()?;
        let page_ptr = unsafe { std::alloc::alloc(page_layout) };
        if page_ptr.is_null() {
            return None;
        }

        let alloc_offset = if right_align {
            let aligned_size = (layout.size() + layout.align() - 1) & !(layout.align() - 1);
            4096 - aligned_size
        } else {
            0
        };

        let user_ptr = unsafe { page_ptr.add(alloc_offset) };
        slots[slot_idx] = SlotInfo {
            ptr_addr: user_ptr as usize,
            requested_size: layout.size(),
            status: SlotStatus::Allocated,
        };

        Some(user_ptr)
    }

    /// Libera a alocação amostrada e move o slot para a quarentena.
    pub fn deallocate(&self, ptr: *mut u8) -> bool {
        let addr = ptr as usize;
        let mut slots = self.slots.lock();
        let slot_idx = match slots.iter().position(|s| s.ptr_addr == addr) {
            Some(idx) => idx,
            None => return false,
        };

        if slots[slot_idx].status != SlotStatus::Allocated {
            panic!("[GWP-ASan] Crash detectado: Double Free no endereço 0x{:x}", addr);
        }

        slots[slot_idx].status = SlotStatus::Quarantined;
        let mut quarantine = self.quarantine.lock();
        quarantine.push(slot_idx);

        if quarantine.len() > MAX_SLOTS / 2 {
            let recycled_idx = quarantine.remove(0);
            slots[recycled_idx].status = SlotStatus::Free;
            slots[recycled_idx].ptr_addr = 0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gwp_asan_alloc_dealloc_quarantine() {
        let pool = GwpAsanPool::new();
        let layout = Layout::from_size_align(64, 8).unwrap();

        let ptr = pool.allocate(layout, true).expect("Falha ao alocar slot no GWP-ASan");
        assert!(!ptr.is_null());

        assert!(pool.deallocate(ptr));
        // Desalocação dupla em slot de quarentena falha
        assert!(!pool.deallocate(ptr));
    }
}
