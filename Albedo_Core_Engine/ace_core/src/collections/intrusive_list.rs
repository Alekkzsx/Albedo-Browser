//! # Lista Duplamente Encadeada Intrusiva (Chromium base::LinkedList & Ladybird AK::IntrusiveList Pattern)
//!
//! Em estruturas de dados de renderização e despachadores de tarefas, alocar nós intermediários (`Box<Node>`)
//! a cada inserção em fila causa fragmentação severa de heap.
//!
//! A lista intrusiva exige que o próprio elemento `T` contenha os ponteiros de encadeamento (`IntrusiveLink`),
//! garantindo:
//! - $O(1)$ para inserção no início/fim e remoção arbitrária por referência ao nó.
//! - **Zero alocações de heap** para inserção, movimentação e remoção.
//! - Desacoplamento de tempo de vida com integridade de ponteiros.

use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;

/// Link intrusivo que deve ser embutido na struct que deseja participar da `IntrusiveList`.
#[derive(Default)]
pub struct IntrusiveLink {
    prev: Cell<Option<NonNull<IntrusiveLink>>>,
    next: Cell<Option<NonNull<IntrusiveLink>>>,
    is_linked: Cell<bool>,
}

impl IntrusiveLink {
    /// Cria um novo link desconectado.
    #[inline]
    pub const fn new() -> Self {
        Self {
            prev: Cell::new(None),
            next: Cell::new(None),
            is_linked: Cell::new(false),
        }
    }

    /// Retorna `true` se o link estiver atualmente inserido em uma lista.
    #[inline]
    pub fn is_linked(&self) -> bool {
        self.is_linked.get()
    }

    /// Desconecta os ponteiros internos sem atualizar a lista (usado em teardown).
    #[inline]
    fn unlink_internal(&self) {
        self.prev.set(None);
        self.next.set(None);
        self.is_linked.set(false);
    }
}

// Send + Sync para permitir listas em contextos multi-thread sob proteção de locks externos
unsafe impl Send for IntrusiveLink {}
unsafe impl Sync for IntrusiveLink {}

impl fmt::Debug for IntrusiveLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntrusiveLink")
            .field("is_linked", &self.is_linked.get())
            .finish()
    }
}

/// Trait implementado por tipos que contêm um `IntrusiveLink` embutido.
pub trait IntrusiveNode {
    /// Retorna uma referência ao link intrusivo deste nó.
    fn intrusive_link(&self) -> &IntrusiveLink;
}

/// Lista duplamente encadeada intrusiva não-alocadora.
pub struct IntrusiveList<T: IntrusiveNode + ?Sized> {
    head: Option<NonNull<IntrusiveLink>>,
    tail: Option<NonNull<IntrusiveLink>>,
    len: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: IntrusiveNode + ?Sized> Default for IntrusiveList<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IntrusiveNode + ?Sized> IntrusiveList<T> {
    /// Cria uma nova lista intrusiva vazia.
    #[inline]
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Retorna a quantidade de nós atualmente na lista.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Retorna `true` se a lista estiver vazia.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insere o nó no final da lista ($O(1)$).
    ///
    /// # Panics
    /// Dispara pânico se o nó já estiver vinculado a alguma lista.
    pub fn push_back(&mut self, node: &T) {
        let link = node.intrusive_link();
        assert!(!link.is_linked.get(), "Tentativa de inserir nó já vinculado");

        let link_ptr = NonNull::from(link);
        link.is_linked.set(true);
        link.next.set(None);
        link.prev.set(self.tail);

        if let Some(mut old_tail) = self.tail {
            unsafe {
                old_tail.as_mut().next.set(Some(link_ptr));
            }
        } else {
            self.head = Some(link_ptr);
        }

        self.tail = Some(link_ptr);
        self.len += 1;
    }

    /// Insere o nó no início da lista ($O(1)$).
    ///
    /// # Panics
    /// Dispara pânico se o nó já estiver vinculado a alguma lista.
    pub fn push_front(&mut self, node: &T) {
        let link = node.intrusive_link();
        assert!(!link.is_linked.get(), "Tentativa de inserir nó já vinculado");

        let link_ptr = NonNull::from(link);
        link.is_linked.set(true);
        link.prev.set(None);
        link.next.set(self.head);

        if let Some(mut old_head) = self.head {
            unsafe {
                old_head.as_mut().prev.set(Some(link_ptr));
            }
        } else {
            self.tail = Some(link_ptr);
        }

        self.head = Some(link_ptr);
        self.len += 1;
    }

    /// Remove um nó arbitrário da lista em $O(1)$.
    ///
    /// Retorna `true` se o nó foi removido, ou `false` se não estava vinculado.
    pub fn remove(&mut self, node: &T) -> bool {
        let link = node.intrusive_link();
        if !link.is_linked.get() {
            return false;
        }

        let prev = link.prev.get();
        let next = link.next.get();

        if let Some(mut p) = prev {
            unsafe {
                p.as_mut().next.set(next);
            }
        } else {
            self.head = next;
        }

        if let Some(mut n) = next {
            unsafe {
                n.as_mut().prev.set(prev);
            }
        } else {
            self.tail = prev;
        }

        link.unlink_internal();
        self.len -= 1;
        true
    }

    /// Limpa toda a lista, desvinculando todos os nós.
    pub fn clear(&mut self) {
        let mut curr = self.head;
        while let Some(ptr) = curr {
            unsafe {
                let link = ptr.as_ref();
                curr = link.next.get();
                link.unlink_internal();
            }
        }
        self.head = None;
        self.tail = None;
        self.len = 0;
    }
}

impl<T: IntrusiveNode + ?Sized> Drop for IntrusiveList<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: IntrusiveNode + ?Sized> fmt::Debug for IntrusiveList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntrusiveList")
            .field("len", &self.len)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Task {
        id: u32,
        link: IntrusiveLink,
    }

    impl Task {
        fn new(id: u32) -> Self {
            Self {
                id,
                link: IntrusiveLink::new(),
            }
        }
    }

    impl IntrusiveNode for Task {
        fn intrusive_link(&self) -> &IntrusiveLink {
            &self.link
        }
    }

    #[test]
    fn test_intrusive_list_push_and_remove() {
        let mut list = IntrusiveList::<Task>::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        let t1 = Task::new(1);
        let t2 = Task::new(2);
        let t3 = Task::new(3);

        list.push_back(&t1);
        list.push_back(&t2);
        list.push_front(&t3); // Ordem: 3 -> 1 -> 2

        assert_eq!(list.len(), 3);
        assert_eq!(t1.id, 1);
        assert!(t1.link.is_linked());
        assert!(t2.link.is_linked());
        assert!(t3.link.is_linked());

        // Remove do meio
        assert!(list.remove(&t1));
        assert!(!t1.link.is_linked());
        assert_eq!(list.len(), 2);

        // Remove o início
        assert!(list.remove(&t3));
        assert_eq!(list.len(), 1);

        // Remove o final
        assert!(list.remove(&t2));
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());

        // Remoção dupla é segura (retorna false)
        assert!(!list.remove(&t2));
    }

    #[test]
    fn test_intrusive_list_clear() {
        let mut list = IntrusiveList::<Task>::new();
        let t1 = Task::new(1);
        let t2 = Task::new(2);

        list.push_back(&t1);
        list.push_back(&t2);
        assert_eq!(list.len(), 2);

        list.clear();
        assert_eq!(list.len(), 0);
        assert!(!t1.link.is_linked());
        assert!(!t2.link.is_linked());
    }
}
