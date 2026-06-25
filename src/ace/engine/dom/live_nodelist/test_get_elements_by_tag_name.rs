//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use super::*;
use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::engine::dom::AceDOM;
    
    #[test]
pub(crate) fn test_get_elements_by_tag_name() {
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
        
        let body_idx = dom.body.expect("Albedo Engine: internal invariant violated");
        let collection = LiveNodeList::new(TagNameQuery("div".to_string()), body_idx);
        
        assert_eq!(collection.length(&dom), 2);
        assert!(collection.item(&dom, 0).is_some());
        assert!(collection.item(&dom, 1).is_some());
        assert!(collection.item(&dom, 2).is_none());
    }
    
    #[test]
pub(crate) fn test_get_elements_by_class_name() {
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
        
        let body_idx = dom.body.expect("Albedo Engine: internal invariant violated");
        let query = ClassNameQuery(vec!["item".to_string()]);
        let collection = LiveNodeList::new(query, body_idx);
        
        assert_eq!(collection.length(&dom), 2);
    }
    
    #[test]
pub(crate) fn test_html_collection_named_item() {
        let dom = AceDOM::from_html(r#"
            <html>
                <body>
                    <input name="username" id="user-input">
                    <input name="password">
                </body>
            </html>
        "#);
        
        let body_idx = dom.body.expect("Albedo Engine: internal invariant violated");
        let collection = HTMLCollection::new("input", body_idx);
        
        assert_eq!(collection.length(&dom), 2);
        assert!(collection.named_item(&dom, "username").is_some());
        assert!(collection.named_item(&dom, "user-input").is_some());
        assert!(collection.named_item(&dom, "password").is_some());
        assert!(collection.named_item(&dom, "nonexistent").is_none());
    }
    
    #[test]
pub(crate) fn test_children_collection() {
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
        let root_idx = dom.query_selector("div").expect("Albedo Engine: internal invariant violated");
        let children = ChildrenCollection::new(root_idx);
        
        // Deve retornar apenas elements (3: span, p, div)
        assert_eq!(children.length(&dom), 3);
    }
    
    #[test]
pub(crate) fn test_live_update() {
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
