//! # Serializador Normativo HTML5 (WHATWG §13.3)
//!
//! Converte nós da árvore DOM em representações textuais serializadas seguras (`outerHTML` e `innerHTML`).

use crate::node::NodeKind;
use crate::tree::Document;
use ace_core::id::NodeId;

/// Faz o escape normativo de texto puro para HTML (WHATWG §13.3).
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\u{00A0}' => out.push_str("&nbsp;"),
            other => out.push(other),
        }
    }
    out
}

/// Faz o escape normativo de valores de atributo (WHATWG §13.3).
fn escape_attribute_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\u{00A0}' => out.push_str("&nbsp;"),
            other => out.push(other),
        }
    }
    out
}

/// Retorna `true` se o elemento for void (não possui tag de fechamento).
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Retorna `true` se o elemento for RAWTEXT (o conteúdo de texto não é escapado).
fn is_rawtext_element(tag: &str) -> bool {
    matches!(
        tag,
        "style" | "script" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext"
    )
}

/// Serializa um nó DOM e todos os seus descendentes em uma string HTML normativa.
pub fn serialize_node(doc: &Document, node_id: NodeId) -> String {
    let mut out = String::new();
    serialize_node_into(doc, node_id, &mut out);
    out
}

/// Serializa os filhos de um nó DOM (`innerHTML`).
pub fn serialize_inner_html(doc: &Document, node_id: NodeId) -> String {
    let mut out = String::new();
    if let Some(node) = doc.get_node(node_id) {
        if let Some(el) = node.as_element() {
            if el.tag_name.eq_ignore_ascii_case("template") {
                if let Some(frag_id) = el.template_content {
                    for (child_id, _) in doc.children(frag_id) {
                        serialize_node_into(doc, child_id, &mut out);
                    }
                    return out;
                }
            }
        }
    }

    for (child_id, _) in doc.children(node_id) {
        serialize_node_into(doc, child_id, &mut out);
    }
    out
}

fn serialize_node_into(doc: &Document, node_id: NodeId, out: &mut String) {
    let node = match doc.get_node(node_id) {
        Some(n) => n,
        None => return,
    };

    match &node.kind {
        NodeKind::Document(_) | NodeKind::DocumentFragment => {
            for (child_id, _) in doc.children(node_id) {
                serialize_node_into(doc, child_id, out);
            }
        }
        NodeKind::DocumentType(d) => {
            out.push_str("<!DOCTYPE ");
            out.push_str(d.name.as_str());
            out.push('>');
        }
        NodeKind::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c.data.as_str());
            out.push_str("-->");
        }
        NodeKind::Text(t) => {
            let is_raw = node
                .parent
                .and_then(|p_id| doc.get_node(p_id))
                .and_then(|p_node| p_node.tag_name())
                .is_some_and(|tag| is_rawtext_element(tag.as_str()));

            if is_raw {
                out.push_str(t.data.as_str());
            } else {
                out.push_str(&escape_text(t.data.as_str()));
            }
        }
        NodeKind::Element(el) => {
            let tag = el.tag_name.as_str();
            out.push('<');
            out.push_str(tag);

            for attr in el.attributes.as_slice() {
                out.push(' ');
                out.push_str(attr.name.as_str());
                out.push_str("=\"");
                out.push_str(&escape_attribute_value(attr.value.as_str()));
                out.push('"');
            }
            out.push('>');

            if is_void_element(tag) {
                return;
            }

            if tag.eq_ignore_ascii_case("template") {
                if let Some(frag_id) = el.template_content {
                    for (child_id, _) in doc.children(frag_id) {
                        serialize_node_into(doc, child_id, out);
                    }
                }
            } else {
                for (child_id, _) in doc.children(node_id) {
                    serialize_node_into(doc, child_id, out);
                }
            }

            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        NodeKind::ShadowRoot(_) => {
            // Shadow roots são inertes para serialização comum de documento
        }
    }
}
