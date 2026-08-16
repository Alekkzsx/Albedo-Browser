//! # O(1) String Pooling
//!
//! A comparação de strings é o calcanhar de Aquiles de qualquer parser HTML/CSS.
//! No Albedo, nós representamos tags (`div`, `span`) e propriedades CSS (`color`)
//! como "Atoms" (átomos).
//! 
//! Quando um Atom é criado, a string é internada em um pool global (ou estático no compile-time).
//! Comparar dois Atoms custa 1 ciclo de CPU, pois apenas o endereço de memória/ID é comparado.

use string_cache::DefaultAtom;

/// Representa uma string otimizada e única em toda a execução do navegador.
/// A igualdade `Atom == Atom` é $O(1)$.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Atom(DefaultAtom);

impl Atom {
    /// Interna uma string dinamicamente em tempo de execução.
    #[inline]
    pub fn new(text: &str) -> Self {
        Self(DefaultAtom::from(text))
    }

    /// Retorna a representação textual do Atom.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Atom {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl AsRef<str> for Atom {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}


