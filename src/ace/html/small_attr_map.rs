//! Small Attribute Map para Elementos HTML
//!
//! Este módulo implementa um mapa híbrido que usa SmallVec para casos comuns
//! (poucos atributos) e HashMap apenas quando necessário (muitos atributos).
//!
//! Benefícios:
//! - Zero alocações para elementos com ≤4 atributos (caso comum)
//! - Transição automática para HashMap quando necessário
//! - Iteração mais rápida (dados contíguos no SmallVec)
//! - Menor pressão no garbage collector

use std::borrow::Cow;
use smallvec::{SmallVec, smallvec};
use crate::html::interner::StringId;

/// Número máximo de atributos armazenados inline antes de promover para HashMap
const SMALL_CAPACITY: usize = 4;

/// Par chave-valor para atributos
#[derive(Clone, Debug, PartialEq, Eq)]
struct AttributePair {
    key: StringId,
    value: StringId,
}

/// Mapa híbrido de atributos
#[derive(Clone, Debug)]
pub enum SmallAttributeMap {
    /// Inline storage para poucos atributos (caso comum)
    Small(SmallVec<[AttributePair; SMALL_CAPACITY]>),
    /// Heap storage para muitos atributos
    Large(Box<std::collections::HashMap<StringId, StringId>>),
}

impl Default for SmallAttributeMap {
    fn default() -> Self {
        Self::Small(SmallVec::new())
    }
}

impl SmallAttributeMap {
    /// Cria um novo mapa vazio
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria um mapa com capacidade inicial
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= SMALL_CAPACITY {
            Self::Small(SmallVec::with_capacity(capacity))
        } else {
            Self::Large(Box::new(std::collections::HashMap::with_capacity(capacity)))
        }
    }

    /// Insere ou atualiza um atributo
    ///
    /// # Complexidade
    /// - Small: O(n) onde n é o número de atributos
    /// - Large: O(1) amortizado
    #[inline]
    pub fn insert(&mut self, key: StringId, value: StringId) -> Option<StringId> {
        match self {
            SmallAttributeMap::Small(vec) => {
                // Busca linear no SmallVec
                for pair in vec.iter_mut() {
                    if pair.key == key {
                        let old_value = pair.value;
                        pair.value = value;
                        return Some(old_value);
                    }
                }

                // Verifica se precisa promover para HashMap
                if vec.len() >= SMALL_CAPACITY {
                    // Promove para HashMap
                    let mut map = std::collections::HashMap::with_capacity(vec.len() + 1);
                    
                    // Move todos os pares existentes
                    for pair in vec.drain(..) {
                        map.insert(pair.key, pair.value);
                    }
                    
                    // Insere o novo par
                    map.insert(key, value);
                    
                    *self = SmallAttributeMap::Large(Box::new(map));
                    return None;
                }

                // Adiciona ao SmallVec
                vec.push(AttributePair { key, value });
                None
            }
            SmallAttributeMap::Large(map) => {
                map.insert(key, value)
            }
        }
    }

    /// Obtém o valor de um atributo
    ///
    /// # Complexidade
    /// - Small: O(n) onde n é o número de atributos
    /// - Large: O(1) amortizado
    #[inline]
    pub fn get(&self, key: StringId) -> Option<StringId> {
        match self {
            SmallAttributeMap::Small(vec) => {
                vec.iter()
                    .find(|pair| pair.key == key)
                    .map(|pair| pair.value)
            }
            SmallAttributeMap::Large(map) => {
                map.get(&key).copied()
            }
        }
    }

    /// Remove um atributo
    ///
    /// # Nota
    /// Não reduz o tamanho do HashMap se estiver usando Large
    #[inline]
    pub fn remove(&mut self, key: StringId) -> Option<StringId> {
        match self {
            SmallAttributeMap::Small(vec) => {
                let mut i = 0;
                while i < vec.len() {
                    if vec[i].key == key {
                        return Some(vec.remove(i).value);
                    }
                    i += 1;
                }
                None
            }
            SmallAttributeMap::Large(map) => {
                map.remove(&key)
            }
        }
    }

    /// Verifica se contém um atributo
    #[inline]
    pub fn contains_key(&self, key: StringId) -> bool {
        self.get(key).is_some()
    }

    /// Retorna o número de atributos
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            SmallAttributeMap::Small(vec) => vec.len(),
            SmallAttributeMap::Large(map) => map.len(),
        }
    }

    /// Verifica se está vazio
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Limpa todos os atributos
    #[inline]
    pub fn clear(&mut self) {
        match self {
            SmallAttributeMap::Small(vec) => vec.clear(),
            SmallAttributeMap::Large(map) => map.clear(),
        }
    }

    /// Retorna true se estiver usando armazenamento Small
    #[inline]
    pub fn is_small(&self) -> bool {
        matches!(self, SmallAttributeMap::Small(_))
    }

    /// Retorna true se estiver usando armazenamento Large
    #[inline]
    pub fn is_large(&self) -> bool {
        matches!(self, SmallAttributeMap::Large(_))
    }

    /// Iterador sobre os atributos
    pub fn iter(&self) -> impl Iterator<Item = (StringId, StringId)> + '_ {
        SmallAttributeMapIter {
            inner: match self {
                SmallAttributeMap::Small(vec) => SmallAttributeMapIterInner::Small(vec.iter()),
                SmallAttributeMap::Large(map) => SmallAttributeMapIterInner::Large(map.iter()),
            },
        }
    }

    /// Estatísticas para profiling
    pub fn stats(&self) -> SmallAttributeMapStats {
        SmallAttributeMapStats {
            count: self.len(),
            is_small: self.is_small(),
            capacity: match self {
                SmallAttributeMap::Small(vec) => vec.capacity(),
                SmallAttributeMap::Large(map) => map.capacity(),
            },
        }
    }
}

impl PartialEq for SmallAttributeMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        match self {
            SmallAttributeMap::Small(vec) => {
                for pair in vec {
                    if other.get(pair.key) != Some(pair.value) {
                        return false;
                    }
                }
                true
            }
            SmallAttributeMap::Large(map) => {
                for (key, value) in map.iter() {
                    if other.get(*key) != Some(*value) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl Eq for SmallAttributeMap {}

/// Iterador sobre atributos
pub struct SmallAttributeMapIter<'a> {
    inner: SmallAttributeMapIterInner<'a>,
}

enum SmallAttributeMapIterInner<'a> {
    Small(std::slice::Iter<'a, AttributePair>),
    Large(std::collections::hash_map::Iter<'a, StringId, StringId>),
}

impl<'a> Iterator for SmallAttributeMapIter<'a> {
    type Item = (StringId, StringId);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            SmallAttributeMapIterInner::Small(iter) => {
                iter.next().map(|pair| (pair.key, pair.value))
            }
            SmallAttributeMapIterInner::Large(iter) => {
                iter.next().map(|(k, v)| (*k, *v))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            SmallAttributeMapIterInner::Small(iter) => iter.size_hint(),
            SmallAttributeMapIterInner::Large(iter) => iter.size_hint(),
        }
    }
}

/// Estatísticas do SmallAttributeMap
#[derive(Debug, Clone, Copy)]
pub struct SmallAttributeMapStats {
    pub count: usize,
    pub is_small: bool,
    pub capacity: usize,
}

// Implementações convenience para construção

impl FromIterator<(StringId, StringId)> for SmallAttributeMap {
    fn from_iter<T: IntoIterator<Item = (StringId, StringId)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<const N: usize> From<[(StringId, StringId); N]> for SmallAttributeMap {
    fn from(arr: [(StringId, StringId); N]) -> Self {
        if N <= SMALL_CAPACITY {
            let vec: SmallVec<[AttributePair; SMALL_CAPACITY]> = arr
                .into_iter()
                .map(|(key, value)| AttributePair { key, value })
                .collect();
            SmallAttributeMap::Small(vec)
        } else {
            let map: std::collections::HashMap<StringId, StringId> = arr.into_iter().collect();
            SmallAttributeMap::Large(Box::new(map))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::interner::StringInterner;

    fn setup_interner() -> StringInterner {
        StringInterner::new()
    }

    #[test]
    fn test_new_is_small() {
        let map = SmallAttributeMap::new();
        assert!(map.is_small());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_insert_and_get_small() {
        let interner = setup_interner();
        let key = interner.intern("class");
        let value = interner.intern("container");

        let mut map = SmallAttributeMap::new();
        assert_eq!(map.insert(key, value), None);
        assert_eq!(map.get(key), Some(value));
        assert_eq!(map.len(), 1);
        assert!(map.is_small());
    }

    #[test]
    fn test_insert_update() {
        let interner = setup_interner();
        let key = interner.intern("id");
        let value1 = interner.intern("foo");
        let value2 = interner.intern("bar");

        let mut map = SmallAttributeMap::new();
        assert_eq!(map.insert(key, value1), None);
        assert_eq!(map.insert(key, value2), Some(value1));
        assert_eq!(map.get(key), Some(value2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_promotion_to_large() {
        let interner = setup_interner();
        let mut map = SmallAttributeMap::new();

        // Insere 5 atributos (mais que SMALL_CAPACITY=4)
        for i in 0..5 {
            let key = interner.intern(&format!("attr{}", i));
            let value = interner.intern(&format!("val{}", i));
            map.insert(key, value);
        }

        assert!(map.is_large());
        assert_eq!(map.len(), 5);

        // Verifica que todos os valores estão presentes
        for i in 0..5 {
            let key = interner.intern(&format!("attr{}", i));
            let expected = interner.intern(&format!("val{}", i));
            assert_eq!(map.get(key), Some(expected));
        }
    }

    #[test]
    fn test_remove() {
        let interner = setup_interner();
        let key1 = interner.intern("class");
        let val1 = interner.intern("foo");
        let key2 = interner.intern("id");
        let val2 = interner.intern("bar");

        let mut map = SmallAttributeMap::new();
        map.insert(key1, val1);
        map.insert(key2, val2);

        assert_eq!(map.remove(key1), Some(val1));
        assert_eq!(map.get(key1), None);
        assert_eq!(map.get(key2), Some(val2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_contains_key() {
        let interner = setup_interner();
        let key = interner.intern("href");
        let value = interner.intern("https://example.com");

        let mut map = SmallAttributeMap::new();
        map.insert(key, value);

        assert!(map.contains_key(key));
        assert!(!map.contains_key(interner.intern("src")));
    }

    #[test]
    fn test_clear() {
        let interner = setup_interner();
        let mut map = SmallAttributeMap::new();

        for i in 0..3 {
            let key = interner.intern(&format!("attr{}", i));
            let value = interner.intern(&format!("val{}", i));
            map.insert(key, value);
        }

        assert_eq!(map.len(), 3);
        map.clear();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_iter() {
        let interner = setup_interner();
        let mut map = SmallAttributeMap::new();

        let pairs = vec![
            (interner.intern("class"), interner.intern("btn")),
            (interner.intern("id"), interner.intern("submit")),
            (interner.intern("type"), interner.intern("button")),
        ];

        for (k, v) in &pairs {
            map.insert(*k, *v);
        }

        let mut collected: Vec<_> = map.iter().collect();
        collected.sort_by_key(|(k, _)| *k);

        assert_eq!(collected.len(), 3);
        // Nota: a ordem pode variar, então verificamos conteúdo
        for (k, v) in &pairs {
            assert!(collected.iter().any(|(ck, cv)| ck == k && cv == v));
        }
    }

    #[test]
    fn test_from_array_small() {
        let interner = setup_interner();
        let arr = [
            (interner.intern("a"), interner.intern("1")),
            (interner.intern("b"), interner.intern("2")),
        ];

        let map: SmallAttributeMap = arr.into();
        assert!(map.is_small());
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_from_array_large() {
        let interner = setup_interner();
        let arr = [
            (interner.intern("a"), interner.intern("1")),
            (interner.intern("b"), interner.intern("2")),
            (interner.intern("c"), interner.intern("3")),
            (interner.intern("d"), interner.intern("4")),
            (interner.intern("e"), interner.intern("5")),
        ];

        let map: SmallAttributeMap = arr.into();
        assert!(map.is_large());
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn test_equality() {
        let interner = setup_interner();
        let k1 = interner.intern("class");
        let v1 = interner.intern("foo");
        let k2 = interner.intern("id");
        let v2 = interner.intern("bar");

        let mut map1 = SmallAttributeMap::new();
        map1.insert(k1, v1);
        map1.insert(k2, v2);

        let mut map2 = SmallAttributeMap::new();
        map2.insert(k2, v2); // Ordem diferente
        map2.insert(k1, v1);

        assert_eq!(map1, map2);
    }

    #[test]
    fn test_stats() {
        let interner = setup_interner();
        let mut map = SmallAttributeMap::new();

        let stats = map.stats();
        assert_eq!(stats.count, 0);
        assert!(stats.is_small);

        for i in 0..5 {
            let key = interner.intern(&format!("attr{}", i));
            let value = interner.intern(&format!("val{}", i));
            map.insert(key, value);
        }

        let stats = map.stats();
        assert_eq!(stats.count, 5);
        assert!(!stats.is_small);
        assert!(stats.capacity >= 5);
    }
}
