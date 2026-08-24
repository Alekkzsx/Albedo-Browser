//! # Motor de Sanitização HTML & Defesas Anti-XSS (WHATWG §8.6)
//!
//! Remove elementos perigosos, scripts maliciosos, manipuladores `on*` inline e esquemas `javascript:`.

pub mod config;

pub use config::SanitizerConfig;

use crate::node::NodeKind;
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;

/// Sanitizador oficial de árvores DOM.
pub struct HTMLSanitizer;

impl HTMLSanitizer {
    /// Sanitiza uma árvore `Document` existente a partir do nó raiz informado.
    pub fn sanitize_subtree(doc: &mut Document, root_id: NodeId, config: &SanitizerConfig) {
        let mut to_remove = Vec::new();

        // 1. Identifica nós inválidos/bloqueados
        for (node_id, node) in doc.descendants(root_id) {
            match &node.kind {
                NodeKind::Element(el) => {
                    let tag = &el.tag_name;
                    // Verifica block list
                    if config.block_elements.contains(tag) {
                        to_remove.push(node_id);
                        continue;
                    }
                    // Verifica allow list
                    if let Some(ref allow) = config.allow_elements {
                        if !allow.contains(tag) {
                            to_remove.push(node_id);
                            continue;
                        }
                    }
                }
                NodeKind::Comment(_) if !config.allow_comments => {
                    to_remove.push(node_id);
                }
                _ => {}
            }
        }

        // Remove os nós bloqueados
        for node_id in to_remove {
            if let Some(parent_id) = doc.get_node(node_id).and_then(|n| n.parent) {
                let _ = doc.remove_child(parent_id, node_id);
            }
        }

        // 2. Sanitiza atributos nos elementos remanescentes
        let element_ids: Vec<NodeId> = doc
            .descendants(root_id)
            .filter_map(|(id, n)| if n.as_element().is_some() { Some(id) } else { None })
            .collect();

        for el_id in element_ids {
            if let Some(el) = doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                let mut attrs_to_remove = Vec::new();

                for attr in el.attributes.as_slice() {
                    let name_str = attr.name.as_str().to_ascii_lowercase();
                    let val_str = attr.value.as_str().trim().to_ascii_lowercase();

                    // Bloqueia qualquer manipulador inline onclick, onload, onerror, etc.
                    if name_str.starts_with("on") {
                        attrs_to_remove.push(attr.name.clone());
                        continue;
                    }

                    // Bloqueia esquemas perigosos javascript: e vbscript: em href e src
                    if (name_str == "href" || name_str == "src" || name_str == "xlink:href")
                        && (val_str.starts_with("javascript:") || val_str.starts_with("vbscript:"))
                    {
                        attrs_to_remove.push(attr.name.clone());
                        continue;
                    }

                    // Verifica listas de bloqueio
                    if config.block_attributes.contains(&attr.name) {
                        attrs_to_remove.push(attr.name.clone());
                        continue;
                    }

                    if let Some(ref allow_attrs) = config.allow_attributes {
                        if !allow_attrs.contains(&attr.name) {
                            attrs_to_remove.push(attr.name.clone());
                            continue;
                        }
                    }
                }

                for attr_name in attrs_to_remove {
                    el.remove_attribute(attr_name.as_str());
                }
            }
        }
    }

    /// Faz o parsing de uma string HTML aplicando automaticamente a sanitização de segurança.
    pub fn parse_html_safe(html: &str, config: Option<SanitizerConfig>) -> Document {
        let mut doc = crate::parse_html(html);
        let cfg = config.unwrap_or_default();
        let root = doc.root();
        Self::sanitize_subtree(&mut doc, root, &cfg);
        doc
    }
}
