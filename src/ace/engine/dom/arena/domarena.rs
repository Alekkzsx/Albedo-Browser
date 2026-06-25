//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use super::*;
use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)


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
pub(crate) fn default() -> Self {
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
        if let Some(idx) = self.blocks[self.current_block].push(node.clone()) {
            self.total_allocated += 1;
            return self.make_global_index(self.current_block, idx);
        }
        
        // Bloco cheio, cria novo bloco
        self.current_block += 1;
        let new_capacity = BLOCK_SIZE / std::mem::size_of::<ArenaNode>();
        self.blocks.push(ArenaBlock::new(new_capacity));
        
        let node_index = self.blocks[self.current_block].push(node).expect("Albedo Engine: internal invariant violated");
        self.total_allocated += 1;
        self.make_global_index(self.current_block, node_index)
    }
    
    /// Aloca um node reutilizando pooled memory (otimização para tipos comuns)
    pub fn alloc_pooled(&mut self, node: ArenaNode) -> usize {
        // Verifica se é um tipo comum que pode ser poolado
        match &node.node_type {
            crate::ace::engine::dom::AceNodeType::Element(el) => {
                if el.tag == "div" && !self.div_pool.is_empty() {
                    let pooled_idx = self.div_pool.pop().expect("Albedo Engine: internal invariant violated");
                    // Reutiliza slot existente
                    if let Some(existing) = self.get_mut(pooled_idx) {
                        *existing = node;
                    }
                    return pooled_idx;
                } else if el.tag == "span" && !self.span_pool.is_empty() {
                    let pooled_idx = self.span_pool.pop().expect("Albedo Engine: internal invariant violated");
                    if let Some(existing) = self.get_mut(pooled_idx) {
                        *existing = node;
                    }
                    return pooled_idx;
                }
            }
            crate::ace::engine::dom::AceNodeType::Text(_) => {
                if !self.text_pool.is_empty() {
                    let pooled_idx = self.text_pool.pop().expect("Albedo Engine: internal invariant violated");
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
                crate::ace::engine::dom::AceNodeType::Element(el) => {
                    if el.tag == "div" {
                        self.div_pool.push(global_idx);
                        return;
                    } else if el.tag == "span" {
                        self.span_pool.push(global_idx);
                        return;
                    }
                }
                crate::ace::engine::dom::AceNodeType::Text(_) => {
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
pub(crate) fn make_global_index(&self, block_idx: usize, local_idx: usize) -> usize {
        // Formato: high bits = block, low bits = local
        // Suporta até 65536 blocks e 65536 nodes por block
        (block_idx << 16) | local_idx
    }
    
    /// Divide índice global em (block_idx, local_idx)
pub(crate) fn split_index(&self, global_idx: usize) -> (usize, usize) {
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
