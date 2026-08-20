//! # FlatMap e FlatSet Contíguos de Alta Performance (Chromium base::flat_map Pattern)
//!
//! Estrutura de dados baseada em vetor contíguo e ordenado para coleções de tamanho pequeno a médio ($N \le 64$).
//! Por manter todos os pares `(Chave, Valor)` adjacentes na memória (L1/L2 cache locality) e realizar
//! busca binária $O(\log N)$, supera tabelas hash e árvores binárias eliminando ponteiros e overhead de alocação de nós.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Index;

/// Um mapa associativo ordenado e contíguo na memória.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlatMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> Default for FlatMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> FlatMap<K, V> {
    /// Cria um novo `FlatMap` vazio sem alocação inicial.
    #[inline]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Cria um novo `FlatMap` com capacidade pré-alocada.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Retorna o número de elementos contidos no mapa.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Retorna `true` se o mapa estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retorna a capacidade atual do vetor subjacente.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    /// Reserva capacidade para pelo menos `additional` novos elementos.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }

    /// Remove todos os elementos do mapa, mantendo a capacidade alocada.
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Reduz a capacidade do vetor interno para coincidir com o tamanho atual.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
    }

    /// Retorna uma fatia de tuplas `(K, V)` ordenadas por chave.
    #[inline]
    pub fn as_slice(&self) -> &[(K, V)] {
        &self.entries
    }

    /// Itera por referências imutáveis aos pares `(&K, &V)` em ordem crescente de chave.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Itera por referências mutáveis aos valores `(&K, &mut V)` em ordem crescente de chave.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.entries.iter_mut().map(|(k, v)| (&*k, v))
    }

    /// Itera pelas chaves ordenadas do mapa.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Itera pelos valores do mapa.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Itera pelos valores mutáveis do mapa.
    #[inline]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| v)
    }

    /// Consome o mapa e retorna o vetor interno ordenado.
    #[inline]
    pub fn into_inner(self) -> Vec<(K, V)> {
        self.entries
    }
}

impl<K: Ord, V> FlatMap<K, V> {
    /// Insere um par `(chave, valor)` no mapa.
    /// Se a chave já existir, atualiza o valor e retorna o valor antigo `Some(antigo)`.
    /// Caso contrário, insere na posição ordenada e retorna `None`.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.entries.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(idx) => {
                let old = std::mem::replace(&mut self.entries[idx].1, value);
                Some(old)
            }
            Err(idx) => {
                self.entries.insert(idx, (key, value));
                None
            }
        }
    }

    /// Retorna uma referência ao valor associado à chave, se existir.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.entries.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
            Ok(idx) => Some(&self.entries[idx].1),
            Err(_) => None,
        }
    }

    /// Retorna uma referência mutável ao valor associado à chave, se existir.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.entries.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
            Ok(idx) => Some(&mut self.entries[idx].1),
            Err(_) => None,
        }
    }

    /// Retorna `true` se a chave especificada estiver presente no mapa.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.entries
            .binary_search_by(|(k, _)| k.borrow().cmp(key))
            .is_ok()
    }

    /// Remove a chave do mapa e retorna o valor associado, se existia.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.entries.binary_search_by(|(k, _)| k.borrow().cmp(key)) {
            Ok(idx) => Some(self.entries.remove(idx).1),
            Err(_) => None,
        }
    }

    /// Retém apenas os elementos que satisfazem o predicado.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.entries.retain_mut(|(k, v)| f(k, v));
    }
}

impl<K: Ord, V, Q> Index<&Q> for FlatMap<K, V>
where
    K: Borrow<Q>,
    Q: Ord + ?Sized,
{
    type Output = V;

    #[inline]
    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).expect("Chave não encontrada no FlatMap")
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for FlatMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.entries.iter().map(|(k, v)| (k, v)))
            .finish()
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for FlatMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K: Ord, V> Extend<(K, V)> for FlatMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

/// Um conjunto ordenado e contíguo na memória (Set ordenado em flat vector).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlatSet<T> {
    elements: Vec<T>,
}

impl<T> Default for FlatSet<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FlatSet<T> {
    /// Cria um novo `FlatSet` vazio.
    #[inline]
    pub const fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Cria um novo `FlatSet` com capacidade pré-alocada.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
        }
    }

    /// Retorna o número de elementos no conjunto.
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Retorna `true` se o conjunto estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Limpa o conjunto, preservando a capacidade.
    #[inline]
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Retorna uma fatia dos elementos ordenados.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.elements
    }

    /// Itera por referências aos elementos em ordem crescente.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.elements.iter()
    }

    /// Consome o conjunto e retorna o vetor ordenado.
    #[inline]
    pub fn into_inner(self) -> Vec<T> {
        self.elements
    }
}

impl<T: Ord> FlatSet<T> {
    /// Insere um elemento no conjunto.
    /// Retorna `true` se o elemento foi inserido (não existia previamente), ou `false` se já existia.
    pub fn insert(&mut self, value: T) -> bool {
        match self.elements.binary_search(&value) {
            Ok(_) => false,
            Err(idx) => {
                self.elements.insert(idx, value);
                true
            }
        }
    }

    /// Retorna `true` se o conjunto contém o valor especificado.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.elements
            .binary_search_by(|x| x.borrow().cmp(value))
            .is_ok()
    }

    /// Remove um elemento do conjunto. Retorna `true` se o elemento estava presente.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.elements.binary_search_by(|x| x.borrow().cmp(value)) {
            Ok(idx) => {
                self.elements.remove(idx);
                true
            }
            Err(_) => false,
        }
    }

    /// Retém apenas os elementos que satisfazem o predicado.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.elements.retain(f);
    }
}

impl<T: fmt::Debug> fmt::Debug for FlatSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.elements.iter()).finish()
    }
}

impl<T: Ord> FromIterator<T> for FlatSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::new();
        for item in iter {
            set.insert(item);
        }
        set
    }
}

impl<T: Ord> Extend<T> for FlatSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.insert(item);
        }
    }
}

impl<'a, T> IntoIterator for &'a FlatSet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl<T> IntoIterator for FlatSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_map_insert_and_get() {
        let mut map = FlatMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        assert_eq!(map.insert("z", 10), None);
        assert_eq!(map.insert("a", 20), None);
        assert_eq!(map.insert("m", 30), None);

        assert_eq!(map.len(), 3);
        assert_eq!(map.get("a"), Some(&20));
        assert_eq!(map.get("m"), Some(&30));
        assert_eq!(map.get("z"), Some(&10));
        assert_eq!(map.get("nonexistent"), None);

        // Chaves devem estar ordenadas
        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys, vec!["a", "m", "z"]);

        // Sobrescrita
        assert_eq!(map.insert("m", 99), Some(30));
        assert_eq!(map.get("m"), Some(&99));
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_flat_map_remove_and_index() {
        let mut map = FlatMap::new();
        map.insert(1, "one");
        map.insert(2, "two");
        map.insert(3, "three");

        assert_eq!(map[&2], "two");
        assert_eq!(map.remove(&2), Some("two"));
        assert_eq!(map.remove(&2), None);
        assert_eq!(map.len(), 2);
        assert!(!map.contains_key(&2));
    }

    #[test]
    fn test_flat_set_operations() {
        let mut set = FlatSet::new();
        assert!(set.insert(50));
        assert!(set.insert(10));
        assert!(set.insert(30));
        assert!(!set.insert(10)); // Duplicata

        assert_eq!(set.len(), 3);
        assert!(set.contains(&10));
        assert!(set.contains(&30));
        assert!(set.contains(&50));
        assert!(!set.contains(&99));

        let items: Vec<_> = set.iter().copied().collect();
        assert_eq!(items, vec![10, 30, 50]);

        assert!(set.remove(&30));
        assert!(!set.remove(&30));
        assert_eq!(set.len(), 2);
    }
}
