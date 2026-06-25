//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use super::*;
use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)


/// Tamanhos dos pools
#[derive(Debug, Clone)]
pub struct ArenaPoolSizes {
    pub div: usize,
    pub span: usize,
    pub text: usize,
}
