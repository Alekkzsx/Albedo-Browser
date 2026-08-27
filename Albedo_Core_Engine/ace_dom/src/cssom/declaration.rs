//! # CSSStyleDeclaration (WHATWG CSSOM §6.4.1)
//!
//! Estrutura compacta para armazenamento de propriedades CSS com `InlineVec<CSSProperty, 8>`
//! (zero alocação no heap para estilos inline de até 8 propriedades) e parsing de alto desempenho.

use ace_core::collections::InlineVec;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Uma declaração de propriedade CSS (ex: `color: red !important;`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSSProperty {
    pub name: Atom,
    pub value: SmolStr,
    pub important: bool,
}

impl CSSProperty {
    pub fn new(name: impl Into<Atom>, value: impl Into<SmolStr>, important: bool) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            important,
        }
    }
}

/// Representação de uma lista de declarações CSS (Blink `CSSPropertyValueSet` pattern).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CSSStyleDeclaration {
    pub properties: InlineVec<CSSProperty, 8>,
}

impl CSSStyleDeclaration {
    /// Cria uma nova declaração de estilo vazia.
    pub fn new() -> Self {
        Self {
            properties: InlineVec::new(),
        }
    }

    /// Faz o parsing de uma string de estilo em linha (ex: `color: #fff; font-size: 16px;`).
    pub fn parse(css_text: &str) -> Self {
        let mut decl = Self::new();
        for chunk in css_text.split(';') {
            let trimmed = chunk.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((name_part, val_part)) = trimmed.split_once(':') {
                let name = name_part.trim().to_ascii_lowercase();
                let mut val = val_part.trim();
                let mut important = false;
                if let Some(stripped) = val.strip_suffix("!important") {
                    val = stripped.trim();
                    important = true;
                }
                if !name.is_empty() && !val.is_empty() {
                    decl.set_property(name.as_str(), val, important);
                }
            }
        }
        decl
    }

    /// Retorna o valor de uma propriedade CSS se definida.
    pub fn get_property_value(&self, prop_name: &str) -> Option<&str> {
        self.properties
            .as_slice()
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(prop_name))
            .map(|p| p.value.as_str())
    }

    /// Retorna a prioridade (`"important"` ou `""`) de uma propriedade CSS.
    pub fn get_property_priority(&self, prop_name: &str) -> &'static str {
        self.properties
            .as_slice()
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(prop_name))
            .map(|p| if p.important { "important" } else { "" })
            .unwrap_or("")
    }

    /// Define ou atualiza uma propriedade CSS.
    pub fn set_property(
        &mut self,
        prop_name: impl Into<Atom>,
        value: impl Into<SmolStr>,
        important: bool,
    ) {
        let name_atom = prop_name.into();
        let value_smol = value.into();

        if let Some(existing) = self
            .properties
            .as_mut_slice()
            .iter_mut()
            .find(|p| p.name == name_atom)
        {
            existing.value = value_smol;
            existing.important = important;
        } else {
            self.properties.push(CSSProperty {
                name: name_atom,
                value: value_smol,
                important,
            });
        }
    }

    /// Remove uma propriedade CSS se ela existir, retornando o valor antigo.
    pub fn remove_property(&mut self, prop_name: &str) -> Option<SmolStr> {
        let pos = self
            .properties
            .as_slice()
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(prop_name))?;
        Some(self.properties.remove(pos).value)
    }

    /// Retorna o número de propriedades declaradas.
    #[inline]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Retorna `true` se não houver propriedades declaradas.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Serializa as propriedades em formato CSS normativo (`cssText`).
    pub fn css_text(&self) -> String {
        let mut out = String::new();
        for (i, prop) in self.properties.as_slice().iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(prop.name.as_str());
            out.push_str(": ");
            out.push_str(prop.value.as_str());
            if prop.important {
                out.push_str(" !important");
            }
            out.push(';');
        }
        out
    }
}
