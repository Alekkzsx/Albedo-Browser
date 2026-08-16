//! # Arena DOM Allocator
//!
//! A árvore DOM sofre de problemas cíclicos (O pai aponta pro filho, que aponta pro pai).
//! Em Rust, isso exige `Rc<RefCell<Node>>` e causa vazamentos terríveis se um ciclo não for
//! quebrado.
//!
//! A `DomArena` resolve isso alocando todos os nós sequencialmente em grandes blocos
//! de memória. Quando a página (iframe) é fechada, a Arena inteira é descartada instantaneamente,
//! sem necessidade de andar nó por nó e sem medo de ciclos.

use bumpalo::Bump;

/// Uma arena segura focada na alocação da Árvore de Layout e DOM.
pub struct DomArena {
    bump: Bump,
}

impl DomArena {
    /// Cria uma nova Arena. Geralmente, existe uma Arena por Aba ou Iframe.
    pub fn new() -> Self {
        Self {
            bump: Bump::new(),
        }
    }

    /// Aloca um objeto na Arena e retorna uma referência de tempo de vida (lifetime)
    /// atrelada a esta Arena.
    #[inline]
    pub fn alloc<T>(&self, val: T) -> &mut T {
        self.bump.alloc(val)
    }

    /// Reseta a Arena, descartando todos os objetos de uma só vez (muito rápido).
    ///
    /// **Atenção:** Tipos alocados na Arena *não terão seu método `Drop` chamado automaticamente* 
    /// pelo `bumpalo::Bump` a menos que alocados explicitamente via ferramentas de Drop.
    /// Para o DOM (onde a maioria são structs de dados planas e IDs), isso é a performance perfeita.
    #[inline]
    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

impl Default for DomArena {
    fn default() -> Self {
        Self::new()
    }
}


