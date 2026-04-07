//! Arena Allocator para Nodes DOM
//! 
//! Este módulo implementa um arena allocator otimizado para alocação
//! de nodes do DOM, proporcionando:
//! - Alocações ultra-rápidas (bump pointer)
//! - Melhor cache locality (dados contíguos)
//! - Dealocação em O(1) (clear da arena inteira)
//! - Redução de fragmentação de memória


use std::ptr::NonNull;

/// Tamanho padrão de cada chunk na arena (64KB)
const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Alinhamento para alocações (otimizado para cache lines)
const ALIGNMENT: usize = 8;

/// ID único para referenciar nodes na arena
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    /// ID nulo para representar ausência de node
    pub const NULL: NodeId = NodeId(usize::MAX);
    
    #[inline]
    pub fn is_null(self) -> bool {
        self == Self::NULL
    }
}

/// Chunk individual de memória na arena
struct Chunk {
    /// Ponteiro para o início dos dados
    data: NonNull<u8>,
    /// Capacidade total em bytes
    capacity: usize,
    /// Quantidade já alocada
    allocated: usize,
}

impl Chunk {
    fn new(capacity: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(capacity, ALIGNMENT)
            .expect("Invalid layout");
        
        unsafe {
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            
            Self {
                data: NonNull::new_unchecked(ptr),
                capacity,
                allocated: 0,
            }
        }
    }

    #[inline]
    fn alloc(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let current = unsafe { self.data.as_ptr().add(self.allocated) };
        
        // Calcula alinhamento necessário
        let aligned = ((current as usize) + (align - 1)) & !(align - 1);
        let padding = aligned - current as usize;
        
        if self.allocated + padding + size > self.capacity {
            return None;
        }
        
        self.allocated += padding + size;
        Some(unsafe { NonNull::new_unchecked(aligned as *mut u8) })
    }

    fn clear(&mut self) {
        self.allocated = 0;
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        unsafe {
            let layout = std::alloc::Layout::from_size_align(self.capacity, ALIGNMENT)
                .expect("Invalid layout");
            std::alloc::dealloc(self.data.as_ptr(), layout);
        }
    }
}

/// Arena allocator para nodes DOM
/// 
/// # Exemplo de uso:
/// ```
/// use ace::html::arena::NodeArena;
/// 
/// let mut arena = NodeArena::new();
/// let id1 = arena.alloc(String::from("div"));
/// let id2 = arena.alloc(String::from("span"));
/// 
/// unsafe {
///     let div = arena.get::<String>(id1);
///     assert_eq!(div, "div");
/// }
/// ```
pub struct NodeArena {
    /// Chunks de memória (pelo menos um sempre existe)
    chunks: Vec<Chunk>,
    /// Índice do chunk atual
    current_chunk: usize,
    /// Contador de alocações (para debugging/stats)
    allocation_count: usize,
}

impl NodeArena {
    /// Cria uma nova arena com capacidade inicial padrão
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHUNK_SIZE)
    }

    /// Cria uma nova arena com capacidade inicial específica
    pub fn with_capacity(initial_capacity: usize) -> Self {
        let chunk = Chunk::new(initial_capacity);
        Self {
            chunks: vec![chunk],
            current_chunk: 0,
            allocation_count: 0,
        }
    }

    /// Aloca espaço para um valor do tipo T na arena
    /// 
    /// Retorna um NodeId que pode ser usado para recuperar o valor
    #[inline]
    pub fn alloc<T>(&mut self, value: T) -> NodeId {
        self.allocation_count += 1;
        
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        
        // Tenta alocar no chunk atual
        if let Some(ptr) = self.try_alloc_in_current(size, align) {
            unsafe {
                ptr.as_ptr().cast::<T>().write(value);
                return NodeId(ptr.as_ptr() as usize);
            }
        }
        
        // Precisa de novo chunk
        self.allocate_new_chunk(size, align);
        
        // Tenta novamente
        if let Some(ptr) = self.try_alloc_in_current(size, align) {
            unsafe {
                ptr.as_ptr().cast::<T>().write(value);
                return NodeId(ptr.as_ptr() as usize);
            }
        }
        
        panic!("Failed to allocate {} bytes in arena", size);
    }

    /// Recupera uma referência para um valor previamente alocado
    /// 
    /// # Safety
    /// - O NodeId deve ser válido e retornado por `alloc`
    /// - O valor não pode ter sido movido ou dealocado
    #[inline]
    pub unsafe fn get<T>(&self, id: NodeId) -> &T {
        debug_assert!(!id.is_null(), "Cannot get null NodeId");
        &*(id.0 as *const T)
    }

    /// Recupera uma referência mutável para um valor
    #[inline]
    pub unsafe fn get_mut<T>(&mut self, id: NodeId) -> &mut T {
        debug_assert!(!id.is_null(), "Cannot get_mut null NodeId");
        &mut *(id.0 as *mut T)
    }

    /// Limpa toda a arena, permitindo reuso da memória
    /// 
    /// ⚠️ **Atenção:** Todos os NodeIds anteriores se tornam inválidos!
    pub fn clear(&mut self) {
        for chunk in self.chunks.iter_mut() {
            chunk.clear();
        }
        self.current_chunk = 0;
        self.allocation_count = 0;
    }

    /// Retorna estatísticas da arena
    pub fn stats(&self) -> ArenaStats {
        let total_capacity: usize = self.chunks.iter().map(|c| c.capacity).sum();
        let total_allocated: usize = self.chunks.iter().map(|c| c.allocated).sum();
        
        ArenaStats {
            chunk_count: self.chunks.len(),
            total_capacity,
            total_allocated,
            allocation_count: self.allocation_count,
            utilization: if total_capacity > 0 {
                total_allocated as f32 / total_capacity as f32
            } else {
                0.0
            },
        }
    }

    /// Tenta alocar no chunk atual
    #[inline]
    fn try_alloc_in_current(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        if self.current_chunk < self.chunks.len() {
            self.chunks[self.current_chunk].alloc(size, align)
        } else {
            None
        }
    }

    /// Aloca um novo chunk grande o suficiente para o valor
    fn allocate_new_chunk(&mut self, min_size: usize, _align: usize) {
        // Novo chunk com pelo menos o dobro do tamanho necessário ou DEFAULT_CHUNK_SIZE
        let new_capacity = std::cmp::max(DEFAULT_CHUNK_SIZE, min_size * 2);
        let new_chunk = Chunk::new(new_capacity);
        
        self.chunks.push(new_chunk);
        self.current_chunk = self.chunks.len() - 1;
    }
}

impl Default for NodeArena {
    fn default() -> Self {
        Self::new()
    }
}

/// Estatísticas da arena para profiling
#[derive(Clone)]
pub struct ArenaStats {
    pub chunk_count: usize,
    pub total_capacity: usize,
    pub total_allocated: usize,
    pub allocation_count: usize,
    pub utilization: f32,
}

impl std::fmt::Debug for ArenaStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArenaStats")
            .field("chunk_count", &self.chunk_count)
            .field("total_capacity", &format_args!("{} KB", self.total_capacity / 1024))
            .field("total_allocated", &format_args!("{} KB", self.total_allocated / 1024))
            .field("allocation_count", &self.allocation_count)
            .field("utilization", &format_args!("{:.1}%", self.utilization * 100.0))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut arena = NodeArena::new();
        let id = arena.alloc(42u32);
        
        unsafe {
            assert_eq!(*arena.get::<u32>(id), 42);
        }
    }

    #[test]
    fn test_multiple_allocations() {
        let mut arena = NodeArena::new();
        
        let id1 = arena.alloc(1u32);
        let id2 = arena.alloc(2u32);
        let id3 = arena.alloc(3u32);
        
        unsafe {
            assert_eq!(*arena.get::<u32>(id1), 1);
            assert_eq!(*arena.get::<u32>(id2), 2);
            assert_eq!(*arena.get::<u32>(id3), 3);
        }
    }

    #[test]
    fn test_clear_and_reuse() {
        let mut arena = NodeArena::new();
        
        let _id1 = arena.alloc(100u32);
        let stats_before = arena.stats();
        assert!(stats_before.total_allocated > 0);
        
        arena.clear();
        
        let stats_after = arena.stats();
        assert_eq!(stats_after.total_allocated, 0);
        assert_eq!(stats_after.allocation_count, 0);
    }

    #[test]
    fn test_large_allocation() {
        let mut arena = NodeArena::new();
        
        // Aloca um array grande
        let data = vec![1u32, 2, 3, 4, 5];
        let id = arena.alloc(data);
        
        unsafe {
            let retrieved = arena.get::<Vec<u32>>(id);
            assert_eq!(retrieved.len(), 5);
            assert_eq!(retrieved[0], 1);
        }
    }

    #[test]
    fn test_chunk_allocation_64kb() {
        let arena = NodeArena::new();
        
        // Verifica que o chunk inicial tem 64KB
        let stats = arena.stats();
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.total_capacity, DEFAULT_CHUNK_SIZE);
        assert_eq!(DEFAULT_CHUNK_SIZE, 64 * 1024);
    }

    #[test]
    fn test_multiple_chunks() {
        let mut arena = NodeArena::new();
        
        // Aloca muitos valores pequenos para preencher o primeiro chunk
        let mut ids = Vec::new();
        for i in 0..10000 {
            ids.push(arena.alloc(i as u64));
        }
        
        let stats = arena.stats();
        // Deve ter alocado múltiplos chunks
        assert!(stats.chunk_count >= 2, "Expected at least 2 chunks, got {}", stats.chunk_count);
        
        // Verifica que todos os valores estão corretos
        for (i, id) in ids.iter().enumerate() {
            unsafe {
                assert_eq!(*arena.get::<u64>(*id), i as u64);
            }
        }
    }

    #[test]
    fn test_alignment() {
        let mut arena = NodeArena::new();
        
        // Aloca valores com diferentes alinhamentos
        let id1 = arena.alloc(1u8);  // align 1
        let id2 = arena.alloc(2u16); // align 2
        let id3 = arena.alloc(3u32); // align 4
        let id4 = arena.alloc(4u64); // align 8
        
        unsafe {
            assert_eq!(*arena.get::<u8>(id1), 1);
            assert_eq!(*arena.get::<u16>(id2), 2);
            assert_eq!(*arena.get::<u32>(id3), 3);
            assert_eq!(*arena.get::<u64>(id4), 4);
            
            // Verifica alinhamento correto
            let ptr4 = id4.0 as *const u64;
            assert_eq!(ptr4 as usize % 8, 0, "u64 should be 8-byte aligned");
        }
    }

    #[test]
    fn test_new_chunk_when_full() {
        let mut arena = NodeArena::with_capacity(128); // Chunk pequeno para teste
        
        let stats_before = arena.stats();
        assert_eq!(stats_before.chunk_count, 1);
        
        // Aloca valores até preencher o chunk
        for _ in 0..20 {
            arena.alloc([0u64; 2]); // 16 bytes cada
        }
        
        let stats_after = arena.stats();
        // Deve ter alocado novo chunk
        assert!(stats_after.chunk_count > 1, "Expected multiple chunks");
    }

    #[test]
    fn test_stats_tracking() {
        let mut arena = NodeArena::new();
        
        let id1 = arena.alloc(100u32);
        let id2 = arena.alloc(200u32);
        let id3 = arena.alloc(300u32);
        
        let stats = arena.stats();
        assert_eq!(stats.allocation_count, 3);
        assert!(stats.total_allocated >= 12); // At least 3 * 4 bytes
        assert!(stats.utilization > 0.0);
        assert!(stats.utilization <= 1.0);
        
        unsafe {
            assert_eq!(*arena.get::<u32>(id1), 100);
            assert_eq!(*arena.get::<u32>(id2), 200);
            assert_eq!(*arena.get::<u32>(id3), 300);
        }
    }
}
