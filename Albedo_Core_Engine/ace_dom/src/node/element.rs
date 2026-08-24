//! # Dados de Elementos e Atributos DOM
//!
//! Estruturas de alta performance com `Atom` para nomes e `InlineVec<Attribute, 4>`
//! para armazenamento local de atributos sem alocações no heap em 95%+ dos nós.

use ace_core::collections::InlineVec;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Namespace de um elemento DOM (HTML, SVG ou MathML).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Namespace {
    #[default]
    Html,
    Svg,
    MathMl,
}

impl Namespace {
    pub const fn uri(self) -> &'static str {
        match self {
            Self::Html => "http://www.w3.org/1999/xhtml",
            Self::Svg => "http://www.w3.org/2000/svg",
            Self::MathMl => "http://www.w3.org/1998/Math/MathML",
        }
    }
}

/// Um atributo HTML/DOM composto por nome e valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: Atom,
    pub value: SmolStr,
}

impl Attribute {
    #[inline]
    pub fn new(name: impl Into<Atom>, value: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

use crate::node::token_list::DOMTokenList;
use ace_core::id::NodeId;

/// Dados específicos de um elemento DOM (`NodeKind::Element`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: Atom,
    pub namespace: Namespace,
    pub attributes: InlineVec<Attribute, 4>,
    pub id_attr: Option<Atom>,
    pub classes: InlineVec<Atom, 4>,
    pub shadow_root: Option<NodeId>,
    pub template_content: Option<NodeId>,
}

impl ElementData {
    /// Cria um novo `ElementData` com tag e namespace informados.
    pub fn new(tag_name: impl Into<Atom>, namespace: Namespace) -> Self {
        Self {
            tag_name: tag_name.into(),
            namespace,
            attributes: InlineVec::new(),
            id_attr: None,
            classes: InlineVec::new(),
            shadow_root: None,
            template_content: None,
        }
    }

    /// Retorna um manipulador `DOMTokenList` para mutação conveniente e viva das classes.
    pub fn class_list(&mut self) -> DOMTokenList<'_> {
        DOMTokenList::new(self)
    }

    /// Adiciona ou substitui um atributo, atualizando automaticamente os caches de ID e Classes.
    pub fn set_attribute(&mut self, name: impl Into<Atom>, value: impl Into<SmolStr>) {
        let name_atom: Atom = name.into();
        let value_str: SmolStr = value.into();

        // 1. Atualiza cache de ID rápido
        if name_atom.eq_ignore_ascii_case("id") {
            if value_str.is_empty() {
                self.id_attr = None;
            } else {
                self.id_attr = Some(Atom::new(&value_str));
            }
        }

        // 2. Atualiza cache de Classes rápido para a cascata e AncestorFilter
        if name_atom.eq_ignore_ascii_case("class") {
            self.classes.clear();
            for class_token in value_str.split_whitespace() {
                self.classes.push(Atom::new(class_token));
            }
        }

        // 3. Insere ou substitui na lista de atributos
        for attr in self.attributes.as_mut_slice() {
            if attr.name == name_atom {
                attr.value = value_str;
                return;
            }
        }

        self.attributes.push(Attribute {
            name: name_atom,
            value: value_str,
        });
    }

    /// Obtém o valor de um atributo pelo nome (case-insensitive para HTML).
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .as_slice()
            .iter()
            .find(|attr| attr.name.eq_ignore_ascii_case(name))
            .map(|attr| attr.value.as_str())
    }

    /// Remove um atributo pelo nome.
    pub fn remove_attribute(&mut self, name: &str) -> bool {
        let before_len = self.attributes.len();
        self.attributes.retain(|attr| !attr.name.eq_ignore_ascii_case(name));
        let removed = self.attributes.len() < before_len;

        if removed {
            if name.eq_ignore_ascii_case("id") {
                self.id_attr = None;
            } else if name.eq_ignore_ascii_case("class") {
                self.classes.clear();
            }
        }

        removed
    }

    /// Retorna `true` se o elemento contiver a classe especificada.
    #[inline]
    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes.as_slice().iter().any(|c| c.eq_ignore_ascii_case(class_name))
    }

    /// Retorna a interface mutável `DOMStringMap` para manipulação de atributos `data-*`.
    pub fn dataset_mut(&mut self) -> crate::node::dataset::DOMStringMap<'_> {
        crate::node::dataset::DOMStringMap::new(self)
    }
}
