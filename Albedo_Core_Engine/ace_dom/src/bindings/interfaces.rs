//! # Implementações Concretas de Interfaces WebIDL (WHATWG DOM Specification)
//!
//! Tabela de funções e despachantes para as interfaces `Node`, `Element`, `Document`, etc.

use crate::bindings::webidl::{WebIDLException, WebIDLResult};
use crate::events::Event;
use crate::fragment::parse_fragment;
use crate::node::element::Namespace;
use crate::node::NodeKind;
use crate::query::selector::ComplexSelector;
use crate::serializer::{serialize_inner_html, serialize_node};
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Implementação WebIDL para a interface `Node` (WHATWG DOM §3).
pub struct NodeBindings;

impl NodeBindings {
    /// WebIDL: `readonly attribute unsigned short nodeType;`
    pub fn get_node_type(doc: &Document, id: NodeId) -> Option<u16> {
        let node = doc.get_node(id)?;
        Some(match node.kind {
            NodeKind::Element(_) => 1,
            NodeKind::Text(_) => 3,
            NodeKind::Comment(_) => 8,
            NodeKind::Document(_) => 9,
            NodeKind::DocumentType(_) => 10,
            NodeKind::DocumentFragment => 11,
            NodeKind::ShadowRoot(_) => 11,
        })
    }

    /// WebIDL: `readonly attribute DOMString nodeName;`
    pub fn get_node_name(doc: &Document, id: NodeId) -> Option<SmolStr> {
        let node = doc.get_node(id)?;
        Some(match node.kind {
            NodeKind::Element(ref el) => SmolStr::new(el.tag_name.as_str().to_ascii_uppercase()),
            NodeKind::Text(_) => SmolStr::new("#text"),
            NodeKind::Comment(_) => SmolStr::new("#comment"),
            NodeKind::Document(_) => SmolStr::new("#document"),
            NodeKind::DocumentType(ref d) => d.name.clone(),
            NodeKind::DocumentFragment => SmolStr::new("#document-fragment"),
            NodeKind::ShadowRoot(_) => SmolStr::new("#shadow-root"),
        })
    }

    /// WebIDL: `attribute DOMString? textContent;` (Getter)
    pub fn get_text_content(doc: &Document, id: NodeId) -> Option<SmolStr> {
        let node = doc.get_node(id)?;
        match node.kind {
            NodeKind::Text(ref t) => Some(t.data.clone()),
            NodeKind::Comment(ref c) => Some(c.data.clone()),
            NodeKind::Element(_) | NodeKind::DocumentFragment | NodeKind::ShadowRoot(_) => {
                let mut buf = String::new();
                for (_desc_id, desc_node) in doc.descendants(id) {
                    if let NodeKind::Text(ref t) = desc_node.kind {
                        buf.push_str(t.data.as_str());
                    }
                }
                Some(SmolStr::new(buf))
            }
            _ => None,
        }
    }

    /// WebIDL: `attribute DOMString? textContent;` (Setter)
    pub fn set_text_content(doc: &mut Document, id: NodeId, text: &str) -> WebIDLResult<()> {
        let node = doc.get_node(id).ok_or_else(|| {
            WebIDLException::NotFoundError(format!("Nó #{:?} não encontrado", id.raw()))
        })?;

        match node.kind {
            NodeKind::Text(_) => {
                if let Some(n) = doc.get_node_mut(id) {
                    if let NodeKind::Text(ref mut t) = n.kind {
                        t.data = SmolStr::new(text);
                    }
                }
            }
            NodeKind::Element(_) | NodeKind::DocumentFragment | NodeKind::ShadowRoot(_) => {
                // Remove todos os filhos existentes
                let children: Vec<NodeId> = doc.children(id).map(|(c_id, _)| c_id).collect();
                for child in children {
                    let _ = doc.remove_child(id, child);
                }
                // Adiciona novo nó de texto se não estiver vazio
                if !text.is_empty() {
                    let text_node = doc.create_text_node(text);
                    let _ = doc.append_child(id, text_node);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// WebIDL: `Node appendChild(Node node);`
    pub fn append_child(
        doc: &mut Document,
        parent: NodeId,
        child: NodeId,
    ) -> WebIDLResult<NodeId> {
        doc.append_child(parent, child)
            .map(|()| child)
            .map_err(WebIDLException::from)
    }

    /// WebIDL: `Node removeChild(Node child);`
    pub fn remove_child(
        doc: &mut Document,
        parent: NodeId,
        child: NodeId,
    ) -> WebIDLResult<NodeId> {
        doc.remove_child(parent, child)
            .map(|()| child)
            .map_err(WebIDLException::from)
    }

    /// WebIDL: `Node insertBefore(Node node, Node? child);`
    pub fn insert_before(
        doc: &mut Document,
        parent: NodeId,
        new_child: NodeId,
        ref_child: Option<NodeId>,
    ) -> WebIDLResult<NodeId> {
        doc.insert_before(parent, new_child, ref_child)
            .map(|()| new_child)
            .map_err(WebIDLException::from)
    }

    /// WebIDL: `boolean contains(Node? other);`
    pub fn contains(doc: &Document, parent: NodeId, other: NodeId) -> bool {
        doc.contains(parent, other)
    }

    /// WebIDL: `undefined normalize();`
    pub fn normalize(doc: &mut Document, id: NodeId) -> WebIDLResult<()> {
        doc.normalize(id).map_err(WebIDLException::from)
    }
}

/// Implementação WebIDL para a interface `Element` (WHATWG DOM §4).
pub struct ElementBindings;

impl ElementBindings {
    /// WebIDL: `readonly attribute DOMString tagName;`
    pub fn get_tag_name(doc: &Document, id: NodeId) -> Option<SmolStr> {
        let node = doc.get_node(id)?;
        let el = node.as_element()?;
        Some(SmolStr::new(el.tag_name.as_str().to_ascii_uppercase()))
    }

    /// WebIDL: `attribute DOMString id;` (Getter)
    pub fn get_id(doc: &Document, id: NodeId) -> Option<SmolStr> {
        let node = doc.get_node(id)?;
        let el = node.as_element()?;
        el.id_attr.as_ref().map(|a| SmolStr::new(a.as_str()))
    }

    /// WebIDL: `attribute DOMString id;` (Setter)
    pub fn set_id(doc: &mut Document, id: NodeId, value: &str) -> WebIDLResult<()> {
        let node = doc.get_node_mut(id).ok_or_else(|| {
            WebIDLException::NotFoundError(format!("Elemento #{:?} não encontrado", id.raw()))
        })?;
        let el = node.as_element_mut().ok_or_else(|| {
            WebIDLException::TypeError("Nó alvo não é um Elemento".to_string())
        })?;
        if value.is_empty() {
            el.id_attr = None;
            el.remove_attribute("id");
        } else {
            let atom = Atom::new(value);
            el.id_attr = Some(atom.clone());
            el.set_attribute("id", value);
        }
        Ok(())
    }

    /// WebIDL: `DOMString? getAttribute(DOMString qualifiedName);`
    pub fn get_attribute(doc: &Document, id: NodeId, name: &str) -> Option<SmolStr> {
        let node = doc.get_node(id)?;
        let el = node.as_element()?;
        el.get_attribute(name).map(SmolStr::new)
    }

    /// WebIDL: `undefined setAttribute(DOMString qualifiedName, DOMString value);`
    pub fn set_attribute(
        doc: &mut Document,
        id: NodeId,
        name: &str,
        value: &str,
    ) -> WebIDLResult<()> {
        let node = doc.get_node_mut(id).ok_or_else(|| {
            WebIDLException::NotFoundError(format!("Elemento #{:?} não encontrado", id.raw()))
        })?;
        let el = node.as_element_mut().ok_or_else(|| {
            WebIDLException::TypeError("Nó alvo não é um Elemento".to_string())
        })?;
        el.set_attribute(name, value);
        if name.eq_ignore_ascii_case("id") {
            el.id_attr = if value.is_empty() { None } else { Some(Atom::new(value)) };
        }
        Ok(())
    }

    /// WebIDL: `undefined removeAttribute(DOMString qualifiedName);`
    pub fn remove_attribute(doc: &mut Document, id: NodeId, name: &str) -> WebIDLResult<()> {
        let node = doc.get_node_mut(id).ok_or_else(|| {
            WebIDLException::NotFoundError(format!("Elemento #{:?} não encontrado", id.raw()))
        })?;
        let el = node.as_element_mut().ok_or_else(|| {
            WebIDLException::TypeError("Nó alvo não é um Elemento".to_string())
        })?;
        el.remove_attribute(name);
        if name.eq_ignore_ascii_case("id") {
            el.id_attr = None;
        }
        Ok(())
    }

    /// WebIDL: `boolean hasAttribute(DOMString qualifiedName);`
    pub fn has_attribute(doc: &Document, id: NodeId, name: &str) -> bool {
        doc.get_node(id)
            .and_then(|n| n.as_element())
            .is_some_and(|el| el.has_attribute(name))
    }

    /// WebIDL: `[CEReactions] attribute [LegacyNullToEmptyString] DOMString innerHTML;` (Getter)
    pub fn get_inner_html(doc: &Document, id: NodeId) -> SmolStr {
        SmolStr::new(serialize_inner_html(doc, id))
    }

    /// WebIDL: `[CEReactions] attribute [LegacyNullToEmptyString] DOMString innerHTML;` (Setter)
    pub fn set_inner_html(doc: &mut Document, id: NodeId, html: &str) -> WebIDLResult<()> {
        let node = doc.get_node(id).ok_or_else(|| {
            WebIDLException::NotFoundError(format!("Elemento #{:?} não encontrado", id.raw()))
        })?;
        if !node.is_element() {
            return Err(WebIDLException::TypeError(
                "innerHTML só pode ser definido em nós Elemento".to_string(),
            ));
        }

        let frag_id = parse_fragment(doc, Some(id), html);

        // Remove filhos existentes
        let children: Vec<NodeId> = doc.children(id).map(|(c_id, _)| c_id).collect();
        for child in children {
            let _ = doc.remove_child(id, child);
        }

        // Anexa novos nós do fragmento
        let frag_children: Vec<NodeId> = doc.children(frag_id).map(|(c_id, _)| c_id).collect();
        for frag_child in frag_children {
            let _ = doc.remove_child(frag_id, frag_child);
            let _ = doc.append_child(id, frag_child);
        }

        Ok(())
    }

    /// WebIDL: `[CEReactions] attribute [LegacyNullToEmptyString] DOMString outerHTML;` (Getter)
    pub fn get_outer_html(doc: &Document, id: NodeId) -> SmolStr {
        SmolStr::new(serialize_node(doc, id))
    }

    /// WebIDL: `Element? querySelector(DOMString selectors);`
    pub fn query_selector(doc: &Document, id: NodeId, selectors: &str) -> Option<NodeId> {
        let selector = ComplexSelector::parse(selectors)?;
        doc.descendants(id)
            .find(|&(desc_id, desc_node)| desc_node.is_element() && selector.matches(doc, desc_id))
            .map(|(desc_id, _)| desc_id)
    }

    /// WebIDL: `NodeList querySelectorAll(DOMString selectors);`
    pub fn query_selector_all(doc: &Document, id: NodeId, selectors: &str) -> Vec<NodeId> {
        let selector = match ComplexSelector::parse(selectors) {
            Some(s) => s,
            None => return Vec::new(),
        };
        doc.descendants(id)
            .filter(|&(desc_id, desc_node)| desc_node.is_element() && selector.matches(doc, desc_id))
            .map(|(desc_id, _)| desc_id)
            .collect()
    }

    /// WebIDL: `boolean matches(DOMString selectors);`
    pub fn matches(doc: &Document, id: NodeId, selectors: &str) -> bool {
        doc.element_matches(id, selectors)
    }

    /// WebIDL: `Element? closest(DOMString selectors);`
    pub fn closest(doc: &Document, id: NodeId, selectors: &str) -> Option<NodeId> {
        doc.element_closest(id, selectors)
    }
}

/// Implementação WebIDL para a interface `Document` (WHATWG DOM §5).
pub struct DocumentBindings;

impl DocumentBindings {
    /// WebIDL: `readonly attribute Element? documentElement;`
    pub fn get_document_element(doc: &Document) -> Option<NodeId> {
        doc.document_element
    }

    /// WebIDL: `readonly attribute HTMLElement? body;`
    pub fn get_body(doc: &Document) -> Option<NodeId> {
        doc.body
    }

    /// WebIDL: `readonly attribute HTMLHeadElement? head;`
    pub fn get_head(doc: &Document) -> Option<NodeId> {
        doc.head
    }

    /// WebIDL: `Element createElement(DOMString localName, optional (DOMString or ElementCreationOptions) options = {});`
    pub fn create_element(doc: &mut Document, local_name: &str) -> NodeId {
        doc.create_element(local_name, Namespace::Html)
    }

    /// WebIDL: `Text createTextNode(DOMString data);`
    pub fn create_text_node(doc: &mut Document, data: &str) -> NodeId {
        doc.create_text_node(data)
    }

    /// WebIDL: `Comment createComment(DOMString data);`
    pub fn create_comment(doc: &mut Document, data: &str) -> NodeId {
        doc.create_comment(data)
    }

    /// WebIDL: `Element? getElementById(DOMString elementId);`
    pub fn get_element_by_id(doc: &Document, id: &str) -> Option<NodeId> {
        doc.get_element_by_id(id)
    }
}

/// Implementação WebIDL para a interface `Text` (WHATWG DOM §4.6).
pub struct TextBindings;

impl TextBindings {
    /// WebIDL: `Text splitText(unsigned long offset);`
    pub fn split_text(doc: &mut Document, id: NodeId, offset: usize) -> WebIDLResult<NodeId> {
        doc.split_text(id, offset).map_err(WebIDLException::from)
    }

    /// WebIDL: `attribute DOMString data;` (Getter)
    pub fn get_data(doc: &Document, id: NodeId) -> Option<SmolStr> {
        let node = doc.get_node(id)?;
        if let NodeKind::Text(ref t) = node.kind {
            Some(t.data.clone())
        } else {
            None
        }
    }

    /// WebIDL: `readonly attribute unsigned long length;`
    pub fn get_length(doc: &Document, id: NodeId) -> Option<usize> {
        let node = doc.get_node(id)?;
        if let NodeKind::Text(ref t) = node.kind {
            Some(t.data.len())
        } else {
            None
        }
    }
}

/// Implementação WebIDL para a interface `Event` (WHATWG DOM §2).
pub struct EventBindings;

impl EventBindings {
    /// WebIDL: `sequence<EventTarget> composedPath();`
    pub fn composed_path(event: &Event) -> Vec<NodeId> {
        event.composed_path().to_vec()
    }

    /// WebIDL: `readonly attribute DOMString type;`
    pub fn get_type(event: &Event) -> SmolStr {
        SmolStr::new(event.event_type.as_str())
    }

    /// WebIDL: `readonly attribute EventTarget? target;`
    pub fn get_target(event: &Event) -> Option<NodeId> {
        event.target
    }

    /// WebIDL: `readonly attribute EventTarget? currentTarget;`
    pub fn get_current_target(event: &Event) -> Option<NodeId> {
        event.current_target
    }

    /// WebIDL: `readonly attribute unsigned short eventPhase;`
    pub fn get_event_phase(event: &Event) -> u16 {
        event.phase as u16
    }

    /// WebIDL: `readonly attribute boolean bubbles;`
    pub fn get_bubbles(event: &Event) -> bool {
        event.bubbles
    }

    /// WebIDL: `readonly attribute boolean cancelable;`
    pub fn get_cancelable(event: &Event) -> bool {
        event.cancelable
    }

    /// WebIDL: `readonly attribute boolean defaultPrevented;`
    pub fn get_default_prevented(event: &Event) -> bool {
        event.is_default_prevented()
    }

    /// WebIDL: `readonly attribute boolean composed;`
    pub fn get_composed(event: &Event) -> bool {
        event.composed
    }

    /// WebIDL: `undefined stopPropagation();`
    pub fn stop_propagation(event: &Event) {
        event.stop_propagation();
    }

    /// WebIDL: `undefined stopImmediatePropagation();`
    pub fn stop_immediate_propagation(event: &Event) {
        event.stop_immediate_propagation();
    }

    /// WebIDL: `undefined preventDefault();`
    pub fn prevent_default(event: &Event) {
        event.prevent_default();
    }
}
