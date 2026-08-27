//! # Registro e Validação de Elementos Customizados (WHATWG HTML §4.13)
//!
//! Implementação normativa da validação PCENChar (WHATWG §4.13.1.2), ciclo de vida,
//! registro de definições, resolução de `whenDefined` e upgrade de nós na árvore DOM.

use crate::custom_elements::reaction_stack::CustomElementReactionsStack;
use crate::error::DomError;
use crate::node::NodeData;
use ace_core::arena::{Arena, ArenaId};
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

/// Valida se um caractere pertence ao conjunto PCENChar conforme WHATWG §4.13.1.2.
#[inline]
pub fn is_pcen_char(c: char) -> bool {
    matches!(
        c,
        '-' | '.'
            | '0'..='9'
            | '_'
            | 'a'..='z'
            | '\u{00B7}'
            | '\u{00C0}'..='\u{00D6}'
            | '\u{00D8}'..='\u{00F6}'
            | '\u{00F8}'..='\u{037D}'
            | '\u{037F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{203F}'..='\u{2040}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}'
    )
}

/// Valida se um nome de tag é um nome de elemento customizado válido (WHATWG §4.13.1.2).
///
/// Regras normativas:
/// 1. Deve iniciar com letra minúscula ASCII `[a-z]`.
/// 2. Deve conter pelo menos um hífen `-`.
/// 3. Todos os caracteres devem pertencer ao conjunto `PCENChar`.
/// 4. Não pode ser um dos 8 nomes reservados proibidos pela especificação.
pub fn is_valid_custom_element_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some('a'..='z') => {}
        _ => return false,
    }

    let mut has_hyphen = false;
    for c in chars {
        if c == '-' {
            has_hyphen = true;
        }
        if !is_pcen_char(c) {
            return false;
        }
    }

    if !has_hyphen {
        return false;
    }

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

/// Estados de ciclo de vida de um elemento customizado (WHATWG §4.13.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomElementState {
    /// Elemento padrão (HTML/SVG/MathML) não customizado.
    #[default]
    Uncustomized,
    /// Elemento com nome customizado válido aguardando definição no registro.
    Undefined,
    /// Elemento customizado ativo com construtor/callbacks inicializados.
    Custom,
    /// Elemento built-in estendido com atributo `is` aguardando upgrade.
    Precustomized,
    /// Falha na inicialização ou construtor do elemento customizado.
    Failed,
}

/// Callbacks de ciclo de vida de elementos customizados.
#[derive(Clone, Default)]
pub struct CustomElementCallbacks {
    /// Callback invocado quando o elemento é conectado ao documento.
    pub connected_callback: Option<Arc<dyn Fn(NodeId) + Send + Sync>>,
    /// Callback invocado quando o elemento é desconectado do documento.
    pub disconnected_callback: Option<Arc<dyn Fn(NodeId) + Send + Sync>>,
    /// Callback invocado quando o elemento é adotado por outro documento.
    pub adopted_callback: Option<Arc<dyn Fn(NodeId, Option<&str>, Option<&str>) + Send + Sync>>,
    /// Callback invocado quando um atributo observado é alterado ou removido.
    pub attribute_changed_callback:
        Option<Arc<dyn Fn(NodeId, &str, Option<&str>, Option<&str>) + Send + Sync>>,
}

impl CustomElementCallbacks {
    /// Cria uma nova estrutura de callbacks vazia.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configura o `connectedCallback`.
    pub fn with_connected<F>(mut self, cb: F) -> Self
    where
        F: Fn(NodeId) + Send + Sync + 'static,
    {
        self.connected_callback = Some(Arc::new(cb));
        self
    }

    /// Configura o `disconnectedCallback`.
    pub fn with_disconnected<F>(mut self, cb: F) -> Self
    where
        F: Fn(NodeId) + Send + Sync + 'static,
    {
        self.disconnected_callback = Some(Arc::new(cb));
        self
    }

    /// Configura o `adoptedCallback`.
    pub fn with_adopted<F>(mut self, cb: F) -> Self
    where
        F: Fn(NodeId, Option<&str>, Option<&str>) + Send + Sync + 'static,
    {
        self.adopted_callback = Some(Arc::new(cb));
        self
    }

    /// Configura o `attributeChangedCallback`.
    pub fn with_attribute_changed<F>(mut self, cb: F) -> Self
    where
        F: Fn(NodeId, &str, Option<&str>, Option<&str>) + Send + Sync + 'static,
    {
        self.attribute_changed_callback = Some(Arc::new(cb));
        self
    }
}

impl std::fmt::Debug for CustomElementCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomElementCallbacks")
            .field("has_connected_callback", &self.connected_callback.is_some())
            .field(
                "has_disconnected_callback",
                &self.disconnected_callback.is_some(),
            )
            .field("has_adopted_callback", &self.adopted_callback.is_some())
            .field(
                "has_attribute_changed_callback",
                &self.attribute_changed_callback.is_some(),
            )
            .finish()
    }
}

/// Opções de definição para elementos customizados estendidos (`is="..."`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElementDefinitionOptions {
    pub extends: Option<Atom>,
}

/// Definição de um elemento customizado registrado no documento.
#[derive(Clone, Debug)]
pub struct CustomElementDefinition {
    /// Nome da tag do elemento customizado (ex: `user-card`).
    pub name: Atom,
    /// Lista de atributos observados que disparam `attributeChangedCallback`.
    pub observed_attributes: Vec<SmolStr>,
    /// Callbacks de ciclo de vida associados.
    pub callbacks: CustomElementCallbacks,
    /// Elemento built-in estendido, se houver (`extends`).
    pub extends: Option<Atom>,
    /// Se `true`, proíbe anexar ShadowRoot.
    pub disable_shadow: bool,
    /// Se `true`, desabilita ElementInternals.
    pub disable_internals: bool,
    /// Se `true`, é associado a formulários (`formAssociated`).
    pub form_associated: bool,
}

impl CustomElementDefinition {
    /// Cria uma nova definição básica.
    pub fn new(
        name: impl Into<Atom>,
        observed_attributes: Vec<SmolStr>,
        callbacks: CustomElementCallbacks,
    ) -> Self {
        Self {
            name: name.into(),
            observed_attributes,
            callbacks,
            extends: None,
            disable_shadow: false,
            disable_internals: false,
            form_associated: false,
        }
    }

    /// Verifica se um atributo está na lista de atributos observados (case-insensitive para HTML).
    pub fn observes_attribute(&self, attr_name: &str) -> bool {
        self.observed_attributes
            .iter()
            .any(|a| a.as_str().eq_ignore_ascii_case(attr_name))
    }
}

/// Registro oficial de elementos customizados (`customElements`).
#[derive(Default)]
pub struct CustomElementRegistry {
    definitions: FxHashMap<Atom, CustomElementDefinition>,
    when_defined_callbacks: FxHashMap<Atom, Vec<Box<dyn FnOnce() + Send + Sync>>>,
}

impl std::fmt::Debug for CustomElementRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomElementRegistry")
            .field("definitions_count", &self.definitions.len())
            .finish()
    }
}

impl CustomElementRegistry {
    /// Cria um novo registro de elementos customizados.
    pub fn new() -> Self {
        Self {
            definitions: FxHashMap::default(),
            when_defined_callbacks: FxHashMap::default(),
        }
    }

    /// Registra uma nova definição de elemento customizado no registro.
    pub fn define(&mut self, name: &str, observed_attrs: &[&str]) -> Result<(), DomError> {
        self.define_with_callbacks(name, observed_attrs, CustomElementCallbacks::default())
    }

    /// Registra uma definição com callbacks de ciclo de vida.
    pub fn define_with_callbacks(
        &mut self,
        name: &str,
        observed_attrs: &[&str],
        callbacks: CustomElementCallbacks,
    ) -> Result<(), DomError> {
        let attrs: Vec<SmolStr> = observed_attrs.iter().map(|&a| SmolStr::new(a)).collect();
        let def = CustomElementDefinition::new(name, attrs, callbacks);
        self.define_custom_element(def)
    }

    /// Registra uma definição completa de elemento customizado.
    pub fn define_custom_element(
        &mut self,
        definition: CustomElementDefinition,
    ) -> Result<(), DomError> {
        let name_str = definition.name.as_str();
        if !is_valid_custom_element_name(name_str) {
            return Err(DomError::HierarchyRequestError(format!(
                "'{}' não é um nome válido de elemento customizado",
                name_str
            )));
        }

        if self.definitions.contains_key(&definition.name) {
            return Err(DomError::HierarchyRequestError(format!(
                "Elemento customizado '{}' já está registrado",
                name_str
            )));
        }

        let atom_name = definition.name.clone();
        self.definitions.insert(atom_name.clone(), definition);

        // Resolve callbacks aguardando em when_defined
        if let Some(callbacks) = self.when_defined_callbacks.remove(&atom_name) {
            for cb in callbacks {
                cb();
            }
        }

        Ok(())
    }

    /// Retorna a definição de um elemento customizado registrado pelo nome da tag.
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition> {
        let atom = Atom::new(name);
        self.definitions.get(&atom)
    }

    /// Retorna `true` se um elemento customizado com o nome informado estiver registrado.
    pub fn is_defined(&self, name: &str) -> bool {
        let atom = Atom::new(name);
        self.definitions.contains_key(&atom)
    }

    /// Registra uma ação a ser executada assim que o elemento for definido (`whenDefined`).
    /// Se o elemento já estiver definido, executa a ação imediatamente.
    pub fn when_defined<F>(&mut self, name: &str, callback: F)
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        let atom = Atom::new(name);
        if self.definitions.contains_key(&atom) {
            callback();
        } else {
            self.when_defined_callbacks
                .entry(atom)
                .or_default()
                .push(Box::new(callback));
        }
    }

    /// Atualiza (`upgrade`) nós existentes na subárvore sob `root_id` cujo nome corresponda a uma definição registrada.
    pub fn upgrade(
        &self,
        arena: &mut Arena<NodeData>,
        root_id: NodeId,
        mut reactions: Option<&mut CustomElementReactionsStack>,
    ) {
        let node_ids = crate::tree::mutation::collect_subtree_node_ids(arena, root_id);

        for node_id in node_ids {
            let (should_upgrade, def_opt, attrs_to_notify, is_connected) = {
                let Some(aid) = ArenaId::<NodeData>::from_node_id(node_id) else {
                    continue;
                };
                let Some(node) = arena.get(aid) else {
                    continue;
                };
                let Some(el) = node.as_element() else {
                    continue;
                };

                let state = el.custom_element_state();
                if state == CustomElementState::Undefined || state == CustomElementState::Precustomized {
                    let tag_name = el.tag_name.as_str();
                    if let Some(def) = self.get(tag_name) {
                        let is_conn = crate::tree::mutation::is_connected_to_document(arena, node_id);
                        let mut observed = Vec::new();
                        for attr in el.attributes.as_slice() {
                            if def.observes_attribute(attr.name.as_str()) {
                                observed.push((attr.name.clone(), attr.value.clone()));
                            }
                        }
                        (true, Some(def.clone()), observed, is_conn)
                    } else {
                        (false, None, Vec::new(), false)
                    }
                } else {
                    (false, None, Vec::new(), false)
                }
            };

            if should_upgrade {
                if let Some(aid) = ArenaId::<NodeData>::from_node_id(node_id) {
                    if let Some(node) = arena.get_mut(aid) {
                        if let Some(el_mut) = node.as_element_mut() {
                            el_mut.set_custom_element_state(CustomElementState::Custom);
                            if let Some(ref def) = def_opt {
                                el_mut.set_custom_element_definition(Some(def.name.clone()));
                            }
                        }
                    }
                }

                if let Some(ref mut rs) = reactions {
                    if is_connected {
                        rs.enqueue_connected(node_id);
                    }
                    for (attr_name, attr_val) in attrs_to_notify {
                        rs.enqueue_attribute_changed(
                            node_id,
                            attr_name.as_str(),
                            None,
                            Some(attr_val),
                        );
                    }
                }
            }
        }
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
