// ============================================================================
// Albedo Core Engine (ACE)
// File: intern.rs
// Description: Global String Pool (Interner) para converter strings em u32.
//              O(1) String Comparisons.
// Author: Albedo Browser Engineering Team
// ============================================================================

use crate::hash::FxHashMap;
use crate::string::AceString;
use crate::sync::SpinLock;
use std::sync::OnceLock;

/// Símbolo Internado O(1).
/// Comparações entre Symbols não comparam a String real, apenas o `u32`,
/// garantindo 1 único ciclo de clock para verificar igualdades (ex: CSS Rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

struct Interner {
    map: FxHashMap<AceString, u32>,
    vec: Vec<AceString>,
}

impl Interner {
    fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            vec: Vec::new(),
        }
    }

    fn intern(&mut self, text: &str) -> Symbol {
        // Converte imediatamente usando Small String Optimization sem alocar no Heap
        // para strings menores que 24 caracteres.
        let ace_str = AceString::from_str(text);

        if let Some(&id) = self.map.get(&ace_str) {
            return Symbol(id);
        }

        let id = self.vec.len() as u32;
        self.map.insert(ace_str.clone(), id);
        self.vec.push(ace_str);

        Symbol(id)
    }

    fn resolve(&self, symbol: Symbol) -> Option<String> {
        self.vec.get(symbol.0 as usize).map(|ace| ace.to_string())
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

/// Resolve o Símbolo de volta para texto alocado, se existir.
pub fn resolve(symbol: Symbol) -> Option<String> {
    let global = get_interner();
    let lock = global.lock();
    lock.resolve(symbol)
}
