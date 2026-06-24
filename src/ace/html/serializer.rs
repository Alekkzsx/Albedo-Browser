use std::fmt::Write;
use super::types::{HtmlDocument, HtmlNode, HtmlElement, DoctypeToken, is_void_element};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializeOptions {
    pub indent: Option<usize>,
    pub escape_text: bool,
}

impl Default for SerializeOptions {
    /// TODO: add docs
    fn default() -> Self {
        Self {
            indent: None,
            escape_text: true,
        }
    }
}

/// TODO: add docs
pub fn serialize_document(doc: &HtmlDocument, options: &SerializeOptions) -> String {
    let mut out = String::new();
    
    if let Some(ref doctype) = doc.doctype {
        serialize_doctype(doctype, &mut out);
    }
    
    for (i, node) in doc.children.iter().enumerate() {
        if i > 0 && options.indent.is_some() {
            out.push('\n');
        }
        serialize_node_recursive(node, 0, options, &mut out);
    }
    
    out
}

/// TODO: add docs
pub fn serialize_node(node: &HtmlNode, options: &SerializeOptions) -> String {
    let mut out = String::new();
    serialize_node_recursive(node, 0, options, &mut out);
    out
}

/// TODO: add docs
fn serialize_doctype(doctype: &DoctypeToken, out: &mut String) {
    out.push_str("<!DOCTYPE");
    if let Some(ref name) = doctype.name {
        write!(out, " {}", name).expect("Albedo Engine: internal invariant violated");
    }
    if let Some(ref public) = doctype.public_id {
        write!(out, " PUBLIC \"{}\"", public).expect("Albedo Engine: internal invariant violated");
    }
    if let Some(ref system) = doctype.system_id {
        if doctype.public_id.is_none() {
            write!(out, " SYSTEM").expect("Albedo Engine: internal invariant violated");
        }
        write!(out, " \"{}\"", system).expect("Albedo Engine: internal invariant violated");
    }
    out.push_str(">\n");
}

/// TODO: add docs
fn serialize_node_recursive(
    node: &HtmlNode,
    depth: usize,
    options: &SerializeOptions,
    out: &mut String,
) {
    if let Some(indent_size) = options.indent {
        out.push_str(&" ".repeat(depth * indent_size));
    }

    match node {
        HtmlNode::Text(text) => {
            if options.escape_text {
                out.push_str(&escape_html(text));
            } else {
                out.push_str(text);
            }
        }
        HtmlNode::Comment(text) => {
            write!(out, "<!--{}-->", text).expect("Albedo Engine: internal invariant violated");
        }
        HtmlNode::Element(el) => {
            serialize_element(el, depth, options, out);
        }
    }
}

/// TODO: add docs
fn serialize_element(
    el: &HtmlElement,
    depth: usize,
    options: &SerializeOptions,
    out: &mut String,
) {
    write!(out, "<{}", el.tag).expect("Albedo Engine: internal invariant violated");
    
    let mut attrs: Vec<_> = el.attributes.iter().collect();
    attrs.sort_by(|a, b| a.0.cmp(b.0));
    
    for (name, value) in attrs {
        write!(out, " {}=\"{}\"", name, escape_attr(value)).expect("Albedo Engine: internal invariant violated");
    }
    
    if is_void_element(&el.tag) {
        out.push_str(">");
        return;
    }
    
    out.push_str(">");
    
    let has_children = !el.children.is_empty();
    let is_pretty = options.indent.is_some();
    
    if has_children && is_pretty {
        out.push('\n');
    }
    
    for child in &el.children {
        serialize_node_recursive(child, depth + 1, options, out);
        if is_pretty {
            out.push('\n');
        }
    }
    
    if is_pretty && has_children {
        out.push_str(&" ".repeat(depth * options.indent.expect("Albedo Engine: internal invariant violated")));
    }
    
    write!(out, "</{}>", el.tag).expect("Albedo Engine: internal invariant violated");
}



/// TODO: add docs
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// TODO: add docs
fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}
