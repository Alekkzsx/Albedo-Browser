//! Arena Allocator para Nodes DOM
//! 
//! Este módulo implementa um arena allocator otimizado para alocação
//! de nodes do DOM, proporcionando:
//! - Alocações ultra-rápidas (bump pointer)
//! - Melhor cache locality (dados contíguos)
//! - Dealocação em O(1) (clear da arena inteira)
//! - Redução de fragmentação de memória

use std::cell::Cell;
use std::ptr::NonNull;

/// Wrapper genérico para dados de node DOM com estrutura de árvore
/// Usa dados opacos (u8) para evitar dependência circular
#[derive(Clone, Debug)]
pub struct ArenaNode {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    // Os dados reais são armazenados inline na arena após este header
}

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
    allocated: Cell<usize>,
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
                allocated: Cell::new(0),
            }
        }
    }

    #[inline]
    fn remaining(&self) -> usize {
        self.capacity - self.allocated.get()
    }

    #[inline]
    fn alloc(&self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let current = self.data.as_ptr().add(self.allocated.get());
        
        // Calcula alinhamento necessário
        let aligned = ((current as usize) + (align - 1)) & !(align - 1);
        let padding = aligned - current as usize;
        
        if self.allocated.get() + padding + size > self.capacity {
            return None;
        }
        
        self.allocated.set(self.allocated.get() + padding + size);
        Some(unsafe { NonNull::new_unchecked(aligned as *mut u8) })
    }

    fn clear(&mut self) {
        self.allocated.set(0);
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
/// let arena = NodeArena::new();
/// let id1 = arena.alloc(String::from("div"));
/// let id2 = arena.alloc(String::from("span"));
/// 
/// let div = arena.get::<String>(id1);
/// assert_eq!(div, "div");
/// ```
pub struct NodeArena {
    /// Chunks de memória (pelo menos um sempre existe)
    chunks: Cell<Vec<Chunk>>,
    /// Índice do chunk atual
    current_chunk: Cell<usize>,
    /// Contador de alocações (para debugging/stats)
    allocation_count: Cell<usize>,
}

/// Wrapper para dados de node DOM com estrutura de árvore
#[derive(Clone, Debug)]
pub struct ArenaNodeWithNodeData {
    pub data: crate::ace::html::tree_builder::InternalNodeData,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
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
            chunks: Cell::new(vec![chunk]),
            current_chunk: Cell::new(0),
            allocation_count: Cell::new(0),
        }
    }

    /// Aloca espaço para um valor do tipo T na arena
    /// 
    /// Retorna um NodeId que pode ser usado para recuperar o valor
    #[inline]
    pub fn alloc<T>(&self, value: T) -> NodeId {
        self.allocation_count.set(self.allocation_count.get() + 1);
        
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
    pub unsafe fn get_mut<T>(&self, id: NodeId) -> &mut T {
        debug_assert!(!id.is_null(), "Cannot get_mut null NodeId");
        &mut *(id.0 as *mut T)
    }
    
    /// Define o parent de um node
    /// 
    /// # Safety
    /// - Os NodeIds devem ser válidos
    pub unsafe fn set_parent(&self, node_id: NodeId, parent: Option<NodeId>) {
        let node = self.get_mut::<ArenaNodeWithNodeData>(node_id);
        node.parent = parent;
    }
    
    /// Adiciona um child a um node
    /// 
    /// # Safety
    /// - Os NodeIds devem ser válidos
    pub unsafe fn add_child(&self, parent_id: NodeId, child_id: NodeId) {
        let parent = self.get_mut::<ArenaNodeWithNodeData>(parent_id);
        parent.children.push(child_id);
    }
    
    /// Aloca um ArenaNodeWithNodeData na arena
    pub fn alloc_node(&self, data: crate::ace::html::tree_builder::InternalNodeData) -> NodeId {
        let node = ArenaNodeWithNodeData {
            data,
            parent: None,
            children: Vec::new(),
        };
        self.alloc(node)
    }
    
    /// Obtém o parent de um node
    /// 
    /// # Safety
    /// - O NodeId deve ser válido
    pub unsafe fn get_parent(&self, node_id: NodeId) -> Option<NodeId> {
        let node = self.get::<ArenaNodeWithNodeData>(node_id);
        node.parent
    }
    
    /// Obtém os children de um node
    /// 
    /// # Safety
    /// - O NodeId deve ser válido
    pub unsafe fn get_children(&self, node_id: NodeId) -> &[NodeId] {
        let node = self.get::<ArenaNodeWithNodeData>(node_id);
        &node.children
    }
    
    /// Obtém os dados de um node
    /// 
    /// # Safety
    /// - O NodeId deve ser válido
    pub unsafe fn get_node_data(&self, node_id: NodeId) -> &crate::ace::html::tree_builder::InternalNodeData {
        let node = self.get::<ArenaNodeWithNodeData>(node_id);
        &node.data
    }

    /// Limpa toda a arena, permitindo reuso da memória
    /// 
    /// ⚠️ **Atenção:** Todos os NodeIds anteriores se tornam inválidos!
    pub fn clear(&self) {
        let mut chunks = self.chunks.take();
        for chunk in chunks.iter_mut() {
            chunk.clear();
        }
        self.current_chunk.set(0);
        self.chunks.set(chunks);
        self.allocation_count.set(0);
    }

    /// Retorna estatísticas da arena
    pub fn stats(&self) -> ArenaStats {
        let chunks = self.chunks.take();
        let total_capacity: usize = chunks.iter().map(|c| c.capacity).sum();
        let total_allocated: usize = chunks.iter().map(|c| c.allocated.get()).sum();
        self.chunks.set(chunks);
        
        ArenaStats {
            chunk_count: self.chunks.take().len(),
            total_capacity,
            total_allocated,
            allocation_count: self.allocation_count.get(),
            utilization: if total_capacity > 0 {
                total_allocated as f32 / total_capacity as f32
            } else {
                0.0
            },
        }
    }

    /// Tenta alocar no chunk atual
    #[inline]
    fn try_alloc_in_current(&self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let chunks = self.chunks.take();
        let current_idx = self.current_chunk.get();
        
        if current_idx < chunks.len() {
            let result = chunks[current_idx].alloc(size, align);
            self.chunks.set(chunks);
            return result;
        }
        
        self.chunks.set(chunks);
        None
    }

    /// Aloca um novo chunk grande o suficiente para o valor
    fn allocate_new_chunk(&self, min_size: usize, align: usize) {
        let mut chunks = self.chunks.take();
        
        // Novo chunk com pelo menos o dobro do tamanho necessário
        let new_capacity = std::cmp::max(DEFAULT_CHUNK_SIZE, min_size * 2);
        let new_chunk = Chunk::new(new_capacity);
        
        chunks.push(new_chunk);
        self.current_chunk.set(chunks.len() - 1);
        self.chunks.set(chunks);
    }
}

impl Default for NodeArena {
    fn default() -> Self {
        Self::new()
    }
}

/// Estatísticas da arena para profiling
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
        let arena = NodeArena::new();
        let id = arena.alloc(42u32);
        
        unsafe {
            assert_eq!(*arena.get::<u32>(id), 42);
        }
    }

    #[test]
    fn test_multiple_allocations() {
        let arena = NodeArena::new();
        
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
        let arena = NodeArena::new();
        
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
        let arena = NodeArena::new();
        
        // Aloca um array grande
        let data = vec![1u32, 2, 3, 4, 5];
        let id = arena.alloc(data);
        
        unsafe {
            let retrieved = arena.get::<Vec<u32>>(id);
            assert_eq!(retrieved.len(), 5);
            assert_eq!(retrieved[0], 1);
        }
    }
}
