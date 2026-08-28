//! # Manipulador de Lista de Tokens DOM (DOMTokenList / classList)
//!
//! Interface viva para manipulação de classes CSS em um `ElementData`.

use crate::node::ElementData;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Estrutura para manipulação ergonômica e sincronizada das classes de um elemento DOM.
pub struct DOMTokenList<'a> {
    element: &'a mut ElementData,
}

impl<'a> DOMTokenList<'a> {
    /// Cria uma nova visão `DOMTokenList` sobre o `ElementData`.
    pub fn new(element: &'a mut ElementData) -> Self {
        Self { element }
    }

    /// Retorna `true` se a classe estiver presente na lista.
    #[inline]
    pub fn contains(&self, token: &str) -> bool {
        self.element.has_class(token)
    }

    /// Adiciona uma nova classe à lista se ainda não estiver presente.
    pub fn add(&mut self, token: &str) {
        if token.is_empty() || token.contains(char::is_whitespace) {
            return;
        }

        if !self.contains(token) {
            self.element.classes.push(Atom::new(token));
            self.sync_attribute();
        }
    }

    /// Remove uma classe da lista.
    pub fn remove(&mut self, token: &str) {
        let before_len = self.element.classes.len();
        self.element.classes.retain(|c| !c.eq_ignore_ascii_case(token));
        if self.element.classes.len() < before_len {
            self.sync_attribute();
        }
    }

    /// Alterna a presença de uma classe na lista (adiciona se ausente, remove se presente).
    /// Retorna `true` se a classe foi adicionada, `false` se foi removida.
    pub fn toggle(&mut self, token: &str) -> bool {
        if self.contains(token) {
            self.remove(token);
            false
        } else {
            self.add(token);
            true
        }
    }

    /// Substitui `old_token` por `new_token`. Retorna `true` se `old_token` foi encontrado e substituído.
    pub fn replace(&mut self, old_token: &str, new_token: &str) -> bool {
        if old_token.is_empty() || new_token.is_empty() || new_token.contains(char::is_whitespace) {
            return false;
        }

        let mut replaced = false;
        for c in self.element.classes.as_mut_slice() {
            if c.eq_ignore_ascii_case(old_token) {
                *c = Atom::new(new_token);
                replaced = true;
                break;
            }
        }

        if replaced {
            self.sync_attribute();
        }
        replaced
    }

    /// Retorna a representação serializada em string separada por espaços.
    pub fn value(&self) -> String {
        let mut out = String::new();
        for (i, c) in self.element.classes.as_slice().iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(c.as_str());
        }
        out
    }

    /// Sincroniza o atributo `class` no vetor de atributos com o cache de classes.
    fn sync_attribute(&mut self) {
        let val = self.value();
        let class_atom = Atom::new("class");
        for attr in self.element.attributes.as_mut_slice() {
            if attr.name == class_atom {
                attr.value = SmolStr::new(&val);
                return;
            }
        }
        if !val.is_empty() {
            self.element.attributes.push(crate::node::Attribute {
                name: class_atom,
                value: SmolStr::new(val),
            });
        }
    }
}
