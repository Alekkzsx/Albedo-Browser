//! # Arena — Gerenciamento de Memória do DOM e do Layout
//!
//! Este módulo resolve o problema mais traiçoeiro da construção de um navegador em
//! Rust: **a árvore DOM é inerentemente cíclica**. O pai referencia o primeiro filho,
//! que referencia de volta o pai; irmãos se encadeiam nos dois sentidos.
//!
//! A resposta ingênua — `Rc<RefCell<Node>>` — cria ciclos que jamais zeram a contagem
//! de referências, vazando memória indefinidamente. Este módulo resolve o problema com
//! duas estruturas complementares:
//!
//! | Estrutura | Propósito | Referencia por... |
//! |---|---|---|
//! | [`Arena<T>`] | Nós homogêneos (DOM, Layout) que se apontam mutuamente | [`ArenaId<T>`] (geracional) |
//! | [`BumpArena`] | Dados auxiliares heterogêneos (strings, fatias, temporários) | referência direta `&mut T` |
//!
//! ## O truque: referências que não possuem
//!
//! Um nó DOM guarda seus vizinhos como [`ArenaId<T>`] — um par `(índice, versão)` de
//! 8 bytes, `Copy`, que **não mantém nada vivo**. A única "dona" da memória é a arena.
//! Quando a aba ou o iframe é destruído, [`DomArena::reset`] descarta tudo de uma vez,
//! sem percorrer nó por nó e sem se importar com ciclos.
//!
//! ## Integração futura com o GC (Fase 10)
//!
//! Quando o `ace_js` existir, objetos JavaScript poderão apontar para nós DOM e
//! vice-versa, criando ciclos *entre os dois heaps*. As arenas expõem iteração sobre
//! todos os itens vivos ([`Arena::iter`]) para que o coletor de ciclos faça a marcação
//! e a quebra desses grafos — exatamente como o Oilpan faz no Blink.
//!
//! ## Exemplo rápido
//!
//! ```
//! use ace_core::arena::{Arena, ArenaId};
//!
//! #[derive(Debug, PartialEq)]
//! struct Node {
//!     parent: Option<ArenaId<Node>>,
//!     tag: &'static str,
//! }
//!
//! let mut arena: Arena<Node> = Arena::new();
//! let body = arena.alloc(Node { parent: None, tag: "body" });
//! let div  = arena.alloc(Node { parent: Some(body), tag: "div" });
//!
//! // Navegando do filho para o pai, sem Rc, sem RefCell, sem ciclos vazando.
//! let div_ref = arena.get(div).unwrap();
//! let body_ref = arena.get(div_ref.parent.unwrap()).unwrap();
//! assert_eq!(body_ref.tag, "body");
//! ```

mod bump;
mod id;
mod slab;
mod stats;

pub use bump::BumpArena;
pub use id::ArenaId;
pub use slab::Arena;
pub use stats::ArenaStats;

/// Arena composta típica do DOM: nós geracionais + payloads de *bump allocation*.
///
/// `ace_dom` (Fase 5) irá especializar este tipo com o nó concreto. Ele é mantido
/// genérico aqui, na camada de fundação, para não acoplar o `ace_core` a tipos do DOM.
///
/// A composição reflete como motores maduros organizam a memória: os **nós** precisam
/// se referenciar por ID estável (por isso a [`Arena`] geracional), enquanto os
/// **payloads** — strings de atributos, valores do parser — só precisam viver tanto
/// quanto a página (por isso a [`BumpArena`]).
#[derive(Debug)]
pub struct DomArena<N> {
    /// Armazenamento dos nós, referenciados por [`ArenaId<N>`].
    nodes: Arena<N>,
    /// Armazenamento de dados auxiliares (strings, fatias, temporários).
    payloads: BumpArena,
}

impl<N> DomArena<N> {
    /// Cria uma arena de DOM vazia.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Arena::new(),
            payloads: BumpArena::new(),
        }
    }

    /// Cria uma arena com capacidade pré-alocada, evitando *reallocs* no carregamento.
    ///
    /// * `node_capacity`: número esperado de nós.
    /// * `payload_bytes`: bytes esperados de dados auxiliares.
    #[must_use]
    pub fn with_capacity(node_capacity: usize, payload_bytes: usize) -> Self {
        Self {
            nodes: Arena::with_capacity(node_capacity),
            payloads: BumpArena::with_capacity(payload_bytes),
        }
    }

    /// Aloca um nó e retorna seu identificador estável.
    #[inline]
    pub fn alloc_node(&mut self, node: N) -> ArenaId<N> {
        self.nodes.alloc(node)
    }

    /// Obtém uma referência imutável a um nó, se o identificador ainda for válido.
    #[inline]
    #[must_use]
    pub fn node(&self, id: ArenaId<N>) -> Option<&N> {
        self.nodes.get(id)
    }

    /// Obtém uma referência mutável a um nó, se o identificador ainda for válido.
    #[inline]
    #[must_use]
    pub fn node_mut(&mut self, id: ArenaId<N>) -> Option<&mut N> {
        self.nodes.get_mut(id)
    }

    /// Remove um nó, devolvendo-o. O identificador passa a ser inválido.
    #[inline]
    pub fn remove_node(&mut self, id: ArenaId<N>) -> Option<N> {
        self.nodes.remove(id)
    }

    /// Aloca um dado auxiliar (payload) e retorna uma referência válida até o `reset`.
    #[inline]
    pub fn alloc_payload<T>(&self, value: T) -> &mut T {
        self.payloads.alloc(value)
    }

    /// Aloca uma cópia de uma `str` como payload. Útil para atributos e texto.
    #[inline]
    pub fn alloc_str(&self, s: &str) -> &mut str {
        self.payloads.alloc_str(s)
    }

    /// Número de nós vivos.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// `true` se não houver nenhum nó vivo.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Itera sobre todos os nós vivos — ponto de integração com o GC (Fase 10).
    #[inline]
    pub fn node_iter(&self) -> impl Iterator<Item = (ArenaId<N>, &N)> {
        self.nodes.iter()
    }

    /// Descarta **todos** os nós e payloads de uma só vez.
    ///
    /// Chamado quando a página/iframe é destruída. É o equivalente a "fechar a aba":
    /// nenhuma desalocação individual acontece, e nenhum ciclo precisa ser quebrado.
    /// Todos os [`ArenaId`] emitidos anteriormente tornam-se inválidos.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.payloads.reset();
    }
}

impl<N> Default for DomArena<N> {
    fn default() -> Self {
        Self::new()
    }
}