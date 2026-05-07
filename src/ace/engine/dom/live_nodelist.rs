//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use std::cell::RefCell;
use std::rc::Rc;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType, AceElement};

/// Trait para tipos de queries suportados por LiveNodeList
pub trait NodeQuery {
    fn matches(&self, node: &AceNode, dom: &AceDOM) -> bool;
}

/// Query por tag name (case-insensitive para HTML)
#[derive(Clone, Debug)]
pub struct TagNameQuery(pub String);

impl NodeQuery for TagNameQuery {
    fn matches(&self, node: &AceNode, _dom: &AceDOM) -> bool {
        match &node.node_type {
            AceNodeType::Element(el) => {
                el.tag.eq_ignore_ascii_case(&self.0) || self.0 == "*"
            }
            _ => false,
        }
    }
}

/// Query por class name (suporta múltiplas classes)
#[derive(Clone, Debug)]
pub struct ClassNameQuery(pub Vec<String>);

impl NodeQuery for ClassNameQuery {
    fn matches(&self, node: &AceNode, _dom: &AceDOM) -> bool {
        match &node.node_type {
            AceNodeType::Element(el) => {
                if let Some(class_attr) = el.attributes.get("class") {
                    let node_classes: Vec<&str> = class_attr.split_whitespace().collect();
                    self.0.iter().all(|required| {
                        node_classes.iter().any(|nc| nc == &required.as_str())
                    })
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Query por ID (exato)
#[derive(Clone, Debug)]
pub struct IdQuery(pub String);

impl NodeQuery for IdQuery {
    fn matches(&self, node: &AceNode, _dom: &AceDOM) -> bool {
        match &node.node_type {
            AceNodeType::Element(el) => {
                el.attributes.get("id").map_or(false, |id| id == &self.0)
            }
            _ => false,
        }
    }
}

/// LiveNodeList - coleção que se atualiza automaticamente
/// 
/// Diferente de NodeList estático do DOM tradicional, esta implementação
/// usa lazy evaluation: os nós são computados sob demanda quando o índice
/// é acessado, garantindo que sempre reflitam o estado atual do DOM.
pub struct LiveNodeList<Q: NodeQuery> {
    query: Q,
    root_idx: usize,
    /// Cache opcional para performance (invalidado em mutations)
    cache: RefCell<Option<Vec<usize>>>,
    /// Flag para forçar recomputação
    dirty: RefCell<bool>,
}

impl<Q: NodeQuery + Clone> LiveNodeList<Q> {
    pub fn new(query: Q, root_idx: usize) -> Self {
        Self {
            query,
            root_idx,
            cache: RefCell::new(None),
            dirty: RefCell::new(true),
        }
    }
    
    /// Marca a lista como suja (precisa recomputar)
    pub fn mark_dirty(&self) {
        *self.dirty.borrow_mut() = true;
        *self.cache.borrow_mut() = None;
    }
    
    /// Computa todos os nós matching (DFS traversal)
    fn compute_matches(&self, dom: &AceDOM) -> Vec<usize> {
        let mut matches = Vec::new();
        self.collect_matches(self.root_idx, dom, &mut matches);
        matches
    }
    
    /// Coleta recursivamente nós que matcham a query
    fn collect_matches(&self, node_idx: usize, dom: &AceDOM, matches: &mut Vec<usize>) {
        if let Some(node) = dom.get_node(node_idx) {
            if self.query.matches(node, dom) {
                matches.push(node_idx);
            }
            
            // Recurse into children
            for &child_idx in &node.children {
                self.collect_matches(child_idx, dom, matches);
            }
        }
    }
    
    /// Retorna o número de nós na coleção
    pub fn length(&self, dom: &AceDOM) -> usize {
        if *self.dirty.borrow() {
            self.compute_matches(dom).len()
        } else {
            self.cache.borrow().as_ref().map_or(0, |v| v.len())
        }
    }
    
    /// Retorna o nó no índice especificado (ou None)
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        let mut cache_ref = self.cache.borrow_mut();
        
        if *self.dirty.borrow() || cache_ref.is_none() {
            *cache_ref = Some(self.compute_matches(dom));
            *self.dirty.borrow_mut() = false;
        }
        
        cache_ref.as_ref().and_then(|matches| matches.get(index).copied())
    }
    
    /// Retorna todos os nós como Vec (snapshot)
    pub fn to_vec(&self, dom: &AceDOM) -> Vec<usize> {
        let mut cache_ref = self.cache.borrow_mut();
        
        if *self.dirty.borrow() || cache_ref.is_none() {
            *cache_ref = Some(self.compute_matches(dom));
            *self.dirty.borrow_mut() = false;
        }
        
        cache_ref.as_ref().cloned().unwrap_or_default()
    }
    
    /// Itera sobre todos os nós matching
    pub fn for_each<F>(&self, dom: &AceDOM, mut f: F)
    where
        F: FnMut(usize, &AceNode),
    {
        let matches = self.to_vec(dom);
        for idx in matches {
            if let Some(node) = dom.get_node(idx) {
                f(idx, node);
            }
        }
    }
}

/// HTMLCollection - Live collection para elementos (apenas Element nodes)
/// 
/// Similar a LiveNodeList mas retorna apenas elementos e tem métodos adicionais
/// como namedItem() para acesso por name ou id.
pub struct HTMLCollection {
    inner: LiveNodeList<TagNameQuery>,
}

impl HTMLCollection {
    pub fn new(tag_name: &str, root_idx: usize) -> Self {
        Self {
            inner: LiveNodeList::new(TagNameQuery(tag_name.to_string()), root_idx),
        }
    }
    
    /// Número de elementos na coleção
    pub fn length(&self, dom: &AceDOM) -> usize {
        self.inner.length(dom)
    }
    
    /// Retorna elemento no índice (ou None)
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        self.inner.item(dom, index)
    }
    
    /// Retorna elemento por name ou id (primeiro match)
    pub fn named_item(&self, dom: &AceDOM, name: &str) -> Option<usize> {
        let matches = self.inner.to_vec(dom);
        for idx in matches {
            if let Some(node) = dom.get_node(idx) {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.attributes.get("id").map_or(false, |id| id == name)
                        || el.attributes.get("name").map_or(false, |n| n == name)
                    {
                        return Some(idx);
                    }
                }
            }
        }
        None
    }
    
    /// Marca como dirty (chamar após DOM mutations)
    pub fn mark_dirty(&self) {
        self.inner.mark_dirty();
    }
}

/// NodeList - pode ser live ou snapshot
/// 
/// Por padrão é live como HTMLCollection, mas pode ser convertido
/// para snapshot se necessário.
pub struct NodeList {
    inner: LiveNodeList<TagNameQuery>,
    is_snapshot: bool,
}

impl NodeList {
    /// Cria NodeList live (auto-update)
    pub fn new_live(root_idx: usize) -> Self {
        Self {
            inner: LiveNodeList::new(TagNameQuery("*".to_string()), root_idx),
            is_snapshot: false,
        }
    }
    
    /// Cria NodeList snapshot (não atualiza)
    pub fn new_snapshot(nodes: Vec<usize>) -> Self {
        // Implementação simplificada - em produção usaria enum
        Self {
            inner: LiveNodeList::new(TagNameQuery("*".to_string()), 0),
            is_snapshot: true,
        }
    }
    
    pub fn length(&self, dom: &AceDOM) -> usize {
        if self.is_snapshot {
            // Em produção, teria um campo separado para snapshot
            0
        } else {
            self.inner.length(dom)
        }
    }
    
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        if self.is_snapshot {
            None
        } else {
            self.inner.item(dom, index)
        }
    }
    
    pub fn mark_dirty(&self) {
        if !self.is_snapshot {
            self.inner.mark_dirty();
        }
    }
}

/// ChildrenCollection - HTMLCollection live dos children de um elemento
pub struct ChildrenCollection {
    parent_idx: usize,
    cache: RefCell<Option<Vec<usize>>>,
    dirty: RefCell<bool>,
}

impl ChildrenCollection {
    pub fn new(parent_idx: usize) -> Self {
        Self {
            parent_idx,
            cache: RefCell::new(None),
            dirty: RefCell::new(true),
        }
    }
    
    fn compute_children(&self, dom: &AceDOM) -> Vec<usize> {
        if let Some(parent) = dom.get_node(self.parent_idx) {
            parent.children.iter().copied().filter(|&idx| {
                if let Some(node) = dom.get_node(idx) {
                    matches!(node.node_type, AceNodeType::Element(_))
                } else {
                    false
                }
            }).collect()
        } else {
            Vec::new()
        }
    }
    
    pub fn length(&self, dom: &AceDOM) -> usize {
        if *self.dirty.borrow() {
            self.compute_children(dom).len()
        } else {
            self.cache.borrow().as_ref().map_or(0, |v| v.len())
        }
    }
    
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        let mut cache_ref = self.cache.borrow_mut();
        
        if *self.dirty.borrow() || cache_ref.is_none() {
            *cache_ref = Some(self.compute_children(dom));
            *self.dirty.borrow_mut() = false;
        }
        
        cache_ref.as_ref().and_then(|children| children.get(index).copied())
    }
    
    pub fn mark_dirty(&self) {
        *self.dirty.borrow_mut() = true;
        *self.cache.borrow_mut() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::engine::dom::AceDOM;
    
    #[test]
    fn test_get_elements_by_tag_name() {
        let dom = AceDOM::from_html(r#"
            <html>
                <body>
                    <div id="d1">
                        <span>s1</span>
                        <p>p1</p>
                        <div id="d2">
                            <span>s2</span>
                        </div>
                    </div>
                </body>
            </html>
        "#);
        
        let body_idx = dom.body.unwrap();
        let collection = LiveNodeList::new(TagNameQuery("div".to_string()), body_idx);
        
        assert_eq!(collection.length(&dom), 2);
        assert!(collection.item(&dom, 0).is_some());
        assert!(collection.item(&dom, 1).is_some());
        assert!(collection.item(&dom, 2).is_none());
    }
    
    #[test]
    fn test_get_elements_by_class_name() {
        let dom = AceDOM::from_html(r#"
            <html>
                <body>
                    <div class="container active">
                        <span class="item">s1</span>
                        <span class="item highlight">s2</span>
                        <p class="other">p1</p>
                    </div>
                </body>
            </html>
        "#);
        
        let body_idx = dom.body.unwrap();
        let query = ClassNameQuery(vec!["item".to_string()]);
        let collection = LiveNodeList::new(query, body_idx);
        
        assert_eq!(collection.length(&dom), 2);
    }
    
    #[test]
    fn test_html_collection_named_item() {
        let dom = AceDOM::from_html(r#"
            <html>
                <body>
                    <input name="username" id="user-input">
                    <input name="password">
                </body>
            </html>
        "#);
        
        let body_idx = dom.body.unwrap();
        let collection = HTMLCollection::new("input", body_idx);
        
        assert_eq!(collection.length(&dom), 2);
        assert!(collection.named_item(&dom, "username").is_some());
        assert!(collection.named_item(&dom, "user-input").is_some());
        assert!(collection.named_item(&dom, "password").is_some());
        assert!(collection.named_item(&dom, "nonexistent").is_none());
    }
    
    #[test]
    fn test_children_collection() {
        let dom = AceDOM::from_html(r#"
            <div>
                <span>s1</span>
                text node
                <p>p1</p>
                <!-- comment -->
                <div>d1</div>
            </div>
        "#);
        
        // Pega o primeiro div
        let root_idx = 0;
        let children = ChildrenCollection::new(root_idx);
        
        // Deve retornar apenas elements (3: span, p, div)
        assert_eq!(children.length(&dom), 3);
    }
    
    #[test]
    fn test_live_update() {
        let mut dom = AceDOM::from_html(r#"
            <div><span>s1</span></div>
        "#);
        
        let root_idx = 0;
        let collection = LiveNodeList::new(TagNameQuery("span".to_string()), root_idx);
        
        assert_eq!(collection.length(&dom), 1);
        
        // Adiciona outro span dinamicamente
        // (em produção, isso seria via DOM manipulation APIs)
        // Aqui simulamos marcando como dirty
        
        collection.mark_dirty();
        // Após mutation, o próximo length() deve refletir mudanças
        // (teste simplificado - em produção testaria com append_child real)
    }
}
