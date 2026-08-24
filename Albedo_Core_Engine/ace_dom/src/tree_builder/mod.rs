//! # Construtor da Árvore HTML5 (HTML5 Tree Builder - WHATWG §12.2.6)
//!
//! Recebe tokens do `HTMLTokenizer` e constrói a árvore DOM resolvendo modos de inserção,
//! Foster Parenting, tabelas, tags <template>, Foreign Content (SVG/MathML) e transições de estado.

pub mod active_formatting;
pub mod adoption_agency;
pub mod foreign;
pub mod insertion_mode;
pub mod open_elements;

pub use active_formatting::ActiveFormattingElements;
pub use adoption_agency::run_adoption_agency_algorithm;
pub use foreign::{adjust_svg_attribute_name, adjust_svg_tag_name};
pub use insertion_mode::InsertionMode;
pub use open_elements::StackOfOpenElements;

use crate::node::element::Namespace;
use crate::node::DocumentMode;
use crate::tokenizer::token::Token;
use crate::tokenizer::{TokenSink, TokenizerAction, TokenizerState};
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
    pub template_insertion_modes: Vec<InsertionMode>,
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
            template_insertion_modes: Vec::new(),
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

    /// Insere um elemento na árvore anexando-o ao nó pai apropriado (ou `template_content`).
    fn insert_element(&mut self, tag_name: impl Into<Atom>, ns: Namespace) -> NodeId {
        let el_id = self.doc.create_element(tag_name, ns);
        let parent_id = self.current_insertion_target();
        let _ = self.doc.append_child(parent_id, el_id);
        self.open_elements.push(el_id);
        el_id
    }

    /// Retorna o alvo de inserção atual (respeitando o `template_content` de tags `<template>`).
    fn current_insertion_target(&self) -> NodeId {
        if let Some(curr_id) = self.open_elements.current_node() {
            if let Some(curr_node) = self.doc.get_node(curr_id) {
                if let Some(el) = curr_node.as_element() {
                    if el.tag_name.eq_ignore_ascii_case("template") {
                        if let Some(frag_id) = el.template_content {
                            return frag_id;
                        }
                    }
                }
            }
            curr_id
        } else {
            self.doc.root()
        }
    }

    /// Insere um nó de texto concatenando com o irmão anterior se já for texto (Text Node Merging).
    fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let parent_id = self.current_insertion_target();

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

    /// Executa Foster Parenting (WHATWG §12.2.6.1): insere o nó imediatamente antes da tabela mais próxima no DOM.
    fn foster_parent_node(&mut self, node_id: NodeId) {
        let stack = self.open_elements.as_slice();
        let table_pos = stack.iter().rposition(|&id| {
            self.doc
                .get_node(id)
                .and_then(|n| n.tag_name())
                .is_some_and(|t| t.eq_ignore_ascii_case("table"))
        });

        if let Some(pos) = table_pos {
            let table_id = stack[pos];
            if let Some(parent_id) = self.doc.get_node(table_id).and_then(|n| n.parent) {
                let _ = self.doc.insert_before(parent_id, node_id, Some(table_id));
                return;
            }
        }

        let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
        let _ = self.doc.append_child(parent_id, node_id);
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
    fn process_token(&mut self, token: Token) -> TokenizerAction {
        match self.mode {
            // 1. Initial Insertion Mode (§12.2.6.4.1)
            InsertionMode::Initial => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    TokenizerAction::Continue
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let _ = self.doc.append_child(self.doc.root(), c_id);
                    TokenizerAction::Continue
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
                    TokenizerAction::Continue
                }
                other => {
                    self.mode = InsertionMode::BeforeHtml;
                    self.process_token(other)
                }
            },

            // 2. Before HTML Insertion Mode (§12.2.6.4.2)
            InsertionMode::BeforeHtml => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    TokenizerAction::Continue
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let _ = self.doc.append_child(self.doc.root(), c_id);
                    TokenizerAction::Continue
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
                    TokenizerAction::Continue
                }
                other => {
                    let html_id = self.doc.create_element("html", Namespace::Html);
                    let _ = self.doc.append_child(self.doc.root(), html_id);
                    self.open_elements.push(html_id);
                    self.doc.document_element = Some(html_id);
                    self.mode = InsertionMode::BeforeHead;
                    self.process_token(other)
                }
            },

            // 3. Before Head Insertion Mode (§12.2.6.4.3)
            InsertionMode::BeforeHead => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    TokenizerAction::Continue
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                    TokenizerAction::Continue
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
                    TokenizerAction::Continue
                }
                other => {
                    let head_id = self.insert_element("head", Namespace::Html);
                    self.head_element = Some(head_id);
                    self.doc.head = Some(head_id);
                    self.mode = InsertionMode::InHead;
                    self.process_token(other)
                }
            },

            // 4. In Head Insertion Mode (§12.2.6.4.4)
            InsertionMode::InHead => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    self.insert_text(s.as_str());
                    TokenizerAction::Continue
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                    TokenizerAction::Continue
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
                        TokenizerAction::Continue
                    } else if matches!(tag, "title" | "style" | "script" | "noscript") {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }

                        if tag == "title" {
                            TokenizerAction::SwitchState(TokenizerState::RCDATA)
                        } else if tag == "style" {
                            TokenizerAction::SwitchState(TokenizerState::RAWTEXT)
                        } else if tag == "script" {
                            TokenizerAction::SwitchState(TokenizerState::ScriptData)
                        } else {
                            TokenizerAction::Continue
                        }
                    } else if tag == "template" {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                        let frag_id = self.doc.create_document_fragment();
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            el.template_content = Some(frag_id);
                        }
                        self.template_insertion_modes.push(InsertionMode::InTemplate);
                        self.mode = InsertionMode::InTemplate;
                        TokenizerAction::Continue
                    } else if tag == "head" {
                        TokenizerAction::Continue
                    } else {
                        self.open_elements.pop_until_tag(&self.doc, "head");
                        self.mode = InsertionMode::AfterHead;
                        self.process_token(Token::StartTag(start_tag))
                    }
                }
                Token::EndTag(end_tag) if end_tag.name.eq_ignore_ascii_case("head") => {
                    self.open_elements.pop_until_tag(&self.doc, "head");
                    self.mode = InsertionMode::AfterHead;
                    TokenizerAction::Continue
                }
                Token::EndTag(end_tag) if matches!(end_tag.name.as_str(), "title" | "style" | "script") => {
                    self.open_elements.pop_until_tag(&self.doc, end_tag.name.as_str());
                    TokenizerAction::Continue
                }
                Token::Character(ref s) => {
                    self.insert_text(s.as_str());
                    TokenizerAction::Continue
                }
                other => {
                    self.open_elements.pop_until_tag(&self.doc, "head");
                    self.mode = InsertionMode::AfterHead;
                    self.process_token(other)
                }
            },

            // 5. After Head Insertion Mode (§12.2.6.4.6)
            InsertionMode::AfterHead => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    self.insert_text(s.as_str());
                    TokenizerAction::Continue
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                    TokenizerAction::Continue
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
                    TokenizerAction::Continue
                }
                other => {
                    let body_id = self.insert_element("body", Namespace::Html);
                    self.doc.body = Some(body_id);
                    self.mode = InsertionMode::InBody;
                    self.process_token(other)
                }
            },

            // 6. In Body Insertion Mode (§12.2.6.4.7)
            InsertionMode::InBody => match token {
                Token::Character(ref s) => {
                    self.insert_text(s.as_str());
                    TokenizerAction::Continue
                }
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                    let _ = self.doc.append_child(parent_id, c_id);
                    TokenizerAction::Continue
                }
                Token::StartTag(start_tag) => {
                    let tag_str = start_tag.name.as_str();

                    if tag_str.eq_ignore_ascii_case("svg") {
                        let adj_tag = adjust_svg_tag_name(tag_str);
                        let el_id = self.insert_element(adj_tag, Namespace::Svg);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                let adj_name = adjust_svg_attribute_name(attr.name.as_str());
                                el.set_attribute(adj_name, attr.value.clone());
                            }
                        }
                        TokenizerAction::Continue
                    } else if tag_str.eq_ignore_ascii_case("math") {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::MathMl);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        TokenizerAction::Continue
                    } else if tag_str == "table" {
                        if self.open_elements.has_element_in_button_scope(&self.doc, "p") {
                            self.open_elements.pop_until_tag(&self.doc, "p");
                        }
                        let el_id = self.insert_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        self.mode = InsertionMode::InTable;
                        TokenizerAction::Continue
                    } else if tag_str == "template" {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                        let frag_id = self.doc.create_document_fragment();
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            el.template_content = Some(frag_id);
                        }
                        self.template_insertion_modes.push(InsertionMode::InTemplate);
                        self.mode = InsertionMode::InTemplate;
                        TokenizerAction::Continue
                    } else if matches!(tag_str, "style" | "script" | "textarea") {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }

                        if tag_str == "textarea" {
                            TokenizerAction::SwitchState(TokenizerState::RCDATA)
                        } else if tag_str == "style" {
                            TokenizerAction::SwitchState(TokenizerState::RAWTEXT)
                        } else {
                            TokenizerAction::SwitchState(TokenizerState::ScriptData)
                        }
                    } else if Self::is_formatting_element(tag_str) {
                        let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        self.active_formatting.push_element(&self.doc, el_id);
                        TokenizerAction::Continue
                    } else if Self::is_void_element(tag_str) {
                        let el_id = self.doc.create_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        let parent_id = self.open_elements.current_node().unwrap_or(self.doc.root());
                        let _ = self.doc.append_child(parent_id, el_id);
                        TokenizerAction::Continue
                    } else {
                        if tag_str == "p" && self.open_elements.has_element_in_button_scope(&self.doc, "p") {
                            self.open_elements.pop_until_tag(&self.doc, "p");
                        }

                        let el_id = self.insert_element(start_tag.name, Namespace::Html);
                        if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                            for attr in start_tag.attributes.as_slice() {
                                el.set_attribute(attr.name.clone(), attr.value.clone());
                            }
                        }
                        TokenizerAction::Continue
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
                            return self.process_token(Token::EndTag(end_tag));
                        }
                    } else if tag_str == "template" {
                        self.open_elements.pop_until_tag(&self.doc, "template");
                        self.template_insertion_modes.pop();
                        self.mode = self.template_insertion_modes.last().copied().unwrap_or(InsertionMode::InBody);
                    } else if Self::is_formatting_element(tag_str) {
                        run_adoption_agency_algorithm(
                            &mut self.doc,
                            &mut self.open_elements,
                            &mut self.active_formatting,
                            tag_str,
                        );
                    } else if self.open_elements.has_element_in_scope(&self.doc, tag_str) {
                        self.open_elements.pop_until_tag(&self.doc, tag_str);
                    }
                    TokenizerAction::Continue
                }
                Token::Eof => TokenizerAction::Continue,
                _ => TokenizerAction::Continue,
            },

            // 7. In Table Insertion Mode (§12.2.6.4.9)
            InsertionMode::InTable => match token {
                Token::StartTag(ref start_tag) if matches!(start_tag.name.as_str(), "tbody" | "thead" | "tfoot") => {
                    let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                    if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                        for attr in start_tag.attributes.as_slice() {
                            el.set_attribute(attr.name.clone(), attr.value.clone());
                        }
                    }
                    self.mode = InsertionMode::InTableBody;
                    TokenizerAction::Continue
                }
                Token::StartTag(ref start_tag) if start_tag.name.eq_ignore_ascii_case("tr") => {
                    let tbody_id = self.insert_element("tbody", Namespace::Html);
                    self.mode = InsertionMode::InTableBody;
                    let _ = tbody_id;
                    self.process_token(token)
                }
                Token::StartTag(ref start_tag) if matches!(start_tag.name.as_str(), "td" | "th") => {
                    let tbody_id = self.insert_element("tbody", Namespace::Html);
                    self.mode = InsertionMode::InTableBody;
                    let _ = tbody_id;
                    self.process_token(token)
                }
                Token::EndTag(ref end_tag) if end_tag.name.eq_ignore_ascii_case("table") => {
                    if self.open_elements.has_element_in_table_scope(&self.doc, "table") {
                        self.open_elements.pop_until_tag(&self.doc, "table");
                        self.mode = InsertionMode::InBody;
                    }
                    TokenizerAction::Continue
                }
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    TokenizerAction::Continue
                }
                other => {
                    match other {
                        Token::Character(ref s) => {
                            let text_id = self.doc.create_text_node(s.as_str());
                            self.foster_parent_node(text_id);
                            TokenizerAction::Continue
                        }
                        Token::StartTag(ref start_tag) => {
                            let el_id = self.doc.create_element(start_tag.name.clone(), Namespace::Html);
                            if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                                for attr in start_tag.attributes.as_slice() {
                                    el.set_attribute(attr.name.clone(), attr.value.clone());
                                }
                            }
                            self.foster_parent_node(el_id);
                            self.open_elements.push(el_id);
                            self.mode = InsertionMode::InBody;
                            TokenizerAction::Continue
                        }
                        _ => TokenizerAction::Continue,
                    }
                }
            },

            // 8. In Table Body Insertion Mode (§12.2.6.4.11)
            InsertionMode::InTableBody => match token {
                Token::StartTag(ref start_tag) if start_tag.name.eq_ignore_ascii_case("tr") => {
                    let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                    if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                        for attr in start_tag.attributes.as_slice() {
                            el.set_attribute(attr.name.clone(), attr.value.clone());
                        }
                    }
                    self.mode = InsertionMode::InRow;
                    TokenizerAction::Continue
                }
                Token::StartTag(ref start_tag) if matches!(start_tag.name.as_str(), "td" | "th") => {
                    let tr_id = self.insert_element("tr", Namespace::Html);
                    self.mode = InsertionMode::InRow;
                    let _ = tr_id;
                    self.process_token(token)
                }
                Token::EndTag(ref end_tag) if matches!(end_tag.name.as_str(), "tbody" | "thead" | "tfoot") => {
                    self.open_elements.pop_until_tag(&self.doc, end_tag.name.as_str());
                    self.mode = InsertionMode::InTable;
                    TokenizerAction::Continue
                }
                Token::EndTag(ref end_tag) if end_tag.name.eq_ignore_ascii_case("table") => {
                    self.open_elements.pop_until_tag(&self.doc, "tbody");
                    self.mode = InsertionMode::InTable;
                    self.process_token(token)
                }
                other => {
                    self.mode = InsertionMode::InBody;
                    self.process_token(other)
                }
            },

            // 9. In Row Insertion Mode (§12.2.6.4.12)
            InsertionMode::InRow => match token {
                Token::StartTag(ref start_tag) if matches!(start_tag.name.as_str(), "td" | "th") => {
                    let el_id = self.insert_element(start_tag.name.clone(), Namespace::Html);
                    if let Some(el) = self.doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
                        for attr in start_tag.attributes.as_slice() {
                            el.set_attribute(attr.name.clone(), attr.value.clone());
                        }
                    }
                    self.mode = InsertionMode::InCell;
                    TokenizerAction::Continue
                }
                Token::EndTag(ref end_tag) if end_tag.name.eq_ignore_ascii_case("tr") => {
                    self.open_elements.pop_until_tag(&self.doc, "tr");
                    self.mode = InsertionMode::InTableBody;
                    TokenizerAction::Continue
                }
                Token::EndTag(ref end_tag) if matches!(end_tag.name.as_str(), "tbody" | "thead" | "tfoot" | "table") => {
                    self.open_elements.pop_until_tag(&self.doc, "tr");
                    self.mode = InsertionMode::InTableBody;
                    self.process_token(token)
                }
                other => {
                    self.mode = InsertionMode::InBody;
                    self.process_token(other)
                }
            },

            // 10. In Cell Insertion Mode (§12.2.6.4.13)
            InsertionMode::InCell => match token {
                Token::EndTag(ref end_tag) if matches!(end_tag.name.as_str(), "td" | "th") => {
                    self.open_elements.pop_until_tag(&self.doc, end_tag.name.as_str());
                    self.mode = InsertionMode::InRow;
                    TokenizerAction::Continue
                }
                Token::StartTag(ref start_tag) if matches!(start_tag.name.as_str(), "td" | "th" | "tr") => {
                    self.open_elements.pop_until_tag(&self.doc, "td");
                    self.mode = InsertionMode::InRow;
                    self.process_token(token)
                }
                Token::EndTag(ref end_tag) if matches!(end_tag.name.as_str(), "tr" | "tbody" | "thead" | "tfoot" | "table") => {
                    self.open_elements.pop_until_tag(&self.doc, "td");
                    self.mode = InsertionMode::InRow;
                    self.process_token(token)
                }
                other => {
                    let prev_mode = self.mode;
                    self.mode = InsertionMode::InBody;
                    let act = self.process_token(other);
                    self.mode = prev_mode;
                    act
                }
            },

            // 11. In Template Insertion Mode (§12.2.6.4.20)
            InsertionMode::InTemplate => match token {
                Token::EndTag(ref end_tag) if end_tag.name.eq_ignore_ascii_case("template") => {
                    self.open_elements.pop_until_tag(&self.doc, "template");
                    self.template_insertion_modes.pop();
                    self.mode = self.template_insertion_modes.last().copied().unwrap_or(InsertionMode::InBody);
                    TokenizerAction::Continue
                }
                Token::StartTag(ref start_tag) => {
                    let tag = start_tag.name.as_str();
                    if matches!(tag, "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title") {
                        let prev_mode = self.mode;
                        self.mode = InsertionMode::InHead;
                        let act = self.process_token(token);
                        self.mode = prev_mode;
                        act
                    } else if matches!(tag, "caption" | "colgroup" | "tbody" | "tfoot" | "thead") {
                        let prev_mode = self.mode;
                        self.mode = InsertionMode::InTable;
                        let act = self.process_token(token);
                        self.mode = prev_mode;
                        act
                    } else if tag == "tr" {
                        let prev_mode = self.mode;
                        self.mode = InsertionMode::InTableBody;
                        let act = self.process_token(token);
                        self.mode = prev_mode;
                        act
                    } else if matches!(tag, "td" | "th") {
                        let prev_mode = self.mode;
                        self.mode = InsertionMode::InRow;
                        let act = self.process_token(token);
                        self.mode = prev_mode;
                        act
                    } else {
                        let prev_mode = self.mode;
                        self.mode = InsertionMode::InBody;
                        let act = self.process_token(token);
                        self.mode = prev_mode;
                        act
                    }
                }
                Token::Character(_) => {
                    let prev_mode = self.mode;
                    self.mode = InsertionMode::InBody;
                    let act = self.process_token(token);
                    self.mode = prev_mode;
                    act
                }
                Token::Eof => {
                    if self.open_elements.has_element_in_scope(&self.doc, "template") {
                        self.open_elements.pop_until_tag(&self.doc, "template");
                        self.template_insertion_modes.pop();
                        self.mode = self.template_insertion_modes.last().copied().unwrap_or(InsertionMode::InBody);
                    }
                    TokenizerAction::Continue
                }
                _ => TokenizerAction::Continue,
            },

            // 12. After Body Insertion Mode (§12.2.6.4.18)
            InsertionMode::AfterBody => match token {
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    self.insert_text(s.as_str());
                    TokenizerAction::Continue
                }
                Token::EndTag(end_tag) if end_tag.name.eq_ignore_ascii_case("html") => {
                    self.mode = InsertionMode::AfterAfterBody;
                    TokenizerAction::Continue
                }
                Token::Eof => TokenizerAction::Continue,
                other => {
                    self.mode = InsertionMode::InBody;
                    self.process_token(other)
                }
            },

            // 13. After After Body (§12.2.6.4.21)
            InsertionMode::AfterAfterBody => match token {
                Token::Comment(ref s) => {
                    let c_id = self.doc.create_comment(s.as_str());
                    let _ = self.doc.append_child(self.doc.root(), c_id);
                    TokenizerAction::Continue
                }
                Token::Character(ref s) if s.chars().all(|c| c.is_ascii_whitespace()) => {
                    TokenizerAction::Continue
                }
                Token::Eof => TokenizerAction::Continue,
                other => {
                    self.mode = InsertionMode::InBody;
                    self.process_token(other)
                }
            },

            _ => {
                self.mode = InsertionMode::InBody;
                self.process_token(token)
            }
        }
    }
}
