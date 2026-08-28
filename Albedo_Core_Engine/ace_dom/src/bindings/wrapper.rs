//! # DOM Wrappers & Wrapper Uniqueness Store (Blink ScriptWrappable / DOMDataStore)
//!
//! Gerenciamento de invólucros (*wrappers*) do JavaScript sobre nós DOM na `Arena<NodeData>`,
//! garantindo unicidade de wrapper ($1:1$) e rastreabilidade total de GC (`Traceable`).

use crate::gc::{GcTracer, Traceable};
use ace_core::id::NodeId;
use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Identificador de um objeto no heap do motor JavaScript (V8 / SpiderMonkey / ace_js).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JSObjectId(pub u64);

impl JSObjectId {
    #[inline]
    pub fn new() -> Self {
        static NEXT_JS_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_JS_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for JSObjectId {
    fn default() -> Self {
        Self::new()
    }
}

/// Invólucro que conecta um nó DOM alocado na arena a uma entidade do runtime JavaScript.
#[derive(Debug, Clone)]
pub struct DOMWrapper {
    /// Identificador do nó DOM associado.
    pub node_id: NodeId,
    /// Identificador do objeto JS correspondente no heap.
    pub js_object_id: JSObjectId,
    /// Marcação se o wrapper possui referências ativas retidas no JS.
    pub is_retained: bool,
}

impl DOMWrapper {
    pub fn new(node_id: NodeId, js_object_id: JSObjectId) -> Self {
        Self {
            node_id,
            js_object_id,
            is_retained: true,
        }
    }
}

impl Traceable for DOMWrapper {
    fn trace(&self, tracer: &mut dyn GcTracer) {
        // Marca o nó DOM subjacente como vivo através do tracer
        tracer.trace_node(self.node_id);
    }
}

/// Repositório de unicidade de wrappers (Blink `DOMDataStore`).
///
/// Garante que `node1 === node2` em JavaScript retorne `true` ao reusar a mesma referência
/// do wrapper para um dado `NodeId`, evitando duplicação de memória e quebra de identidade.
#[derive(Debug, Default)]
pub struct DOMDataStore {
    /// Mapeamento de `NodeId` para seu `DOMWrapper` ativo.
    wrappers: FxHashMap<NodeId, DOMWrapper>,
    /// Mapeamento reverso de `JSObjectId` para `NodeId`.
    js_to_node: FxHashMap<JSObjectId, NodeId>,
}

impl DOMDataStore {
    /// Cria um novo `DOMDataStore` vazio.
    pub fn new() -> Self {
        Self {
            wrappers: FxHashMap::default(),
            js_to_node: FxHashMap::default(),
        }
    }

    /// Obtém o wrapper existente de um nó ou registra um novo se ainda não existir.
    pub fn get_or_create_wrapper(
        &mut self,
        node_id: NodeId,
        create_js_object: impl FnOnce() -> JSObjectId,
    ) -> &DOMWrapper {
        self.wrappers.entry(node_id).or_insert_with(|| {
            let js_id = create_js_object();
            self.js_to_node.insert(js_id, node_id);
            DOMWrapper::new(node_id, js_id)
        })
    }

    /// Retorna o `NodeId` associado a um determinado `JSObjectId`.
    #[inline]
    pub fn get_node_by_js_id(&self, js_id: JSObjectId) -> Option<NodeId> {
        self.js_to_node.get(&js_id).copied()
    }

    /// Remove a associação quando o objeto JS for coletado pelo Garbage Collector.
    pub fn remove_wrapper(&mut self, node_id: NodeId) -> Option<DOMWrapper> {
        if let Some(wrapper) = self.wrappers.remove(&node_id) {
            self.js_to_node.remove(&wrapper.js_object_id);
            Some(wrapper)
        } else {
            None
        }
    }

    /// Retorna a quantidade total de wrappers ativos.
    #[inline]
    pub fn len(&self) -> usize {
        self.wrappers.len()
    }

    /// Retorna `true` se não houver wrappers registrados.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.wrappers.is_empty()
    }
}

impl Traceable for DOMDataStore {
    fn trace(&self, tracer: &mut dyn GcTracer) {
        for wrapper in self.wrappers.values() {
            if wrapper.is_retained {
                wrapper.trace(tracer);
            }
        }
    }
}
