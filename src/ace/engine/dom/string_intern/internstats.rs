use super::*;
//! AceDOM String Interning System
//! Otimização crítica de memória para strings repetidas (tag names, attributes)
//! Reduz uso de memória em 60-80% para strings frequentes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Hasher rápido e seguro para string interning
use fxhash::FxBuildHasher;



/// Estatísticas de uso do interner
#[derive(Default, Debug, Clone)]
pub struct InternStats {
    pub total_interns: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub memory_saved_bytes: usize,
}
