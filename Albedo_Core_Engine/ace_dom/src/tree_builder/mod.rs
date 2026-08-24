//! # Construtor da Árvore HTML5 (HTML5 Tree Builder - WHATWG §12.2.6)
//!
//! Recebe tokens do `HTMLTokenizer` e constrói a árvore DOM resolvendo modos de inserção e correções normativas.

pub mod active_formatting;
pub mod adoption_agency;
pub mod insertion_mode;
pub mod open_elements;

pub use active_formatting::ActiveFormattingElements;
pub use adoption_agency::run_adoption_agency_algorithm;
pub use insertion_mode::InsertionMode;
pub use open_elements::StackOfOpenElements;

use crate::node::element::Namespace;
use crate::node::DocumentMode;
use crate::tokenizer::token::Token;
use crate::tokenizer::TokenSink;
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// O construtor oficial da árvore DOM do Albedo Browser.
#[derive(Debug)]
pub struct HTMLTreeBuilder {
    pub doc: Document,
    pub open_elements: StackOfOpenElements,
    pub active_formatting: ActiveFormattingElements,
    pub mode: InsertionMode,
    pub original_mode: Option<InsertionMode>,
    pub head_element: Option<NodeId>,
    pub form_element: Option<NodeId>,
    pub frameset_ok: bool,
}

impl Default for HTMLTreeBuilder {
    fn default() -> Self {
        Self::new(None)
    }
}

impl HTMLTreeBuilder {
    /// Inicializa um novo `HTMLTreeBuilder` para um documento.
    pub fn new(url: Option<&str>) -> Self {
        Self {
            doc: Document::new(url),
            open_elements: StackOfOpenElements::new(),
            active_formatting: ActiveFormattingElements::new(),
            mode: InsertionMode::Initial,
            original_mode: None,
            head_element: None,
            form_element: None,
            frameset_ok: true,
        }
    }

    /// Retorna o documento finalizado após o parsing.
    pub fn finish(self) -> Document {
        self.doc
    }

    /// Insere um elemento na árvore anexando-o ao nó atual no topo da pilha.
    fn insert_element(&mut self, tag_name: impl Into<Atom>, ns: Namespace) -> NodeId {
        let el_id = self.doc.create_element(tag_name, ns);
        let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
        let _ = self.doc.append_child(parent_id, el_id);
        self.open_elements.push(el_id);
        el_id
    }

    /// Insere um nó de texto concatenando com o irmão anterior se já for texto (Text Node Merging).
    fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());

        // Tenta mesclar com o último filho do nó pai se já for TextData
        if let Some(parent_node) = self.doc.get_node(parent_id) {
            if let Some(last_child_id) = parent_node.last_child {
                if let Some(last_child) = self.doc.get_node_mut(last_child_id) {
                    if let crate::node::NodeKind::Text(ref mut t) = last_child.kind {
                        let mut merged = t.data.to_string();
                        merged.push_str(text);
                        t.data = SmolStr::new(merged);
                        return;
                    }
                }
            }
        }

        let text_id = self.doc.create_text_node(text);
        let _ = self.doc.append_child(parent_id, text_id);
    }

    /// Trata elementos vazios (*Void Elements*) que não têm filhos e fecham imediatamente.
    fn is_void_element(tag_name: &str) -> bool {
        matches!(
            tag_name,
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

    /// Trata elementos de formatação (*Formatting Elements*).
    fn is_formatting_element(tag_name: &str) -> bool {
        matches!(
            tag_name,
            "a"
                | "b"
                | "big"
                | "code"
                | "em"
                | "font"
                | "i"
                | "nobr"
                | "s"
                | "small"
                | "strike"
                | "strong"
                | "tt"
                | "u"
        )
    }
}

impl TokenSink for HTMLTreeBuilder {
    fn process_token(&mut self, token: Token) {
        match self.mode {
            // 1. Initial Insertion Mode (§12.2.6.4.1)
            InsertionMode::Initial => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    // Ignora espaços em branco antes do Doctype
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let _ = self.doc.append_child(self.doc.root(), c_id);
                }
                Token::Doctype(doctype) => {
                    let name = doctype.name.unwrap_or_default();
                    let d_id = self.doc.create_doctype(
                        name,
                        doctype.public_identifier,
                        doctype.system_identifier,
                        doctype.force_quirks,
                    );
                    let _ = self.doc.append_child(self.doc.root(), d_id);
                    self.doc.doctype = Some(d_id);

                    if doctype.force_quirks {
                        self.doc.mode = DocumentMode::Quirks;
                    }
                    self.mode = InsertionMode::BeforeHtml;
                }
                other => {
                    self.mode = InsertionMode::BeforeHtml;
                    self.process_token(other);
                }
            },

            // 2. Before HTML Insertion Mode (§12.2.6.4.2)
            InsertionMode::BeforeHtml => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {}
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let _ = self.doc.append_child(self.doc.root(), c_id);
                }
                Token::StartTag(start_tag) if start_tag.name.eq_ignore_ascii_case("html") => {
                    let html_id = self.doc.create_element(start_tag.name, Namespace::Html);
                    if let Some(el) = self.doc.get_node_mut(html_id).and_then(|n| n.as_element_mut()) {
                        for attr in start_tag.attributes.as_slice() {
                            el.set_attribute(attr.name.clone(), attr.value.clone());
                        }
                    }
                    let _ = self.doc.append_child(self.doc.root(), html_id);
                    self.open_elements.push(html_id);
                    self.doc.document_element = Some(html_id);
                    self.mode = InsertionMode::BeforeHead;
                }
                other => {
                    // Cria tag <html> implícita
                    let html_id = self.doc.create_element("html", Namespace::Html);
                    let _ = self.doc.append_child(self.doc.root(), html_id);
                    self.open_elements.push(html_id);
                    self.doc.document_element = Some(html_id);
                    self.mode = InsertionMode::BeforeHead;
                    self.process_token(other);
                }
            },

            // 3. Before Head Insertion Mode (§12.2.6.4.3)
            InsertionMode::BeforeHead => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {}
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                }
                Token::StartTag(start_tag) if start_tag.name.eq_ignore_ascii_case("head") => {
                    let head_id = self.insert_element(start_tag.name, Namespace::Html);
                    if let Some(el) = self.doc.get_node_mut(head_id).and_then(|n| n.as_element_mut()) {
                        for attr in start_tag.attributes.as_slice() {
                            el.set_attribute(attr.name.clone(), attr.value.clone());
                        }
                    }
                    self.head_element = Some(head_id);
                    self.doc.head = Some(head_id);
                    self.mode = InsertionMode::InHead;
                }
                other => {
                    // Cria <head> implícito
                    let head_id = self.insert_element("head", Namespace::Html);
                    self.head_element = Some(head_id);
                    self.doc.head = Some(head_id);
                    self.mode = InsertionMode::InHead;
                    self.process_token(other);
                }
            },

            // 4. In Head Insertion Mode (§12.2.6.4.4)
            InsertionMode::InHead => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    self.insert_text(s.as_str());
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                }
                Token::StartTag(start_tag) => {
                    let tag = start_tag.name.as_str();
                    if matches!(tag, "meta" | "link" | "base" | "basefont" | "bgsound") {
                        let el_id = self.doc.create_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                        let _ = self.doc.append_child(parent_id, el_id);
                    } else if matches!(tag, "title" | "style" | "script" | "noscript") {
                        let el_id = self.insert_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                    } else if tag == "head" {
                        // Ignora <head> duplicado
                    } else {
                        // Sai do head
                        self.open_elements.pop_until_tag(&self.doc, "head");
                        self.mode = InsertionMode::AfterHead;
                        self.process_token(Token::StartTag(start_tag));
                    }
                }
                Token::EndTag(end_tag) if end_tag.name.eq_ignore_ascii_case("head") => {
                    self.open_elements.pop_until_tag(&self.doc, "head");
                    self.mode = InsertionMode::AfterHead;
                }
                Token::EndTag(end_tag) if matches!(end_tag.name.as_str(), "title" | "style" | "script") => {
                    self.open_elements.pop_until_tag(&self.doc, end_tag.name.as_str());
                }
                Token::Character(ref s) => {
                    self.insert_text(s.as_str());
                }
                other => {
                    self.open_elements.pop_until_tag(&self.doc, "head");
                    self.mode = InsertionMode::AfterHead;
                    self.process_token(other);
                }
            },

            // 5. After Head Insertion Mode (§12.2.6.4.6)
            InsertionMode::AfterHead => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    self.insert_text(s.as_str());
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                }
                Token::StartTag(start_tag) if start_tag.name.eq_ignore_ascii_case("body") => {
                    let body_id = self.insert_element(start_tag.name, Namespace::Html);
                    if let Some(el) = self.doc.get_node_mut(body_id).and_then(|n| n.as_element_mut()) {
                        for attr in start_tag.attributes.as_slice() {
                            el.set_attribute(attr.name.clone(), attr.value.clone());
                        }
                    }
                    self.doc.body = Some(body_id);
                    self.mode = InsertionMode::InBody;
                }
                other => {
                    // Cria <body> implícito
                    let body_id = self.insert_element("body", Namespace::Html);
                    self.doc.body = Some(body_id);
                    self.mode = InsertionMode::InBody;
                    self.process_token(other);
                }
            },

            // 6. In Body Insertion Mode (§12.2.6.4.7)
            InsertionMode::InBody => match token {
                Token::Character(ref s) => {
                    self.insert_text(s.as_str());
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                }
                Token::StartTag(start_tag) => {
                    let tag_str = start_tag.name.as_str();

                    if Self::is_formatting_element(tag_str) {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        self.active_formatting.push_element(el_id);
                    } else if Self::is_void_element(tag_str) {
                        let el_id = self.doc.create_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                        let _ = self.doc.append_child(parent_id, el_id);
                    } else {
                        // Elementos comuns de bloco e inline (div, p, span, h1-h6, table, form, etc.)
                        if tag_str == "p" && self.open_elements.has_element_in_button_scope(&self.doc, "p") {
                            self.open_elements.pop_until_tag(&self.doc, "p");
                        }

                        let el_id = self.insert_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                    }
                }
                Token::EndTag(end_tag) => {
                    let tag_str = end_tag.name.as_str();

                    if tag_str == "body" {
                        if self.open_elements.has_element_in_scope(&self.doc, "body") {
                            self.mode = InsertionMode::AfterBody;
                        }
                    } else if tag_str == "html" {
                        if self.open_elements.has_element_in_scope(&self.doc, "body") {
                            self.mode = InsertionMode::AfterBody;
                            self.process_token(Token::EndTag(end_tag));
                        }
                    } else if Self::is_formatting_element(tag_str) {
                        // Executa o Adoption Agency Algorithm (AAA)
                        run_adoption_agency_algorithm(
                            &mut self.doc,
                            &mut self.open_elements,
                            &mut self.active_formatting,
                            tag_str,
                        );
                    } else if self.open_elements.has_element_in_scope(&self.doc, tag_str) {
                        self.open_elements.pop_until_tag(&self.doc, tag_str);
                    }
                }
                Token::Eof => {
                    // Finaliza parsing
                }
                _ => {}
            },

            // 7. After Body Insertion Mode (§12.2.6.4.18)
            InsertionMode::AfterBody => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    self.insert_text(s.as_str());
                }
                Token::EndTag(end_tag) if end_tag.name.eq_ignore_ascii_case("html") => {
                    self.mode = InsertionMode::AfterAfterBody;
                }
                Token::Eof => {}
                other => {
                    self.mode = InsertionMode::InBody;
                    self.process_token(other);
                }
            },

            // 8. After After Body (§12.2.6.4.21)
            InsertionMode::AfterAfterBody => match token {
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let _ = self.doc.append_child(self.doc.root(), c_id);
                }
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {}
                Token::Eof => {}
                other => {
                    self.mode = InsertionMode::InBody;
                    self.process_token(other);
                }
            },

            _ => {
                // Fallback para InBody
                self.mode = InsertionMode::InBody;
                self.process_token(token);
            }
        }
    }
}
