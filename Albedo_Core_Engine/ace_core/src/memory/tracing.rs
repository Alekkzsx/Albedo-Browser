//! # Protocolo Unificado de Tracing e Raízes de GC (Blink Oilpan Pattern)
//!
//! Fornece traits de visitação e marcação de grafos para quebra de ciclos entre
//! a Árvore DOM (`ace_dom`), nós de layout e o Coletor de Lixo do motor JavaScript (`ace_js`).

use crate::id::NodeId;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static ROOT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Interface de visitação para o algoritmo de Marcação e Varredura (*Mark-and-Sweep Tracing*).
pub trait Visitor {
    /// Registra a visita a um identificador de nó na árvore DOM / Render Tree.
    fn visit_node(&mut self, id: NodeId);

    /// Registra a visita a uma raiz de memória ou estrutura rastreável aninhada.
    fn visit_root(&mut self, root: &dyn Traceable);

    /// Registra a visita a um objeto opaco identificado por tag e ID numérico.
    fn visit_opaque(&mut self, tag: &'static str, id: u64);
}

/// Contrato obrigatório para qualquer estrutura gerenciada por GC ou participante de grafos cíclicos.
pub trait Traceable: Send + Sync {
    /// Percorre todas as referências ativas de saída e as entrega ao `Visitor`.
    fn trace(&self, visitor: &mut dyn Visitor);
}

/// Conjunto thread-safe de raízes de memória ativas do processo ou da aba.
#[derive(Default)]
pub struct RootSet {
    roots: RwLock<Vec<(u64, Arc<dyn Traceable>)>>,
}

impl RootSet {
    /// Cria um novo conjunto de raízes vazio.
    pub fn new() -> Self {
        Self {
            roots: RwLock::new(Vec::new()),
        }
    }

    /// Registra uma nova raiz ativa no coletor de lixo. Retorna o ID único para desregistro.
    pub fn add_root(&self, root: Arc<dyn Traceable>) -> u64 {
        let id = ROOT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.roots.write().push((id, root));
        id
    }

    /// Remove uma raiz do conjunto de raízes ativas.
    pub fn remove_root(&self, id: u64) -> bool {
        let mut guard = self.roots.write();
        if let Some(pos) = guard.iter().position(|(root_id, _)| *root_id == id) {
            guard.remove(pos);
            true
        } else {
            false
        }
    }

    /// Executa o rastreamento a partir de todas as raízes registradas sobre o `Visitor` fornecido.
    pub fn trace_all(&self, visitor: &mut dyn Visitor) {
        let snapshot: Vec<Arc<dyn Traceable>> = {
            let guard = self.roots.read();
            guard.iter().map(|(_, r)| Arc::clone(r)).collect()
        };

        for root in snapshot {
            visitor.visit_root(&*root);
        }
    }

    /// Retorna a quantidade de raízes ativas no conjunto.
    pub fn len(&self) -> usize {
        self.roots.read().len()
    }

    /// Retorna `true` se não houver raízes registradas.
    pub fn is_empty(&self) -> bool {
        self.roots.read().is_empty()
    }
}

/// Handle RAII que registra automaticamente uma raiz no `RootSet` e a remove no `Drop`.
pub struct GCRoot<T: Traceable> {
    id: u64,
    root_set: Arc<RootSet>,
    value: Arc<T>,
}

impl<T: Traceable + 'static> GCRoot<T> {
    /// Cria uma nova raiz RAII protegida contra coleta.
    pub fn new(value: T, root_set: Arc<RootSet>) -> Self {
        let arc_val = Arc::new(value);
        let id = root_set.add_root(Arc::clone(&arc_val) as Arc<dyn Traceable>);
        Self {
            id,
            root_set,
            value: arc_val,
        }
    }

    /// Acessa o valor interno da raiz.
    #[inline]
    pub fn get(&self) -> &T {
        &self.value
    }
}

impl<T: Traceable> std::ops::Deref for GCRoot<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Traceable> Drop for GCRoot<T> {
    fn drop(&mut self) {
        self.root_set.remove_root(self.id);
    }
}

impl<T: Traceable> Traceable for GCRoot<T> {
    fn trace(&self, visitor: &mut dyn Visitor) {
        self.value.trace(visitor);
    }
}
