//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use super::*;
use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)


/// Estatísticas de uso de memória da arena
#[derive(Debug, Clone)]
pub struct ArenaMemoryStats {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub wasted_bytes: usize,
    pub pool_sizes: ArenaPoolSizes,
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
