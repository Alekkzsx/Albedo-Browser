use super::*;
//! AceDOM Virtual DOM & Diff/Patch Engine
//! Implementação otimizada para frameworks reativos (React, Solid, Svelte)
//! Objetivo: Updates 50x mais rápidos que re-renderização completa

use std::collections::HashMap;
use std::rc::Rc;
use crate::ace::engine::dom::{AceDOM, NodeId};

/// Representação leve de um nó no Virtual DOM

#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    /// Elemento HTML (ex: <div>)
    Element {
        tag: Rc<str>,
        attrs: HashMap<Rc<str>, Rc<str>>,
        children: Vec<VNode>,
        key: Option<Rc<str>>,
        namespace: Option<Rc<str>>,
    },
    /// Nó de texto
    Text(Rc<str>),
    /// Comentário
    Comment(Rc<str>),
    /// Fragmento (grupo sem pai)
    Fragment(Vec<VNode>),
}

impl VNode {
    /// TODO: add docs
    pub fn element(tag: &str, attrs: HashMap<&str, &str>, children: Vec<VNode>) -> Self {
        let attrs_rc = attrs.into_iter()
            .map(|(k, v)| (Rc::from(k), Rc::from(v)))
            .collect();
        
        VNode::Element {
            tag: Rc::from(tag),
            attrs: attrs_rc,
            children,
            key: None,
            namespace: None,
        }
    }

    /// TODO: add docs
    pub fn text(content: &str) -> Self {
        VNode::Text(Rc::from(content))
    }

    /// TODO: add docs
    pub fn with_key(self, key: &str) -> Self {
        match self {
            VNode::Element { tag, attrs, children, namespace, .. } => {
                VNode::Element {
                    tag,
                    attrs,
                    children,
                    key: Some(Rc::from(key)),
                    namespace,
                }
            }
            _ => self,
        }
    }
}
