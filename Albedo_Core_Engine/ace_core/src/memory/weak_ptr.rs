//! # Referências Fracas e Fábricas WeakPtr (Chromium base::WeakPtr Pattern)
//!
//! Em árvores de renderização web e despachadores de tarefas assíncronas, callbacks de timers e observers
//! frequentemente precisam referenciar objetos pais sem estender o seu tempo de vida (evitando memory leaks e ciclos).
//!
//! Este módulo provê o padrão Chromium `WeakPtr<T>` e `WeakPtrFactory<T>`:
//! - Quando o proprietário (`T`) é dropado, a `WeakPtrFactory` invalida instantaneamente todos os `WeakPtr` ativos.
//! - Consultas subsequentes retornam `None` de forma segura, eliminando ponteiros pendentes (*dangling pointers*) e UAF (Use-After-Free).
//! - Provê variantes para concorrência multi-thread (`WeakPtr<T>`) e ultra-rápidas para thread única sem overhead atômico (`LocalWeakPtr<T>`).

use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ============================================================================
// 1. Thread-Safe / Multi-Thread WeakPtr (Baseado em AtomicBool)
// ============================================================================

struct SharedFlag {
    is_valid: AtomicBool,
}

/// Ponteiro fraco thread-safe que se auto-invalida quando a `WeakPtrFactory` correspondente é descartada.
pub struct WeakPtr<T> {
    ptr: Option<NonNull<T>>,
    flag: Arc<SharedFlag>,
}

unsafe impl<T: Send> Send for WeakPtr<T> {}
unsafe impl<T: Sync> Sync for WeakPtr<T> {}

impl<T> Clone for WeakPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            flag: Arc::clone(&self.flag),
        }
    }
}

impl<T> Default for WeakPtr<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T> WeakPtr<T> {
    /// Cria um `WeakPtr` nulo permanente (sempre inválido).
    pub fn null() -> Self {
        Self {
            ptr: None,
            flag: Arc::new(SharedFlag {
                is_valid: AtomicBool::new(false),
            }),
        }
    }

    /// Retorna `true` se o objeto referenciado ainda estiver vivo e válido.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.ptr.is_some() && self.flag.is_valid.load(Ordering::Acquire)
    }

    /// Retorna uma referência imutável ao objeto apontado, ou `None` se o objeto já foi destruído.
    ///
    /// # Safety Invariant
    /// O chamador deve garantir que o objeto referenciado não está sendo modificado concorrentemente
    /// sem sincronização interna durante a chamada.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.is_valid() {
            self.ptr.map(|p| unsafe { p.as_ref() })
        } else {
            None
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for WeakPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Some(val) => f.debug_tuple("WeakPtr::Valid").field(val).finish(),
            None => f.debug_tuple("WeakPtr::Invalid").finish(),
        }
    }
}

/// Fábrica de `WeakPtr` que deve ser embutida dentro da struct proprietária `T`.
///
/// Ao ser destruída (drop), invalida automaticamente todas as referências fracas emitidas.
pub struct WeakPtrFactory<T> {
    flag: Arc<SharedFlag>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Default for WeakPtrFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> WeakPtrFactory<T> {
    /// Cria uma nova fábrica de ponteiros fracos ativa.
    pub fn new() -> Self {
        Self {
            flag: Arc::new(SharedFlag {
                is_valid: AtomicBool::new(true),
            }),
            _marker: std::marker::PhantomData,
        }
    }

    /// Emite um novo `WeakPtr<T>` apontando para a instância de `owner`.
    pub fn create_weak_ptr(&self, owner: &T) -> WeakPtr<T> {
        WeakPtr {
            ptr: Some(NonNull::from(owner)),
            flag: Arc::clone(&self.flag),
        }
    }

    /// Invalida manualmente todas as referências fracas já emitidas sem destruir a fábrica.
    pub fn invalidate_weak_ptrs(&mut self) {
        self.flag.is_valid.store(false, Ordering::Release);
        self.flag = Arc::new(SharedFlag {
            is_valid: AtomicBool::new(true),
        });
    }
}

impl<T> Drop for WeakPtrFactory<T> {
    fn drop(&mut self) {
        self.flag.is_valid.store(false, Ordering::Release);
    }
}

// ============================================================================
// 2. Single-Thread LocalWeakPtr (Ultra-rápido, zero atômicos via Rc<Cell<bool>>)
// ============================================================================

struct LocalSharedFlag {
    is_valid: Cell<bool>,
}

/// Ponteiro fraco para thread única (DOM / Layout / CSS) sem custo de instruções atômicas de barramento de cache.
pub struct LocalWeakPtr<T> {
    ptr: Option<NonNull<T>>,
    flag: Rc<LocalSharedFlag>,
}

impl<T> Clone for LocalWeakPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            flag: Rc::clone(&self.flag),
        }
    }
}

impl<T> LocalWeakPtr<T> {
    /// Retorna `true` se o objeto referenciado ainda estiver vivo.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.ptr.is_some() && self.flag.is_valid.get()
    }

    /// Retorna uma referência ao objeto apontado, ou `None` se o dono foi dropado.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.is_valid() {
            self.ptr.map(|p| unsafe { p.as_ref() })
        } else {
            None
        }
    }
}

/// Fábrica para emissão de `LocalWeakPtr<T>` em thread única.
pub struct LocalWeakPtrFactory<T> {
    flag: Rc<LocalSharedFlag>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Default for LocalWeakPtrFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LocalWeakPtrFactory<T> {
    /// Cria uma nova fábrica local.
    pub fn new() -> Self {
        Self {
            flag: Rc::new(LocalSharedFlag {
                is_valid: Cell::new(true),
            }),
            _marker: std::marker::PhantomData,
        }
    }

    /// Emite um `LocalWeakPtr<T>` apontando para `owner`.
    pub fn create_weak_ptr(&self, owner: &T) -> LocalWeakPtr<T> {
        LocalWeakPtr {
            ptr: Some(NonNull::from(owner)),
            flag: Rc::clone(&self.flag),
        }
    }

    /// Invalida todos os ponteiros fracos locais já emitidos.
    pub fn invalidate_weak_ptrs(&mut self) {
        self.flag.is_valid.set(false);
        self.flag = Rc::new(LocalSharedFlag {
            is_valid: Cell::new(true),
        });
    }
}

impl<T> Drop for LocalWeakPtrFactory<T> {
    fn drop(&mut self) {
        self.flag.is_valid.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Element {
        name: String,
        factory: WeakPtrFactory<Element>,
    }

    impl Element {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                factory: WeakPtrFactory::new(),
            }
        }

        fn weak_ref(&self) -> WeakPtr<Element> {
            self.factory.create_weak_ptr(self)
        }
    }

    #[test]
    fn test_weak_ptr_lifecycle() {
        let weak_ref;
        {
            let elem = Element::new("div.header");
            weak_ref = elem.weak_ref();
            assert!(weak_ref.is_valid());
            assert_eq!(weak_ref.get().map(|e| e.name.as_str()), Some("div.header"));
        }
        // Após a destruição de `elem`, o weak_ref deve ser instantaneamente invalidado
        assert!(!weak_ref.is_valid());
        assert!(weak_ref.get().is_none());
    }

    #[test]
    fn test_weak_ptr_manual_invalidation() {
        let mut elem = Element::new("button.submit");
        let weak1 = elem.weak_ref();
        assert!(weak1.is_valid());

        elem.factory.invalidate_weak_ptrs();
        assert!(!weak1.is_valid());

        // Novos weak pointers emitidos após a invalidação voltam a ser válidos
        let weak2 = elem.weak_ref();
        assert!(weak2.is_valid());
        assert_eq!(weak2.get().map(|e| e.name.as_str()), Some("button.submit"));
    }

    #[test]
    fn test_local_weak_ptr() {
        struct LocalNode {
            id: u64,
            factory: LocalWeakPtrFactory<LocalNode>,
        }

        let weak;
        {
            let node = LocalNode {
                id: 42,
                factory: LocalWeakPtrFactory::new(),
            };
            weak = node.factory.create_weak_ptr(&node);
            assert!(weak.is_valid());
            assert_eq!(weak.get().map(|n| n.id), Some(42));
        }
        assert!(!weak.is_valid());
        assert_eq!(weak.get().map(|n| n.id), None);
    }
}
