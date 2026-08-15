// ============================================================================
// Albedo Core Engine (ACE)
// File: intern.rs
// Description: Global String Pool (Interner) para converter strings em u32.
//              Usa técnica de Sharding para reduzir a contenção multi-thread.
//              O(1) String Comparisons. Suporta limpeza de strings dinâmicas.
// Author: Albedo Browser Engineering Team
// ============================================================================

use crate::hash::{FxHashMap, FxHasher};
use crate::string::AceString;
use std::hash::{Hash, Hasher};
use std::sync::{OnceLock, RwLock};

const SHARD_COUNT: usize = 32;
const SHARD_MASK: u64 = (SHARD_COUNT - 1) as u64;

/// Limite máximo de strings dinâmicas por Shard (evita ataques OOM).
/// Aproximadamente 4 Milhões no total (32 * 131.072).
const MAX_DYNAMIC_PER_SHARD: usize = 131_072;

/// Símbolo Internado O(1).
/// O ID possui:
/// - Bit 31: Is Dynamic (1 = Dynamic, 0 = Static)
/// - Bits 26-30 (5 bits): Shard ID
/// - Bits 0-25 (26 bits): Index (máx ~67 milhões por shard)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

impl Symbol {
    #[inline(always)]
    fn is_dynamic(&self) -> bool {
        (self.0 & 0x80000000) != 0
    }
    
    #[inline(always)]
    fn shard_idx(&self) -> usize {
        ((self.0 >> 26) & 0x1F) as usize
    }
    
    #[inline(always)]
    fn index(&self) -> usize {
        (self.0 & 0x03FFFFFF) as usize
    }
}

#[repr(align(64))]
struct InternerShard {
    static_map: FxHashMap<AceString, u32>,
    static_vec: Vec<AceString>,
    
    dynamic_map: FxHashMap<AceString, u32>,
    dynamic_vec: Vec<AceString>,
}

impl InternerShard {
    fn new() -> Self {
        Self {
            static_map: FxHashMap::default(),
            static_vec: Vec::new(),
            dynamic_map: FxHashMap::default(),
            dynamic_vec: Vec::new(),
        }
    }
    
    fn flush_dynamic(&mut self) {
        self.dynamic_map.clear();
        self.dynamic_vec.clear();
    }
}

pub struct GlobalInterner {
    shards: [RwLock<InternerShard>; SHARD_COUNT],
}

impl GlobalInterner {
    fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| RwLock::new(InternerShard::new())),
        }
    }

    /// Hash otimizado preparatório para vetorização SIMD de 16/32 bytes.
    #[inline]
    fn shard_idx(&self, text: &str) -> usize {
        let mut hasher = FxHasher::default();
        // Um motor SIMD processaria chunks de 32 bytes aqui antes do Fallback.
        text.hash(&mut hasher);
        (hasher.finish() & SHARD_MASK) as usize
    }

    /// Interna uma string estática (Ex: Tags HTML ou Keywords CSS). Nunca é deletada.
    pub fn intern_static(&self, text: &str) -> Symbol {
        let ace_str = AceString::from_str(text);
        let shard_idx = self.shard_idx(text);

        {
            let shard = self.shards[shard_idx].read().unwrap();
            if let Some(&id) = shard.static_map.get(&ace_str) {
                return Symbol((shard_idx as u32) << 26 | id);
            }
        }

        let mut shard = self.shards[shard_idx].write().unwrap();
        if let Some(&id) = shard.static_map.get(&ace_str) {
            return Symbol((shard_idx as u32) << 26 | id);
        }

        let id = shard.static_vec.len() as u32;
        shard.static_map.insert(ace_str.clone(), id);
        shard.static_vec.push(ace_str);

        Symbol((shard_idx as u32) << 26 | id)
    }

    /// Interna uma string dinâmica (Ex: IDs gerados por JavaScript).
    /// Pode ser expurgada da memória para evitar Memory Leaks ou ataques OOM.
    pub fn intern_dynamic(&self, text: &str) -> Symbol {
        let ace_str = AceString::from_str(text);
        let shard_idx = self.shard_idx(text);

        {
            let shard = self.shards[shard_idx].read().unwrap();
            if let Some(&id) = shard.dynamic_map.get(&ace_str) {
                return Symbol(0x80000000 | (shard_idx as u32) << 26 | id);
            }
        }

        let mut shard = self.shards[shard_idx].write().unwrap();
        if let Some(&id) = shard.dynamic_map.get(&ace_str) {
            return Symbol(0x80000000 | (shard_idx as u32) << 26 | id);
        }

        // Defesa OOM (Out of Memory): Se o limite estourar, limpa o cache dinâmico deste shard.
        if shard.dynamic_vec.len() >= MAX_DYNAMIC_PER_SHARD {
            crate::ace_trace!("Interner Shard {} estourou capacidade de strings dinâmicas. Limpando!", shard_idx);
            shard.flush_dynamic();
        }

        let id = shard.dynamic_vec.len() as u32;
        shard.dynamic_map.insert(ace_str.clone(), id);
        shard.dynamic_vec.push(ace_str);

        Symbol(0x80000000 | (shard_idx as u32) << 26 | id)
    }

    pub fn resolve(&self, symbol: Symbol) -> Option<String> {
        let shard_idx = symbol.shard_idx();
        let id = symbol.index();
        
        let shard = self.shards[shard_idx].read().unwrap();
        if symbol.is_dynamic() {
            shard.dynamic_vec.get(id).map(|ace| ace.to_string())
        } else {
            shard.static_vec.get(id).map(|ace| ace.to_string())
        }
    }
    
    /// Limpa toda a memória ocupada por strings dinâmicas (Chamado ao fechar abas pesadas).
    pub fn flush_dynamic_strings(&self) {
        for shard_lock in &self.shards {
            let mut shard = shard_lock.write().unwrap();
            shard.flush_dynamic();
        }
    }
}

/// Tabela Global da Engine com Sharding.
static INTERNER: OnceLock<GlobalInterner> = OnceLock::new();

fn get_interner() -> &'static GlobalInterner {
    INTERNER.get_or_init(|| GlobalInterner::new())
}

/// Interna uma string dinamicamente por padrão (O(1)).
pub fn intern(text: &str) -> Symbol {
    get_interner().intern_dynamic(text)
}

/// Interna uma string estaticamente (O(1)).
pub fn intern_static(text: &str) -> Symbol {
    get_interner().intern_static(text)
}

/// Resolve o Símbolo de volta para texto alocado, se existir.
pub fn resolve(symbol: Symbol) -> Option<String> {
    get_interner().resolve(symbol)
}

/// Limpa toda a memória ocupada por strings dinâmicas (Chamado ao fechar abas pesadas).
pub fn flush_dynamic_strings() {
    get_interner().flush_dynamic_strings();
}
