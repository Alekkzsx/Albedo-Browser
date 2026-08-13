// ============================================================================
// Albedo Core Engine (ACE)
// File: intern.rs
// Description: Global String Pool (Interner) para converter strings em u32.
//              Usa técnica de Sharding para reduzir a contenção multi-thread.
//              O(1) String Comparisons.
// Author: Albedo Browser Engineering Team
// ============================================================================

use crate::hash::{FxHashMap, FxHasher};
use crate::string::AceString;
use crate::sync::SpinLock;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

const SHARD_COUNT: usize = 32;
const SHARD_MASK: u64 = (SHARD_COUNT - 1) as u64;

/// Símbolo Internado O(1).
/// O ID possui 5 bits para o Shard e 27 bits para o índice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

struct InternerShard {
    map: FxHashMap<AceString, u32>,
    vec: Vec<AceString>,
}

impl InternerShard {
    fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            vec: Vec::new(),
        }
    }
}

pub struct GlobalInterner {
    shards: [SpinLock<InternerShard>; SHARD_COUNT],
}

impl GlobalInterner {
    fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| SpinLock::new(InternerShard::new())),
        }
    }

    fn shard_idx(&self, text: &str) -> usize {
        let mut hasher = FxHasher::default();
        text.hash(&mut hasher);
        (hasher.finish() & SHARD_MASK) as usize
    }

    pub fn intern(&self, text: &str) -> Symbol {
        let ace_str = AceString::from_str(text);
        let shard_idx = self.shard_idx(text);
        let mut shard = self.shards[shard_idx].lock();

        if let Some(&id) = shard.map.get(&ace_str) {
            return Symbol((shard_idx as u32) << 27 | id);
        }

        let id = shard.vec.len() as u32;
        shard.map.insert(ace_str.clone(), id);
        shard.vec.push(ace_str);

        Symbol((shard_idx as u32) << 27 | id)
    }

    pub fn resolve(&self, symbol: Symbol) -> Option<String> {
        let shard_idx = (symbol.0 >> 27) as usize;
        let id = symbol.0 & 0x07FFFFFF;

        let shard = self.shards[shard_idx].lock();
        shard.vec.get(id as usize).map(|ace| ace.to_string())
    }
}

/// Tabela Global da Engine com Sharding.
static INTERNER: OnceLock<GlobalInterner> = OnceLock::new();

fn get_interner() -> &'static GlobalInterner {
    INTERNER.get_or_init(|| GlobalInterner::new())
}

/// Interna uma string e devolve seu Símbolo representativo (O(1)).
pub fn intern(text: &str) -> Symbol {
    get_interner().intern(text)
}

/// Resolve o Símbolo de volta para texto alocado, se existir.
pub fn resolve(symbol: Symbol) -> Option<String> {
    get_interner().resolve(symbol)
}
