use bumpalo::Bump;
use std::cell::RefCell;
use std::rc::Rc;

/// O `AceAllocator` é o motor de gerenciamento de memória de ultra-performance do Albedo.
/// Ele utiliza alocação por Arena (Bump Allocation), permitindo que milhares de nós do DOM
/// e atributos sejam alocados em blocos contínuos de memória.
/// 
/// Isso elimina o overhead do `malloc` tradicional e garante localidade de cache superior
/// durante o parsing e o style resolve.
pub struct AceAllocator {
    arena: Bump,
}

impl AceAllocator {
    /// Cria um novo alocador com uma arena vazia.
    pub fn new() -> Self {
        Self {
            arena: Bump::new(),
        }
    }

    /// Cria um novo alocador com uma capacidade inicial sugerida.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Bump::with_capacity(capacity),
        }
    }

    /// Aloca um valor na arena e retorna uma referência mutável.
    /// O valor viverá enquanto o alocador viver.
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.arena.alloc(value)
    }

    /// Aloca uma string na arena, útil para nomes de tags e atributos curtos.
    pub fn alloc_str(&self, value: &str) -> &str {
        self.arena.alloc_str(value)
    }

    /// Limpa a arena, invalidando todas as referências anteriores.
    /// Extremamente rápido: apenas reseta um ponteiro interno.
    pub fn reset(&mut self) {
        self.arena.reset();
    }

    /// Retorna a quantidade de bytes já alocados na arena.
    pub fn allocated_bytes(&self) -> usize {
        self.arena.allocated_bytes()
    }
}

/// Helper para usar o alocador de forma compartilhada durante o pipeline de parsing.
#[derive(Clone)]
pub struct SharedAceAllocator(Rc<RefCell<AceAllocator>>);

impl SharedAceAllocator {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(AceAllocator::new())))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Rc::new(RefCell::new(AceAllocator::with_capacity(capacity))))
    }

    pub fn use_allocator<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&AceAllocator) -> R,
    {
        let allocator = self.0.borrow();
        f(&allocator)
    }

    pub fn reset(&self) {
        self.0.borrow_mut().reset();
    }
}
