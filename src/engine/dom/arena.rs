//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)
const BLOCK_SIZE: usize = 4096;

/// Alinhamento para cache line (64 bytes)
const CACHE_LINE_ALIGN: usize = 64;

/// Capacidade inicial da arena (número de nodes)
const INITIAL_CAPACITY: usize = 256;

/// Estrutura que representa um nó na arena
#[repr(C)]
#[derive(Clone, Debug)]
pub struct ArenaNode {
    pub node_type: crate::dom::AceNodeType,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub prev_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub shadow_root: Option<usize>,
    pub dirty: crate::dom::NodeDirtyFlags,
}

impl ArenaNode {
    pub fn new(node_type: crate::dom::AceNodeType) -> Self {
        Self {
            node_type,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: crate::dom::NodeDirtyFlags::LAYOUT | crate::dom::NodeDirtyFlags::STYLE,
        }
    }
    
    /// Retorna o tamanho em bytes deste nó (para cálculo de memória)
    pub fn size_bytes(&self) -> usize {
        let base_size = std::mem::size_of::<Self>();
        let children_size = self.children.capacity() * std::mem::size_of::<usize>();
        
        // Tamanho aproximado do node_type
        let node_type_size = match &self.node_type {
            crate::dom::AceNodeType::Element(el) => {
                el.tag.len() + std::mem::size_of_val(&el.attributes)
            }
            crate::dom::AceNodeType::Text(s) | crate::dom::AceNodeType::Comment(s) => s.len(),
            _ => 0,
        };
        
        base_size + children_size + node_type_size
    }
}

/// Um bloco de memória na arena
struct ArenaBlock {
    data: NonNull<[ArenaNode]>,
    capacity: usize,
    len: Cell<usize>,
}

impl ArenaBlock {
    fn new(capacity: usize) -> Self {
        let layout = Layout::array::<ArenaNode>(capacity).unwrap();
        unsafe {
            let ptr = alloc::alloc(layout);
            if ptr.is_null() {
                alloc::handle_alloc_error(layout);
            }
            
            // Inicializar memória
            for i in 0..capacity {
                std::ptr::write(ptr.add(i) as *mut ArenaNode, ArenaNode::new(
                    crate::dom::AceNodeType::Document
                ));
            }
            
            let slice = std::slice::from_raw_parts_mut(ptr as *mut ArenaNode, capacity);
            Self {
                data: NonNull::new_unchecked(slice),
                capacity,
                len: Cell::new(0),
            }
        }
    }
    
    fn push(&self, node: ArenaNode) -> Option<usize> {
        let current_len = self.len.get();
        if current_len >= self.capacity {
            return None;
        }
        
        unsafe {
            let ptr = self.data.as_ptr() as *mut ArenaNode;
            std::ptr::write(ptr.add(current_len), node);
        }
        
        self.len.set(current_len + 1);
        Some(current_len)
    }
    
    fn get(&self, index: usize) -> Option<&ArenaNode> {
        if index >= self.len.get() {
            return None;
        }
        unsafe {
            Some(&*self.data.as_ptr().add(index))
        }
    }
    
    fn get_mut(&mut self, index: usize) -> Option<&mut ArenaNode> {
        if index >= self.len.get() {
            return None;
        }
        unsafe {
            Some(&mut *self.data.as_ptr().add(index))
        }
    }
}

impl Drop for ArenaBlock {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::array::<ArenaNode>(self.capacity).unwrap();
            alloc::dealloc(self.data.as_ptr() as *mut u8, layout);
        }
    }
}

/// Arena principal para alocação de nodes DOM
pub struct DomArena {
    blocks: Vec<ArenaBlock>,
    current_block: usize,
    total_allocated: usize,
    
    // Pools para tipos comuns (otimização)
    div_pool: Vec<usize>,
    span_pool: Vec<usize>,
    text_pool: Vec<usize>,
}

impl Default for DomArena {
    fn default() -> Self {
        Self::new(INITIAL_CAPACITY)
    }
}

impl DomArena {
    /// Cria uma nova arena com capacidade inicial
    pub fn new(initial_capacity: usize) -> Self {
        let mut arena = Self {
            blocks: Vec::new(),
            current_block: 0,
            total_allocated: 0,
            div_pool: Vec::new(),
            span_pool: Vec::new(),
            text_pool: Vec::new(),
        };
        
        // Adiciona bloco inicial
        arena.blocks.push(ArenaBlock::new(initial_capacity));
        arena
    }
    
    /// Aloca um novo node na arena
    pub fn alloc(&mut self, node: ArenaNode) -> usize {
        // Tenta alocar no bloco atual
        if let Some(idx) = self.blocks[self.current_block].push(node) {
            self.total_allocated += 1;
            return self.make_global_index(self.current_block, idx);
        }
        
        // Bloco cheio, cria novo bloco
        self.current_block += 1;
        let new_capacity = BLOCK_SIZE / std::mem::size_of::<ArenaNode>();
        self.blocks.push(ArenaBlock::new(new_capacity));
        
        let idx = self.blocks[self.current_block].push(node).unwrap();
        self.total_allocated += 1;
        self.make_global_index(self.current_block, idx)
    }
    
    /// Aloca um node reutilizando pooled memory (otimização para tipos comuns)
    pub fn alloc_pooled(&mut self, node: ArenaNode) -> usize {
        // Verifica se é um tipo comum que pode ser poolado
        match &node.node_type {
            crate::dom::AceNodeType::Element(el) => {
                if el.tag == "div" && !self.div_pool.is_empty() {
                    let pooled_idx = self.div_pool.pop().unwrap();
                    // Reutiliza slot existente
                    if let Some(existing) = self.get_mut(pooled_idx) {
                        *existing = node;
                    }
                    return pooled_idx;
                } else if el.tag == "span" && !self.span_pool.is_empty() {
                    let pooled_idx = self.span_pool.pop().unwrap();
                    if let Some(existing) = self.get_mut(pooled_idx) {
                        *existing = node;
                    }
                    return pooled_idx;
                }
            }
            crate::dom::AceNodeType::Text(_) => {
                if !self.text_pool.is_empty() {
                    let pooled_idx = self.text_pool.pop().unwrap();
                    if let Some(existing) = self.get_mut(pooled_idx) {
                        *existing = node;
                    }
                    return pooled_idx;
                }
            }
            _ => {}
        }
        
        // Não encontrou pool, aloca normalmente
        self.alloc(node)
    }
    
    /// Libera um node (adiciona ao pool se elegível)
    pub fn dealloc(&mut self, global_idx: usize) {
        let (block_idx, local_idx) = self.split_index(global_idx);
        
        if block_idx >= self.blocks.len() {
            return;
        }
        
        // Adiciona ao pool apropriado para reutilização
        if let Some(node) = self.blocks[block_idx].get(local_idx) {
            match &node.node_type {
                crate::dom::AceNodeType::Element(el) => {
                    if el.tag == "div" {
                        self.div_pool.push(global_idx);
                        return;
                    } else if el.tag == "span" {
                        self.span_pool.push(global_idx);
                        return;
                    }
                }
                crate::dom::AceNodeType::Text(_) => {
                    self.text_pool.push(global_idx);
                    return;
                }
                _ => {}
            }
        }
        
        // Não é poolable, apenas marca como disponível
        // (em uma implementação mais avançada, poderíamos compactar)
    }
    
    /// Obtém referência imutável para um node
    pub fn get(&self, global_idx: usize) -> Option<&ArenaNode> {
        let (block_idx, local_idx) = self.split_index(global_idx);
        
        if block_idx >= self.blocks.len() {
            return None;
        }
        
        self.blocks[block_idx].get(local_idx)
    }
    
    /// Obtém referência mutável para um node
    pub fn get_mut(&mut self, global_idx: usize) -> Option<&mut ArenaNode> {
        let (block_idx, local_idx) = self.split_index(global_idx);
        
        if block_idx >= self.blocks.len() {
            return None;
        }
        
        self.blocks[block_idx].get_mut(local_idx)
    }
    
    /// Retorna o número total de nodes alocados
    pub fn len(&self) -> usize {
        self.total_allocated
    }
    
    /// Retorna true se a arena está vazia
    pub fn is_empty(&self) -> bool {
        self.total_allocated == 0
    }
    
    /// Retorna estatísticas de uso de memória
    pub fn memory_stats(&self) -> ArenaMemoryStats {
        let mut total_bytes = 0;
        let mut used_bytes = 0;
        
        for block in &self.blocks {
            total_bytes += block.capacity * std::mem::size_of::<ArenaNode>();
            used_bytes += block.len.get() * std::mem::size_of::<ArenaNode>();
        }
        
        ArenaMemoryStats {
            total_bytes,
            used_bytes,
            wasted_bytes: total_bytes - used_bytes,
            pool_sizes: ArenaPoolSizes {
                div: self.div_pool.len(),
                span: self.span_pool.len(),
                text: self.text_pool.len(),
            },
        }
    }
    
    /// Converte (block_idx, local_idx) em índice global
    fn make_global_index(&self, block_idx: usize, local_idx: usize) -> usize {
        // Formato: high bits = block, low bits = local
        // Suporta até 65536 blocks e 65536 nodes por block
        (block_idx << 16) | local_idx
    }
    
    /// Divide índice global em (block_idx, local_idx)
    fn split_index(&self, global_idx: usize) -> (usize, usize) {
        let block_idx = global_idx >> 16;
        let local_idx = global_idx & 0xFFFF;
        (block_idx, local_idx)
    }
    
    /// Limpa toda a arena (reset completo)
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.blocks.push(ArenaBlock::new(INITIAL_CAPACITY));
        self.current_block = 0;
        self.total_allocated = 0;
        self.div_pool.clear();
        self.span_pool.clear();
        self.text_pool.clear();
    }
}

/// Estatísticas de uso de memória da arena
#[derive(Debug, Clone)]
pub struct ArenaMemoryStats {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub wasted_bytes: usize,
    pub pool_sizes: ArenaPoolSizes,
}

/// Tamanhos dos pools
#[derive(Debug, Clone)]
pub struct ArenaPoolSizes {
    pub div: usize,
    pub span: usize,
    pub text: usize,
}

impl ArenaMemoryStats {
    /// Retorna eficiência de uso (0.0 a 1.0)
    pub fn efficiency(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.used_bytes as f64 / self.total_bytes as f64
    }
    
    /// Retorna porcentagem de desperdício
    pub fn waste_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.wasted_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::{AceNodeType, AceElement, NodeDirtyFlags};
    
    #[test]
    fn test_basic_allocation() {
        let mut arena = DomArena::new(10);
        
        let node = ArenaNode::new(AceNodeType::Document);
        let idx = arena.alloc(node);
        
        assert!(arena.get(idx).is_some());
        assert_eq!(arena.len(), 1);
    }
    
    #[test]
    fn test_multiple_allocations() {
        let mut arena = DomArena::new(5);
        
        let mut indices = Vec::new();
        for i in 0..10 {
            let node = ArenaNode::new(AceNodeType::Text(format!("text{}", i).into()));
            let idx = arena.alloc(node);
            indices.push(idx);
        }
        
        assert_eq!(arena.len(), 10);
        
        // Verifica se todos os nodes estão acessíveis
        for (i, &idx) in indices.iter().enumerate() {
            assert!(arena.get(idx).is_some());
        }
    }
    
    #[test]
    fn test_memory_stats() {
        let mut arena = DomArena::new(100);
        
        for i in 0..50 {
            let node = ArenaNode::new(AceNodeType::Text(format!("text{}", i).into()));
            arena.alloc(node);
        }
        
        let stats = arena.memory_stats();
        assert!(stats.used_bytes > 0);
        assert!(stats.total_bytes >= stats.used_bytes);
        assert!(stats.efficiency() > 0.0);
    }
    
    #[test]
    fn test_pooling() {
        let mut arena = DomArena::new(10);
        
        // Aloca alguns divs
        let div1 = ArenaNode::new(AceNodeType::Element(AceElement {
            tag: "div".to_string(),
            namespace: crate::ace::html::Namespace::Html,
            attributes: std::collections::HashMap::new(),
        }));
        let idx1 = arena.alloc_pooled(div1);
        
        // Libera
        arena.dealloc(idx1);
        
        // Aloca outro div - deve reutilizar
        let div2 = ArenaNode::new(AceNodeType::Element(AceElement {
            tag: "div".to_string(),
            namespace: crate::ace::html::Namespace::Html,
            attributes: std::collections::HashMap::new(),
        }));
        let idx2 = arena.alloc_pooled(div2);
        
        // Deve ter reutilizado o mesmo índice
        assert_eq!(idx1, idx2);
    }
}
