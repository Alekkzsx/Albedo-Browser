//! # Registro e Validação de Elementos Customizados (WHATWG HTML §4.13)
//!
//! Implementação da validação estrita de nomes de tags customizadas e registro de definições.

use crate::error::DomError;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Valida se um nome de tag é um nome de elemento customizado válido (WHATWG §4.13.1.2).
pub fn is_valid_custom_element_name(name: &str) -> bool {
    // 1. Deve começar com letra minúscula ASCII
    let mut chars = name.chars();
    match chars.next() {
        Some('a'..='z') => {}
        _ => return false,
    }

    // 2. Deve conter pelo menos um hífen '-'
    if !name.contains('-') {
        return false
    }

    // 3. Não pode ser um nome reservado
    !matches!(
        name,
        "annotation-xml"
            | "color-profile"
            | "font-face"
            | "font-face-src"
            | "font-face-uri"
            | "font-face-format"
            | "font-face-name"
            | "missing-glyph"
    )
}

/// Definição de um elemento customizado registrado no documento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomElementDefinition {
    /// Nome da tag do elemento customizado (ex: `user-card`).
    pub name: Atom,
    /// Lista de atributos observados que disparam `attributeChangedCallback`.
    pub observed_attributes: Vec<Atom>,
}

/// Registro oficial de elementos customizados (`customElements`).
#[derive(Debug, Default)]
pub struct CustomElementRegistry {
    definitions: FxHashMap<Atom, CustomElementDefinition>,
}

impl CustomElementRegistry {
    /// Cria um novo registro de elementos customizados.
    pub fn new() -> Self {
        Self {
            definitions: FxHashMap::default(),
        }
    }

    /// Registra uma nova definição de elemento customizado no registro.
    pub fn define(&mut self, name: &str, observed_attrs: &[&str]) -> Result<(), DomError> {
        if !is_valid_custom_element_name(name) {
            return Err(DomError::HierarchyRequestError(format!(
                "'{}' não é um nome válido de elemento customizado",
                name
            )));
        }

        let atom_name = Atom::new(name);
        if self.definitions.contains_key(&atom_name) {
            return Err(DomError::HierarchyRequestError(format!(
                "Elemento customizado '{}' já está registrado",
                name
            )));
        }

        let attrs = observed_attrs.iter().map(|&a| Atom::new(a)).collect();
        self.definitions.insert(
            atom_name.clone(),
            CustomElementDefinition {
                name: atom_name,
                observed_attributes: attrs,
            },
        );

        Ok(())
    }

    /// Retorna a definição de um elemento customizado registrado pelo nome da tag.
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition> {
        let atom = Atom::new(name);
        self.definitions.get(&atom)
    }

    /// Retorna o número de elementos registrados.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Retorna `true` se não houver elementos registrados.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}
