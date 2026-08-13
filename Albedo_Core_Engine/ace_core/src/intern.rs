// ============================================================================
// Albedo Core Engine (ACE)
// File: intern.rs
// Description: Global String Pool (Interner) para converter strings em u32.
//              O(1) String Comparisons.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::sync::SpinLock;

/// Símbolo Internado O(1).
/// Comparações entre Symbols não comparam a String real, apenas o `u32`,
/// garantindo 1 único ciclo de clock para verificar igualdades (ex: CSS Rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

struct Interner {
    map: HashMap<&'static str, u32>,
    vec: Vec<&'static str>,
}

impl Interner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            vec: Vec::new(),
        }
    }

    fn intern(&mut self, text: &str) -> Symbol {
        if let Some(&id) = self.map.get(text) {
            return Symbol(id);
        }
        
        // Se a string não existe, devemos convertê-la em 'static str.
        // Como o interner nunca apaga dados, vazar (leak) a string aqui é o
        // comportamento correto arquiteturalmente para "imortais".
        let id = self.vec.len() as u32;
        let leaked_str: &'static str = Box::leak(text.to_string().into_boxed_str());
        
        self.map.insert(leaked_str, id);
        self.vec.push(leaked_str);
        
        Symbol(id)
    }

    fn resolve(&self, symbol: Symbol) -> Option<&'static str> {
        self.vec.get(symbol.0 as usize).copied()
    }
}

/// Tabela Global da Engine.
/// Usamos nosso próprio `SpinLock` em vez de um `Mutex` para reduzir a latência de 
/// chamadas ao kernel do SO durante o parsing massivo de HTML/CSS.
static INTERNER: OnceLock<SpinLock<Interner>> = OnceLock::new();

fn get_interner() -> &'static SpinLock<Interner> {
    INTERNER.get_or_init(|| SpinLock::new(Interner::new()))
}

/// Interna uma string e devolve seu Símbolo representativo (O(1)).
pub fn intern(text: &str) -> Symbol {
    let global = get_interner();
    let mut lock = global.lock();
    lock.intern(text)
}

/// Resolve o Símbolo de volta para texto, se existir.
pub fn resolve(symbol: Symbol) -> Option<&'static str> {
    let global = get_interner();
    let lock = global.lock();
    lock.resolve(symbol)
}


