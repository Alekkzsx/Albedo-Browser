//! # Dados de Elementos e Atributos DOM
//!
//! Estruturas de alta performance com `Atom` para nomes e `InlineVec<Attribute, 4>`
//! para armazenamento local de atributos sem alocações no heap em 95%+ dos nós.

use crate::node::token_list::DOMTokenList;
use ace_core::collections::InlineVec;
use ace_core::id::NodeId;
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

/// Dados raros ou de componentes específicos alocados sob demanda (Blink/WebKit ElementRareData pattern).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElementRareData {
    pub shadow_root: Option<NodeId>,
    pub template_content: Option<NodeId>,
    pub custom_element_definition: Option<Atom>,
    pub custom_element_state: crate::custom_elements::CustomElementState,
    pub is_value: Option<Atom>,
    pub form_owner: Option<NodeId>,
    pub inline_style: Option<SmolStr>,
    pub inline_style_decl: Option<Box<crate::cssom::CSSStyleDeclaration>>,
    pub aria_role: Option<Atom>,
}

/// Dados específicos de um elemento DOM (`NodeKind::Element`).
///
/// Otimizado para densidade de cache L1/L2: campos comuns residem no layout plano (48B),
/// enquanto campos raros (Shadow DOM, templates, custom elements) são delegados ao `ElementRareData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: Atom,
    pub namespace: Namespace,
    pub attributes: InlineVec<Attribute, 4>,
    pub id_attr: Option<Atom>,
    pub classes: InlineVec<Atom, 4>,
    pub rare_data: Option<Box<ElementRareData>>,
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
            rare_data: None,
        }
    }

    /// Retorna o `shadow_root` se anexado neste elemento.
    #[inline]
    pub fn shadow_root(&self) -> Option<NodeId> {
        self.rare_data.as_ref().and_then(|r| r.shadow_root)
    }

    /// Define o `shadow_root` para este elemento.
    #[inline]
    pub fn set_shadow_root(&mut self, shadow_root: Option<NodeId>) {
        if shadow_root.is_some() || self.rare_data.is_some() {
            self.ensure_rare_data().shadow_root = shadow_root;
        }
    }

    /// Retorna o `template_content` se este for um elemento `<template>`.
    #[inline]
    pub fn template_content(&self) -> Option<NodeId> {
        self.rare_data.as_ref().and_then(|r| r.template_content)
    }

    /// Define o `template_content` para este elemento `<template>`.
    #[inline]
    pub fn set_template_content(&mut self, template_content: Option<NodeId>) {
        if template_content.is_some() || self.rare_data.is_some() {
            self.ensure_rare_data().template_content = template_content;
        }
    }

    /// Garante e retorna a estrutura `ElementRareData` alocada sob demanda.
    #[inline]
    pub fn ensure_rare_data(&mut self) -> &mut ElementRareData {
        self.rare_data.get_or_insert_with(Box::default)
    }

    /// Retorna uma referência a `ElementRareData`, se existir.
    #[inline]
    pub fn rare_data(&self) -> Option<&ElementRareData> {
        self.rare_data.as_deref()
    }

    /// Retorna uma referência mutável a `ElementRareData`, se existir.
    #[inline]
    pub fn rare_data_mut(&mut self) -> Option<&mut ElementRareData> {
        self.rare_data.as_deref_mut()
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

        // 3. Atualiza cache de estilos CSS inline
        if name_atom.eq_ignore_ascii_case("style") {
            let decl = crate::cssom::CSSStyleDeclaration::parse(&value_str);
            self.ensure_rare_data().inline_style_decl = Some(Box::new(decl));
            self.ensure_rare_data().inline_style = Some(value_str.clone());
        }

        // 4. Insere ou substitui na lista de atributos
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

    /// Retorna `true` se o elemento contiver o atributo especificado.
    #[inline]
    pub fn has_attribute(&self, name: &str) -> bool {
        self.get_attribute(name).is_some()
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
            } else if name.eq_ignore_ascii_case("style") {
                if let Some(rare) = self.rare_data.as_mut() {
                    rare.inline_style = None;
                    rare.inline_style_decl = None;
                }
            }
        }

        removed
    }

    /// Retorna uma referência à declaração de estilo inline (`CSSStyleDeclaration`), se existir.
    pub fn style(&self) -> Option<&crate::cssom::CSSStyleDeclaration> {
        self.rare_data.as_ref().and_then(|r| r.inline_style_decl.as_deref())
    }

    /// Retorna uma referência mutável à declaração de estilo inline, criando-a sob demanda se necessário.
    pub fn style_mut(&mut self) -> &mut crate::cssom::CSSStyleDeclaration {
        let rare = self.ensure_rare_data();
        if rare.inline_style_decl.is_none() {
            rare.inline_style_decl = Some(Box::default());
        }
        rare.inline_style_decl.as_deref_mut().unwrap()
    }

    /// Retorna `true` se o elemento contiver a classe especificada.
    #[inline]
    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes.as_slice().iter().any(|c| c.eq_ignore_ascii_case(class_name))
    }

    /// Retorna o estado do elemento customizado (WHATWG §4.13.2).
    pub fn custom_element_state(&self) -> crate::custom_elements::CustomElementState {
        if let Some(rare) = self.rare_data.as_ref() {
            rare.custom_element_state
        } else if self.namespace == Namespace::Html
            && crate::custom_elements::is_valid_custom_element_name(self.tag_name.as_str())
        {
            crate::custom_elements::CustomElementState::Undefined
        } else {
            crate::custom_elements::CustomElementState::Uncustomized
        }
    }

    /// Define o estado do elemento customizado.
    pub fn set_custom_element_state(&mut self, state: crate::custom_elements::CustomElementState) {
        if state != crate::custom_elements::CustomElementState::Uncustomized || self.rare_data.is_some() {
            self.ensure_rare_data().custom_element_state = state;
        }
    }

    /// Retorna o nome da definição do elemento customizado associado.
    pub fn custom_element_definition(&self) -> Option<Atom> {
        self.rare_data.as_ref().and_then(|r| r.custom_element_definition.clone())
    }

    /// Define o nome da definição do elemento customizado associado.
    pub fn set_custom_element_definition(&mut self, def: Option<Atom>) {
        if def.is_some() || self.rare_data.is_some() {
            self.ensure_rare_data().custom_element_definition = def;
        }
    }

    /// Retorna o valor do atributo `is` para elementos customizados estendidos.
    pub fn is_value(&self) -> Option<Atom> {
        self.rare_data.as_ref().and_then(|r| r.is_value.clone())
    }

    /// Define o valor do atributo `is`.
    pub fn set_is_value(&mut self, is_value: Option<Atom>) {
        if is_value.is_some() || self.rare_data.is_some() {
            self.ensure_rare_data().is_value = is_value;
        }
    }

    /// Adiciona ou substitui um atributo, enfileirando reações se for um elemento customizado com atributo observado.
    pub fn set_attribute_with_reaction(
        &mut self,
        node_id: NodeId,
        name: impl Into<Atom>,
        value: impl Into<SmolStr>,
        reactions: &mut crate::custom_elements::CustomElementReactionsStack,
        registry: &crate::custom_elements::CustomElementRegistry,
    ) {
        let name_atom: Atom = name.into();
        let value_str: SmolStr = value.into();
        let old_value = self.get_attribute(name_atom.as_str()).map(SmolStr::new);

        let value_changed = old_value.as_ref() != Some(&value_str);

        self.set_attribute(name_atom.clone(), value_str.clone());

        if value_changed && self.custom_element_state() == crate::custom_elements::CustomElementState::Custom {
            let def_name = self.custom_element_definition().unwrap_or_else(|| self.tag_name.clone());
            if let Some(def) = registry.get(def_name.as_str()) {
                if def.observes_attribute(name_atom.as_str()) {
                    reactions.enqueue_attribute_changed(
                        node_id,
                        name_atom.as_str(),
                        old_value,
                        Some(value_str),
                    );
                }
            }
        }
    }

    /// Remove um atributo, enfileirando reação se for um elemento customizado com atributo observado.
    pub fn remove_attribute_with_reaction(
        &mut self,
        node_id: NodeId,
        name: &str,
        reactions: &mut crate::custom_elements::CustomElementReactionsStack,
        registry: &crate::custom_elements::CustomElementRegistry,
    ) -> bool {
        let old_value = self.get_attribute(name).map(SmolStr::new);
        let removed = self.remove_attribute(name);

        if removed && self.custom_element_state() == crate::custom_elements::CustomElementState::Custom {
            let def_name = self.custom_element_definition().unwrap_or_else(|| self.tag_name.clone());
            if let Some(def) = registry.get(def_name.as_str()) {
                if def.observes_attribute(name) {
                    reactions.enqueue_attribute_changed(
                        node_id,
                        name,
                        old_value,
                        None,
                    );
                }
            }
        }

        removed
    }

    /// Retorna a interface mutável `DOMStringMap` para manipulação de atributos `data-*`.
    pub fn dataset_mut(&mut self) -> crate::node::dataset::DOMStringMap<'_> {
        crate::node::dataset::DOMStringMap::new(self)
    }
}

