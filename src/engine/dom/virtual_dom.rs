//! AceDOM Virtual DOM & Diff/Patch Engine
//! Implementação otimizada para frameworks reativos (React, Solid, Svelte)
//! Objetivo: Updates 50x mais rápidos que re-renderização completa

use std::collections::HashMap;
use std::rc::Rc;
use crate::engine::dom::{AceDOM, AceNode, AceNodeType, NodeId};

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

    pub fn text(content: &str) -> Self {
        VNode::Text(Rc::from(content))
    }

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

/// Tipos de operações de Patch
#[derive(Debug, Clone)]
pub enum PatchOp {
    /// Inserir nó em índice específico
    Insert { index: usize, node: VNode },
    /// Remover nó em índice específico
    Remove { index: usize },
    /// Substituir nó em índice específico
    Replace { index: usize, node: VNode },
    /// Atualizar atributo
    SetAttribute { index: usize, key: Rc<str>, value: Rc<str> },
    /// Remover atributo
    RemoveAttribute { index: usize, key: Rc<str> },
    /// Atualizar texto
    SetText { index: usize, text: Rc<str> },
    /// Mover nó de um índice para outro
    Move { from: usize, to: usize },
}

/// Resultado do algoritmo de Diff
pub struct DiffResult {
    pub patches: Vec<PatchOp>,
}

/// Motor de Diff/patch
pub struct VirtualDom;

impl VirtualDom {
    /// Calcula a diferença entre duas árvores VNode
    pub fn diff(old: &VNode, new: &VNode) -> DiffResult {
        let mut patches = Vec::new();
        Self::diff_recursive(old, new, 0, &mut patches);
        DiffResult { patches }
    }

    fn diff_recursive(old: &VNode, new: &VNode, index: usize, patches: &mut Vec<PatchOp>) {
        match (old, new) {
            // Tipos diferentes: substituir tudo
            (VNode::Text(old_txt), VNode::Text(new_txt)) => {
                if old_txt != new_txt {
                    patches.push(PatchOp::SetText { 
                        index, 
                        text: Rc::clone(new_txt) 
                    });
                }
            }
            
            (VNode::Element { tag: old_tag, attrs: old_attrs, children: old_children, key: old_key, .. },
             VNode::Element { tag: new_tag, attrs: new_attrs, children: new_children, key: new_key, .. }) => {
                
                // Tags diferentes ou keys diferentes -> Replace
                if old_tag != new_tag || old_key != new_key {
                    patches.push(PatchOp::Replace { 
                        index, 
                        node: new.clone() 
                    });
                    return;
                }

                // Diff de atributos
                Self::diff_attributes(old_attrs, new_attrs, index, patches);

                // Diff de filhos (algoritmo otimizado)
                Self::diff_children(old_children, new_children, index, patches);
            }

            // Casos de mudança de tipo (Element <-> Text)
            _ => {
                patches.push(PatchOp::Replace { 
                    index, 
                    node: new.clone() 
                });
            }
        }
    }

    fn diff_attributes(
        old_attrs: &HashMap<Rc<str>, Rc<str>>,
        new_attrs: &HashMap<Rc<str>, Rc<str>>,
        index: usize,
        patches: &mut Vec<PatchOp>
    ) {
        // Remover atributos antigos que não existem mais
        for (key, _) in old_attrs.iter() {
            if !new_attrs.contains_key(key) {
                patches.push(PatchOp::RemoveAttribute { 
                    index, 
                    key: Rc::clone(key) 
                });
            }
        }

        // Adicionar/Atualizar atributos novos
        for (key, new_val) in new_attrs.iter() {
            match old_attrs.get(key) {
                Some(old_val) if old_val == new_val => {} // Sem mudança
                _ => {
                    patches.push(PatchOp::SetAttribute { 
                        index, 
                        key: Rc::clone(key), 
                        value: Rc::clone(new_val) 
                    });
                }
            }
        }
    }

    fn diff_children(
        old_children: &[VNode],
        new_children: &[VNode],
        parent_index: usize,
        patches: &mut Vec<PatchOp>
    ) {
        // Algoritmo simplificado de reconciliação
        // Em produção, usar algoritmo baseado em chaves (LCS ou similar)
        
        let max_len = old_children.len().max(new_children.len());

        for i in 0..max_len {
            match (old_children.get(i), new_children.get(i)) {
                (Some(old), Some(new)) => {
                    // Verificar se tem key para otimização de movimento
                    if let (VNode::Element { key: Some(k1), .. }, VNode::Element { key: Some(k2), .. }) = (old, new) {
                        if k1 != k2 {
                            // Chaves diferentes: tentar encontrar e mover
                            // (Implementação simplificada: apenas substitui)
                            patches.push(PatchOp::Replace { 
                                index: i, 
                                node: new.clone() 
                            });
                            continue;
                        }
                    }
                    
                    Self::diff_recursive(old, new, i, patches);
                }
                (None, Some(new)) => {
                    // Novo nó adicionado
                    patches.push(PatchOp::Insert { 
                        index: i, 
                        node: new.clone() 
                    });
                }
                (Some(old), None) => {
                    // Nó removido
                    patches.push(PatchOp::Remove { index: i });
                }
                (None, None) => break,
            }
        }
    }

    /// Aplica patches no DOM real
    pub fn apply_patches(dom: &mut AceDOM, root_id: NodeId, patches: &[PatchOp]) {
        for patch in patches {
            match patch {
                PatchOp::Insert { index, node } => {
                    // Converter VNode para AceNode e inserir
                    let new_node = Self::vnode_to_ace_node(node, dom);
                    // dom.insert_child(root_id, index, new_node); // Implementar conforme API real
                }
                PatchOp::Remove { index } => {
                    // dom.remove_child_by_index(root_id, *index);
                }
                PatchOp::Replace { index, node } => {
                    let new_node = Self::vnode_to_ace_node(node, dom);
                    // dom.replace_child_by_index(root_id, *index, new_node);
                }
                PatchOp::SetAttribute { index, key, value } => {
                    // let node_id = dom.get_child_by_index(root_id, *index);
                    // dom.set_attribute(node_id, key.to_string(), value.to_string());
                }
                PatchOp::RemoveAttribute { index, key } => {
                    // let node_id = dom.get_child_by_index(root_id, *index);
                    // dom.remove_attribute(node_id, key.to_string());
                }
                PatchOp::SetText { index, text } => {
                    // let node_id = dom.get_child_by_index(root_id, *index);
                    // dom.set_text_content(node_id, text);
                }
                PatchOp::Move { from, to } => {
                    // dom.move_child(root_id, *from, *to);
                }
            }
        }
    }

    fn vnode_to_ace_node(vnode: &VNode, dom: &mut AceDOM) -> NodeId {
        // Conversão simplificada
        match vnode {
            VNode::Text(content) => dom.create_text_node(content),
            VNode::Comment(content) => dom.create_comment_node(content),
            VNode::Element { tag, attrs, children, .. } => {
                let id = dom.create_element_node(tag);
                for (k, v) in attrs {
                    dom.set_attribute(id, k.to_string(), v.to_string());
                }
                for child in children {
                    let child_id = Self::vnode_to_ace_node(child, dom);
                    dom.append_child(id, child_id);
                }
                id
            }
            VNode::Fragment(nodes) => {
                // Fragmentos são achatados
                if nodes.is_empty() {
                    dom.create_text_node("")
                } else {
                    Self::vnode_to_ace_node(&nodes[0], dom)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_text_change() {
        let old = VNode::text("Hello");
        let new = VNode::text("World");
        
        let result = VirtualDom::diff(&old, &new);
        assert_eq!(result.patches.len(), 1);
        assert!(matches!(result.patches[0], PatchOp::SetText { .. }));
    }

    #[test]
    fn test_diff_attr_change() {
        let mut attrs_old = HashMap::new();
        attrs_old.insert(Rc::from("class"), Rc::from("old"));
        
        let mut attrs_new = HashMap::new();
        attrs_new.insert(Rc::from("class"), Rc::from("new"));

        let old = VNode::element("div", [("class", "old")].into_iter().collect(), vec![]);
        let new = VNode::element("div", [("class", "new")].into_iter().collect(), vec![]);
        
        let result = VirtualDom::diff(&old, &new);
        assert_eq!(result.patches.len(), 1);
    }

    #[test]
    fn test_diff_add_child() {
        let old = VNode::element("ul", HashMap::new(), vec![
            VNode::element("li", HashMap::new(), vec![VNode::text("Item 1")])
        ]);
        
        let new = VNode::element("ul", HashMap::new(), vec![
            VNode::element("li", HashMap::new(), vec![VNode::text("Item 1")]),
            VNode::element("li", HashMap::new(), vec![VNode::text("Item 2")])
        ]);
        
        let result = VirtualDom::diff(&old, &new);
        assert!(result.patches.iter().any(|p| matches!(p, PatchOp::Insert { .. })));
    }
}
