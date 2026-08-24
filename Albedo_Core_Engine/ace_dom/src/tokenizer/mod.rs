//! # Máquina de Estados Finita do Tokenizer HTML5 (WHATWG §12.2.5)
//!
//! Processamento de streaming de alto desempenho desacoplado via trait `TokenSink`.

pub mod state;
pub mod token;

pub use state::TokenizerState;
pub use token::{DoctypeToken, EndTagToken, StartTagToken, Token};

use crate::entities::decode_character_reference;
use crate::node::element::Attribute;
use ace_core::collections::InlineVec;
use ace_core::intern::Atom;
use ace_core::text::SegmentedString;
use smol_str::SmolStr;

/// Ação ou transição de estado requisitada pelo receptor de tokens (TreeBuilder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TokenizerAction {
    #[default]
    Continue,
    SwitchState(TokenizerState),
}

/// Receptor de tokens emitidos pelo Tokenizer (implementado pelo `HTMLTreeBuilder`).
pub trait TokenSink {
    fn process_token(&mut self, token: Token) -> TokenizerAction;
}

/// Helper para espiar os próximos `n` caracteres do `SegmentedString` sem avançar.
fn peek_str(input: &SegmentedString, n: usize) -> String {
    let mut s = String::with_capacity(n);
    for i in 0..n {
        if let Some(c) = input.peek_at(i) {
            s.push(c);
        } else {
            break;
        }
    }
    s
}

/// O Tokenizer oficial HTML5 do Albedo Browser.
#[derive(Debug)]
pub struct HTMLTokenizer {
    pub state: TokenizerState,
    current_tag_name: String,
    current_tag_is_end: bool,
    current_tag_self_closing: bool,
    current_attributes: InlineVec<Attribute, 4>,
    current_attr_name: String,
    current_attr_value: String,
    current_comment: String,
    current_doctype: DoctypeToken,
    last_start_tag_name: Option<Atom>,
}

impl Default for HTMLTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl HTMLTokenizer {
    /// Cria um novo `HTMLTokenizer` inicializado no `DataState`.
    pub fn new() -> Self {
        Self {
            state: TokenizerState::Data,
            current_tag_name: String::with_capacity(32),
            current_tag_is_end: false,
            current_tag_self_closing: false,
            current_attributes: InlineVec::new(),
            current_attr_name: String::with_capacity(32),
            current_attr_value: String::with_capacity(64),
            current_comment: String::with_capacity(64),
            current_doctype: DoctypeToken::default(),
            last_start_tag_name: None,
        }
    }

    /// Emite a tag atual para o `TokenSink` (como `StartTag` ou `EndTag`).
    fn emit_current_tag(&mut self, sink: &mut dyn TokenSink) {
        let tag_atom = Atom::new(&self.current_tag_name.to_ascii_lowercase());

        let action = if self.current_tag_is_end {
            sink.process_token(Token::EndTag(EndTagToken { name: tag_atom }))
        } else {
            self.last_start_tag_name = Some(tag_atom.clone());
            let mut attrs = InlineVec::new();
            for attr in self.current_attributes.as_slice() {
                attrs.push(attr.clone());
            }

            sink.process_token(Token::StartTag(StartTagToken {
                name: tag_atom,
                self_closing: self.current_tag_self_closing,
                attributes: attrs,
            }))
        };

        if let TokenizerAction::SwitchState(new_state) = action {
            self.state = new_state;
        }

        self.current_tag_name.clear();
        self.current_tag_is_end = false;
        self.current_tag_self_closing = false;
        self.current_attributes.clear();
        self.current_attr_name.clear();
        self.current_attr_value.clear();
    }

    /// Finaliza o atributo atual e adiciona à lista de atributos da tag em construção.
    fn flush_attribute(&mut self) {
        if !self.current_attr_name.is_empty() {
            let attr_name = Atom::new(&self.current_attr_name.to_ascii_lowercase());
            let attr_value = SmolStr::new(&self.current_attr_value);

            // Não duplica atributos já presentes na mesma tag (primeiro vence conforme WHATWG)
            let already_exists = self
                .current_attributes
                .as_slice()
                .iter()
                .any(|a| a.name == attr_name);

            if !already_exists {
                self.current_attributes.push(Attribute {
                    name: attr_name,
                    value: attr_value,
                });
            }

            self.current_attr_name.clear();
            self.current_attr_value.clear();
        }
    }

    /// Executa a tokenização completa sobre um `SegmentedString`, emitindo tokens continuamente para o `TokenSink`.
    pub fn tokenize(&mut self, input: &mut SegmentedString, sink: &mut dyn TokenSink) {
        while let Some(ch) = input.advance() {
            match self.state {
                // 1. Data State (WHATWG §12.2.5.1)
                TokenizerState::Data => match ch {
                    '&' => {
                        let remaining = peek_str(input, 32);
                        if let Some((decoded, consumed)) = decode_character_reference(&remaining) {
                            for _ in 0..consumed {
                                input.advance();
                            }
                            sink.process_token(Token::Character(decoded));
                        } else {
                            sink.process_token(Token::Character(SmolStr::new("&")));
                        }
                    }
                    '<' => {
                        self.state = TokenizerState::TagOpen;
                    }
                    '\0' => {
                        sink.process_token(Token::Character(SmolStr::new("\u{FFFD}")));
                    }
                    other => {
                        sink.process_token(Token::Character(SmolStr::new(other.to_string())));
                    }
                },

                // 2. Tag Open State (§12.2.5.6)
                TokenizerState::TagOpen => match ch {
                    '!' => {
                        self.state = TokenizerState::MarkupDeclarationOpen;
                    }
                    '/' => {
                        self.state = TokenizerState::EndTagOpen;
                    }
                    '?' => {
                        self.current_comment.clear();
                        self.current_comment.push(ch);
                        self.state = TokenizerState::BogusComment;
                    }
                    c if c.is_ascii_alphabetic() => {
                        self.current_tag_name.clear();
                        self.current_tag_name.push(c);
                        self.current_tag_is_end = false;
                        self.state = TokenizerState::TagName;
                    }
                    other => {
                        sink.process_token(Token::Character(SmolStr::new("<")));
                        input.push_front_char(other);
                        self.state = TokenizerState::Data;
                    }
                },

                // 3. End Tag Open State (§12.2.5.7)
                TokenizerState::EndTagOpen => match ch {
                    c if c.is_ascii_alphabetic() => {
                        self.current_tag_name.clear();
                        self.current_tag_name.push(c);
                        self.current_tag_is_end = true;
                        self.state = TokenizerState::TagName;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                    }
                    other => {
                        self.current_comment.clear();
                        self.current_comment.push(other);
                        self.state = TokenizerState::BogusComment;
                    }
                },

                // 4. Tag Name State (§12.2.5.8)
                TokenizerState::TagName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                    '/' => {
                        self.state = TokenizerState::SelfClosingStartTag;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    '\0' => {
                        self.current_tag_name.push('\u{FFFD}');
                    }
                    c => {
                        self.current_tag_name.push(c);
                    }
                },

                // 5. Before Attribute Name State (§12.2.5.32)
                TokenizerState::BeforeAttributeName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        // Ignora espaços em branco
                    }
                    '/' => {
                        self.state = TokenizerState::SelfClosingStartTag;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    '=' => {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch);
                        self.current_attr_value.clear();
                        self.state = TokenizerState::AttributeName;
                    }
                    '\0' => {
                        self.current_attr_name.clear();
                        self.current_attr_name.push('\u{FFFD}');
                        self.current_attr_value.clear();
                        self.state = TokenizerState::AttributeName;
                    }
                    c => {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(c);
                        self.current_attr_value.clear();
                        self.state = TokenizerState::AttributeName;
                    }
                },

                // 6. Attribute Name State (§12.2.5.33)
                TokenizerState::AttributeName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        self.state = TokenizerState::AfterAttributeName;
                    }
                    '=' => {
                        self.state = TokenizerState::BeforeAttributeValue;
                    }
                    '/' => {
                        self.flush_attribute();
                        self.state = TokenizerState::SelfClosingStartTag;
                    }
                    '>' => {
                        self.flush_attribute();
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    '\0' => {
                        self.current_attr_name.push('\u{FFFD}');
                    }
                    c => {
                        self.current_attr_name.push(c);
                    }
                },

                // 7. After Attribute Name State (§12.2.5.34)
                TokenizerState::AfterAttributeName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        // Ignora espaços
                    }
                    '=' => {
                        self.state = TokenizerState::BeforeAttributeValue;
                    }
                    '/' => {
                        self.flush_attribute();
                        self.state = TokenizerState::SelfClosingStartTag;
                    }
                    '>' => {
                        self.flush_attribute();
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    c => {
                        self.flush_attribute();
                        self.current_attr_name.clear();
                        self.current_attr_name.push(c);
                        self.current_attr_value.clear();
                        self.state = TokenizerState::AttributeName;
                    }
                },

                // 8. Before Attribute Value State (§12.2.5.35)
                TokenizerState::BeforeAttributeValue => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        // Ignora espaços antes do valor
                    }
                    '"' => {
                        self.state = TokenizerState::AttributeValueDoubleQuoted;
                    }
                    '\'' => {
                        self.state = TokenizerState::AttributeValueSingleQuoted;
                    }
                    '>' => {
                        self.flush_attribute();
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    c => {
                        self.current_attr_value.push(c);
                        self.state = TokenizerState::AttributeValueUnquoted;
                    }
                },

                // 9. Attribute Value (Double-Quoted) State (§12.2.5.36)
                TokenizerState::AttributeValueDoubleQuoted => match ch {
                    '"' => {
                        self.flush_attribute();
                        self.state = TokenizerState::AfterAttributeValueQuoted;
                    }
                    '&' => {
                        let remaining = peek_str(input, 32);
                        if let Some((decoded, consumed)) = decode_character_reference(&remaining) {
                            for _ in 0..consumed {
                                input.advance();
                            }
                            self.current_attr_value.push_str(&decoded);
                        } else {
                            self.current_attr_value.push('&');
                        }
                    }
                    '\0' => {
                        self.current_attr_value.push('\u{FFFD}');
                    }
                    c => {
                        self.current_attr_value.push(c);
                    }
                },

                // 10. Attribute Value (Single-Quoted) State (§12.2.5.37)
                TokenizerState::AttributeValueSingleQuoted => match ch {
                    '\'' => {
                        self.flush_attribute();
                        self.state = TokenizerState::AfterAttributeValueQuoted;
                    }
                    '&' => {
                        let remaining = peek_str(input, 32);
                        if let Some((decoded, consumed)) = decode_character_reference(&remaining) {
                            for _ in 0..consumed {
                                input.advance();
                            }
                            self.current_attr_value.push_str(&decoded);
                        } else {
                            self.current_attr_value.push('&');
                        }
                    }
                    '\0' => {
                        self.current_attr_value.push('\u{FFFD}');
                    }
                    c => {
                        self.current_attr_value.push(c);
                    }
                },

                // 11. Attribute Value (Unquoted) State (§12.2.5.38)
                TokenizerState::AttributeValueUnquoted => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        self.flush_attribute();
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                    '&' => {
                        let remaining = peek_str(input, 32);
                        if let Some((decoded, consumed)) = decode_character_reference(&remaining) {
                            for _ in 0..consumed {
                                input.advance();
                            }
                            self.current_attr_value.push_str(&decoded);
                        } else {
                            self.current_attr_value.push('&');
                        }
                    }
                    '>' => {
                        self.flush_attribute();
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    '\0' => {
                        self.current_attr_value.push('\u{FFFD}');
                    }
                    c => {
                        self.current_attr_value.push(c);
                    }
                },

                // 12. After Attribute Value (Quoted) State (§12.2.5.39)
                TokenizerState::AfterAttributeValueQuoted => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                    '/' => {
                        self.state = TokenizerState::SelfClosingStartTag;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    c => {
                        input.push_front_char(c);
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                },

                // 13. Self-Closing Start Tag State (§12.2.5.40)
                TokenizerState::SelfClosingStartTag => match ch {
                    '>' => {
                        self.current_tag_self_closing = true;
                        self.state = TokenizerState::Data;
                        self.emit_current_tag(sink);
                    }
                    c => {
                        input.push_front_char(c);
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                },

                // 14. Markup Declaration Open State (§12.2.5.42)
                TokenizerState::MarkupDeclarationOpen => {
                    input.push_front_char(ch);
                    let prefix = peek_str(input, 7);
                    if prefix.starts_with("--") {
                        for _ in 0..2 {
                            input.advance();
                        }
                        self.current_comment.clear();
                        self.state = TokenizerState::CommentStart;
                    } else if prefix.to_ascii_uppercase().starts_with("DOCTYPE") {
                        for _ in 0..7 {
                            input.advance();
                        }
                        self.current_doctype = DoctypeToken::default();
                        self.state = TokenizerState::Doctype;
                    } else if prefix.starts_with("[CDATA[") {
                        for _ in 0..7 {
                            input.advance();
                        }
                        self.state = TokenizerState::CDataSection;
                    } else {
                        self.current_comment.clear();
                        self.state = TokenizerState::BogusComment;
                    }
                }

                // 15. Comment Start State (§12.2.5.43)
                TokenizerState::CommentStart => match ch {
                    '-' => {
                        self.state = TokenizerState::CommentStartDash;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Comment(SmolStr::new(&self.current_comment)));
                    }
                    c => {
                        self.current_comment.push(c);
                        self.state = TokenizerState::Comment;
                    }
                },

                // 16. Comment Start Dash State (§12.2.5.44)
                TokenizerState::CommentStartDash => match ch {
                    '-' => {
                        self.state = TokenizerState::CommentEnd;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Comment(SmolStr::new(&self.current_comment)));
                    }
                    c => {
                        self.current_comment.push('-');
                        self.current_comment.push(c);
                        self.state = TokenizerState::Comment;
                    }
                },

                // 17. Comment State (§12.2.5.45)
                TokenizerState::Comment => match ch {
                    '-' => {
                        self.state = TokenizerState::CommentEndDash;
                    }
                    '\0' => {
                        self.current_comment.push('\u{FFFD}');
                    }
                    c => {
                        self.current_comment.push(c);
                    }
                },

                // 18. Comment End Dash State (§12.2.5.50)
                TokenizerState::CommentEndDash => match ch {
                    '-' => {
                        self.state = TokenizerState::CommentEnd;
                    }
                    c => {
                        self.current_comment.push('-');
                        self.current_comment.push(c);
                        self.state = TokenizerState::Comment;
                    }
                },

                // 19. Comment End State (§12.2.5.51)
                TokenizerState::CommentEnd => match ch {
                    '>' => {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Comment(SmolStr::new(&self.current_comment)));
                    }
                    '-' => {
                        self.current_comment.push('-');
                    }
                    c => {
                        self.current_comment.push_str("--");
                        self.current_comment.push(c);
                        self.state = TokenizerState::Comment;
                    }
                },

                // 20. Bogus Comment State (§12.2.5.41)
                TokenizerState::BogusComment => match ch {
                    '>' => {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Comment(SmolStr::new(&self.current_comment)));
                    }
                    '\0' => {
                        self.current_comment.push('\u{FFFD}');
                    }
                    c => {
                        self.current_comment.push(c);
                    }
                },

                // 21. Doctype State (§12.2.5.53)
                TokenizerState::Doctype => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        self.state = TokenizerState::BeforeDoctypeName;
                    }
                    '>' => {
                        self.current_doctype.force_quirks = true;
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Doctype(self.current_doctype.clone()));
                    }
                    c => {
                        input.push_front_char(c);
                        self.state = TokenizerState::BeforeDoctypeName;
                    }
                },

                // 22. Before Doctype Name State (§12.2.5.54)
                TokenizerState::BeforeDoctypeName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        // Ignora espaços
                    }
                    '>' => {
                        self.current_doctype.force_quirks = true;
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Doctype(self.current_doctype.clone()));
                    }
                    '\0' => {
                        self.current_doctype.name = Some(SmolStr::new("\u{FFFD}"));
                        self.state = TokenizerState::DoctypeName;
                    }
                    c => {
                        self.current_doctype.name = Some(SmolStr::new(c.to_string()));
                        self.state = TokenizerState::DoctypeName;
                    }
                },

                // 23. Doctype Name State (§12.2.5.55)
                TokenizerState::DoctypeName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        self.state = TokenizerState::AfterDoctypeName;
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Doctype(self.current_doctype.clone()));
                    }
                    '\0' => {
                        if let Some(ref mut name) = self.current_doctype.name {
                            let mut s = name.to_string();
                            s.push('\u{FFFD}');
                            *name = SmolStr::new(s);
                        }
                    }
                    c => {
                        if let Some(ref mut name) = self.current_doctype.name {
                            let mut s = name.to_string();
                            s.push(c);
                            *name = SmolStr::new(s);
                        }
                    }
                },

                // 24. After Doctype Name State (§12.2.5.56)
                TokenizerState::AfterDoctypeName => match ch {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        // Ignora espaços
                    }
                    '>' => {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Doctype(self.current_doctype.clone()));
                    }
                    _ => {
                        self.state = TokenizerState::BogusDoctype;
                    }
                },

                // 25. Bogus Doctype State (§12.2.5.67)
                TokenizerState::BogusDoctype => {
                    if ch == '>' {
                        self.state = TokenizerState::Data;
                        sink.process_token(Token::Doctype(self.current_doctype.clone()));
                    }
                }

                // 26. CDATA Section State (§12.2.5.68)
                TokenizerState::CDataSection => match ch {
                    ']' => {
                        self.state = TokenizerState::CDataSectionBracket;
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                    }
                },

                TokenizerState::CDataSectionBracket => match ch {
                    ']' => {
                        self.state = TokenizerState::CDataSectionEnd;
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new("]")));
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                        self.state = TokenizerState::CDataSection;
                    }
                },

                TokenizerState::CDataSectionEnd => match ch {
                    '>' => {
                        self.state = TokenizerState::Data;
                    }
                    ']' => {
                        sink.process_token(Token::Character(SmolStr::new("]")));
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new("]]")));
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                        self.state = TokenizerState::CDataSection;
                    }
                },

                // 27. RAWTEXT State (WHATWG §12.2.5.3 - ex: <style>, <iframe>, <xmp>)
                TokenizerState::RAWTEXT => match ch {
                    '<' => {
                        if let Some(expected_tag) = &self.last_start_tag_name {
                            let expected_str = expected_tag.as_str();
                            let lookahead_len = 1 + expected_str.len() + 1; // '/' + tag + '>'
                            let peeked = peek_str(input, lookahead_len);

                            if let Some(after_slash) = peeked.strip_prefix('/') {
                                if after_slash.to_ascii_lowercase().starts_with(expected_str) {
                                    let remainder = &after_slash[expected_str.len()..];
                                    if remainder.starts_with('>') || remainder.starts_with(' ') || remainder.starts_with('\t') || remainder.starts_with('\n') || remainder.starts_with('/') {
                                        // Consome '/' e o nome da tag
                                        input.advance(); // consome '/'
                                        for _ in 0..expected_str.len() {
                                            input.advance();
                                        }
                                        // Consome até '>'
                                        while let Some(next_c) = input.advance() {
                                            if next_c == '>' {
                                                break;
                                            }
                                        }
                                        sink.process_token(Token::EndTag(EndTagToken {
                                            name: expected_tag.clone(),
                                        }));
                                        self.state = TokenizerState::Data;
                                        continue;
                                    }
                                }
                            }
                        }
                        sink.process_token(Token::Character(SmolStr::new("<")));
                    }
                    '\0' => {
                        sink.process_token(Token::Character(SmolStr::new("\u{FFFD}")));
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                    }
                },

                // 28. RCDATA State (WHATWG §12.2.5.2 - ex: <title>, <textarea>)
                TokenizerState::RCDATA => match ch {
                    '&' => {
                        let remaining = peek_str(input, 32);
                        if let Some((decoded, consumed)) = decode_character_reference(&remaining) {
                            for _ in 0..consumed {
                                input.advance();
                            }
                            sink.process_token(Token::Character(decoded));
                        } else {
                            sink.process_token(Token::Character(SmolStr::new("&")));
                        }
                    }
                    '<' => {
                        if let Some(expected_tag) = &self.last_start_tag_name {
                            let expected_str = expected_tag.as_str();
                            let lookahead_len = 1 + expected_str.len() + 1;
                            let peeked = peek_str(input, lookahead_len);

                            if let Some(after_slash) = peeked.strip_prefix('/') {
                                if after_slash.to_ascii_lowercase().starts_with(expected_str) {
                                    let remainder = &after_slash[expected_str.len()..];
                                    if remainder.starts_with('>') || remainder.starts_with(' ') || remainder.starts_with('\t') || remainder.starts_with('\n') || remainder.starts_with('/') {
                                        input.advance(); // consome '/'
                                        for _ in 0..expected_str.len() {
                                            input.advance();
                                        }
                                        while let Some(next_c) = input.advance() {
                                            if next_c == '>' {
                                                break;
                                            }
                                        }
                                        sink.process_token(Token::EndTag(EndTagToken {
                                            name: expected_tag.clone(),
                                        }));
                                        self.state = TokenizerState::Data;
                                        continue;
                                    }
                                }
                            }
                        }
                        sink.process_token(Token::Character(SmolStr::new("<")));
                    }
                    '\0' => {
                        sink.process_token(Token::Character(SmolStr::new("\u{FFFD}")));
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                    }
                },

                // 29. Script Data State (WHATWG §12.2.5.4 - ex: <script>)
                TokenizerState::ScriptData => match ch {
                    '<' => {
                        let peeked = peek_str(input, 8); // '/script'
                        if peeked.to_ascii_lowercase().starts_with("/script") {
                            let remainder = &peeked[7..];
                            if remainder.starts_with('>') || remainder.starts_with(' ') || remainder.starts_with('\t') || remainder.starts_with('\n') || remainder.starts_with('/') {
                                for _ in 0..7 {
                                    input.advance();
                                }
                                while let Some(next_c) = input.advance() {
                                    if next_c == '>' {
                                        break;
                                    }
                                }
                                sink.process_token(Token::EndTag(EndTagToken {
                                    name: Atom::new("script"),
                                }));
                                self.state = TokenizerState::Data;
                                continue;
                            }
                        }
                        sink.process_token(Token::Character(SmolStr::new("<")));
                    }
                    '\0' => {
                        sink.process_token(Token::Character(SmolStr::new("\u{FFFD}")));
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                    }
                },

                // 30. PLAINTEXT State (WHATWG §12.2.5.5 - ex: <plaintext>)
                TokenizerState::PLAINTEXT => match ch {
                    '\0' => {
                        sink.process_token(Token::Character(SmolStr::new("\u{FFFD}")));
                    }
                    c => {
                        sink.process_token(Token::Character(SmolStr::new(c.to_string())));
                    }
                },

                // Fallback de segurança para estados especializados
                _ => {
                    self.state = TokenizerState::Data;
                }
            }
        }

        // Emite EOF
        sink.process_token(Token::Eof);
    }
}
