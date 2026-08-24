//! # LRU Cache de Alta Performance (Chromium base::LRUCache Pattern)
//!
//! Cache de tamanho limitado com política de descarte do item menos recentemente usado (Least Recently Used).
//! Implementado com tabela hash `FxHashMap` acoplada a uma lista duplamente encadeada indexada (sem ponteiros crus inseguros),
//! garantindo $O(1)$ amortizado para inserção, consulta, atualização de prioridade e descarte (eviction).

use rustc_hash::FxHashMap;
use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;

struct LruNode<K, V> {
    key: K,
    value: V,
    prev: Option<usize>,
    next: Option<usize>,
}

/// Cache com política de descarte LRU e capacidade máxima configurável.
pub struct LruCache<K, V> {
    map: FxHashMap<K, usize>,
    nodes: Vec<Option<LruNode<K, V>>>,
    free_slots: Vec<usize>,
    head: Option<usize>, // MRU (Most Recently Used)
    tail: Option<usize>, // LRU (Least Recently Used)
    capacity: usize,
}

impl<K: Clone + Hash + Eq, V> LruCache<K, V> {
    /// Cria um novo `LruCache` com a capacidade máxima especificada.
    ///
    /// # Panics
    /// Dispara pânico se `capacity == 0`.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacidade do LRU Cache deve ser maior que 0");
        Self {
            map: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            nodes: Vec::with_capacity(capacity),
            free_slots: Vec::new(),
            head: None,
            tail: None,
            capacity,
        }
    }

    /// Retorna a quantidade de elementos atualmente no cache.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Retorna `true` se o cache estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Retorna a capacidade máxima do cache.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Move o nó indicado para a cabeça (MRU) da lista.
    fn move_to_head(&mut self, idx: usize) {
        if self.head == Some(idx) {
            return;
        }

        // Desconecta de sua posição atual
        let (prev, next) = {
            let node = self.nodes[idx].as_ref().unwrap();
            (node.prev, node.next)
        };

        if let Some(p) = prev {
            self.nodes[p].as_mut().unwrap().next = next;
        }
        if let Some(n) = next {
            self.nodes[n].as_mut().unwrap().prev = prev;
        }
        if self.tail == Some(idx) {
            self.tail = prev;
        }

        // Insere na cabeça
        if let Some(old_head) = self.head {
            self.nodes[old_head].as_mut().unwrap().prev = Some(idx);
        }
        self.nodes[idx].as_mut().unwrap().next = self.head;
        self.nodes[idx].as_mut().unwrap().prev = None;
        self.head = Some(idx);

        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }

    /// Obtém uma referência ao valor e atualiza a sua prioridade LRU (move para MRU).
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_head(idx);
            Some(&self.nodes[idx].as_ref().unwrap().value)
        } else {
            None
        }
    }

    /// Obtém uma referência mutável ao valor e atualiza a sua prioridade LRU.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_head(idx);
            Some(&mut self.nodes[idx].as_mut().unwrap().value)
        } else {
            None
        }
    }

    /// Consulta um valor sem atualizar a sua prioridade LRU (*peek*).
    pub fn peek<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map
            .get(key)
            .and_then(|&idx| self.nodes[idx].as_ref().map(|n| &n.value))
    }

    /// Retorna `true` se a chave estiver presente no cache (sem atualizar LRU).
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }

    /// Insere um par `(chave, valor)` no cache.
    ///
    /// Se a chave já existir, atualiza o valor e a move para MRU.
    /// Se a capacidade for excedida, descarta o item LRU e retorna `Some((chave_descartada, valor_descartado))`.
    pub fn put(&mut self, key: K, value: V) -> Option<(K, V)> {
        if let Some(&idx) = self.map.get(&key) {
            self.nodes[idx].as_mut().unwrap().value = value;
            self.move_to_head(idx);
            return None;
        }

        let mut evicted = None;

        // Se estiver na capacidade máxima, remove o item da cauda (LRU)
        let slot = if self.len() >= self.capacity {
            let tail_idx = self.tail.expect("LRU tail ausente com len >= capacity");
            let tail_node = self.nodes[tail_idx].take().unwrap();
            self.map.remove(&tail_node.key);

            self.tail = tail_node.prev;
            if let Some(new_tail) = self.tail {
                self.nodes[new_tail].as_mut().unwrap().next = None;
            } else {
                self.head = None;
            }

            evicted = Some((tail_node.key, tail_node.value));
            tail_idx
        } else if let Some(free_idx) = self.free_slots.pop() {
            free_idx
        } else {
            let new_idx = self.nodes.len();
            self.nodes.push(None);
            new_idx
        };

        // Aloca o novo nó no slot
        self.nodes[slot] = Some(LruNode {
            key: key.clone(),
            value,
            prev: None,
            next: self.head,
        });

        if let Some(old_head) = self.head {
            self.nodes[old_head].as_mut().unwrap().prev = Some(slot);
        }
        self.head = Some(slot);
        if self.tail.is_none() {
            self.tail = Some(slot);
        }

        self.map.insert(key, slot);
        evicted
    }

    /// Remove um elemento específico do cache.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = self.map.remove(key)?;
        let node = self.nodes[idx].take().unwrap();

        if let Some(p) = node.prev {
            self.nodes[p].as_mut().unwrap().next = node.next;
        } else {
            self.head = node.next;
        }

        if let Some(n) = node.next {
            self.nodes[n].as_mut().unwrap().prev = node.prev;
        } else {
            self.tail = node.prev;
        }

        self.free_slots.push(idx);
        Some(node.value)
    }

    /// Remove e retorna o item menos recentemente usado (LRU tail).
    pub fn pop_lru(&mut self) -> Option<(K, V)> {
        let tail_idx = self.tail?;
        let node = self.nodes[tail_idx].take().unwrap();
        self.map.remove(&node.key);

        self.tail = node.prev;
        if let Some(new_tail) = self.tail {
            self.nodes[new_tail].as_mut().unwrap().next = None;
        } else {
            self.head = None;
        }

        self.free_slots.push(tail_idx);
        Some((node.key, node.value))
    }

    /// Limpa todo o cache.
    pub fn clear(&mut self) {
        self.map.clear();
        self.nodes.clear();
        self.free_slots.clear();
        self.head = None;
        self.tail = None;
    }
}

impl<K: fmt::Debug + Clone + Hash + Eq, V: fmt::Debug> fmt::Debug for LruCache<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LruCache")
            .field("len", &self.len())
            .field("capacity", &self.capacity)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_basic_put_get() {
        let mut cache = LruCache::new(2);
        assert_eq!(cache.len(), 0);

        assert_eq!(cache.put("a", 1), None);
        assert_eq!(cache.put("b", 2), None);
        assert_eq!(cache.len(), 2);

        assert_eq!(cache.get("a"), Some(&1)); // "a" vira MRU, "b" é LRU
        assert_eq!(cache.put("c", 3), Some(("b", 2))); // "b" deve ser descartado!

        assert_eq!(cache.get("b"), None);
        assert_eq!(cache.get("a"), Some(&1));
        assert_eq!(cache.get("c"), Some(&3));
    }

    #[test]
    fn test_lru_peek_and_remove() {
        let mut cache = LruCache::new(3);
        cache.put(1, "one");
        cache.put(2, "two");
        cache.put(3, "three");

        assert_eq!(cache.peek(&1), Some(&"one"));
        assert_eq!(cache.remove(&2), Some("two"));
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains_key(&2));

        // Inserção após remoção reusa slot
        cache.put(4, "four");
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&4), Some(&"four"));
    }

    #[test]
    fn test_lru_pop_lru() {
        let mut cache = LruCache::new(2);
        cache.put("x", 10);
        cache.put("y", 20);

        assert_eq!(cache.pop_lru(), Some(("x", 10)));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.pop_lru(), Some(("y", 20)));
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.pop_lru(), None);
    }
}
