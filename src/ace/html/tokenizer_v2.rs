use html5gum::{Tokenizer, Token, TokenizerState};
use std::collections::HashMap;
use crate::ace::html::{Namespace};
use crate::ace::util::allocator::AceAllocator;

/// O `AceTokenizer` é um wrapper de alta performance sobre o `html5gum`.
/// Ele transforma bytes/strings em tokens WHATWG usando otimizações SIMD
/// e minimizando alocações desnecessárias.
pub struct AceTokenizer<'a> {
    inner: Tokenizer<'a>,
    /// Referência ao alocador para strings temporárias se necessário.
    allocator: &'a AceAllocator,
}

#[derive(Debug, Clone)]
pub enum AceTokenKind<'a> {
    Doctype {
        name: Option<&'a str>,
        public_id: Option<&'a str>,
        system_id: Option<&'a str>,
        force_quirks: bool,
    },
    StartTag {
        name: &'a str,
        attributes: Vec<(&'a str, &'a str)>, // Usando Vec para performance em vez de HashMap
        self_closing: bool,
    },
    EndTag {
        name: &'a str,
    },
    Comment {
        data: &'a str,
    },
    Text {
        data: &'a str,
    },
    Eof,
}

impl<'a> AceTokenizer<'a> {
    pub fn new(input: &'a str, allocator: &'a AceAllocator) -> Self {
        Self {
            inner: Tokenizer::new(input),
            allocator,
        }
    }

    /// Obtém o próximo token da stream.
    /// Como o html5gum usa lifetimes curtos, convertemos para nossa representação
    /// que pode viver na Arena se necessário, ou apenas referenciar o buffer original.
    pub fn next_token(&mut self) -> Option<AceTokenKind<'a>> {
        // html5gum retorna Option<Token>
        match self.inner.next() {
            Some(Token::StartTag(tag)) => {
                let name = self.allocator.alloc_str(std::str::from_utf8(&tag.name).unwrap_or(""));
                let mut attributes = Vec::with_capacity(tag.attributes.len());
                
                for (name_bytes, value_bytes) in tag.attributes {
                    let attr_name = self.allocator.alloc_str(std::str::from_utf8(&name_bytes).unwrap_or(""));
                    let attr_value = self.allocator.alloc_str(std::str::from_utf8(&value_bytes).unwrap_or(""));
                    attributes.push((attr_name, attr_value));
                }

                Some(AceTokenKind::StartTag {
                    name,
                    attributes,
                    self_closing: tag.self_closing,
                })
            }
            Some(Token::EndTag(tag)) => {
                let name = self.allocator.alloc_str(std::str::from_utf8(&tag.name).unwrap_or(""));
                Some(AceTokenKind::EndTag { name })
            }
            Some(Token::Comment(data)) => {
                let comment_text = self.allocator.alloc_str(std::str::from_utf8(&data).unwrap_or(""));
                Some(AceTokenKind::Comment { data: comment_text })
            }
            Some(Token::String(data)) => {
                let text = self.allocator.alloc_str(std::str::from_utf8(&data).unwrap_or(""));
                Some(AceTokenKind::Text { data: text })
            }
            Some(Token::Doctype(dt)) => {
                let name = dt.name.as_ref().map(|b| self.allocator.alloc_str(std::str::from_utf8(b).unwrap_or("")));
                let public_id = dt.public_id.as_ref().map(|b| self.allocator.alloc_str(std::str::from_utf8(b).unwrap_or("")));
                let system_id = dt.system_id.as_ref().map(|b| self.allocator.alloc_str(std::str::from_utf8(b).unwrap_or("")));
                
                Some(AceTokenKind::Doctype {
                    name,
                    public_id,
                    system_id,
                    force_quirks: dt.force_quirks,
                })
            }
            None => Some(AceTokenKind::Eof),
            _ => None, // Outros tipos que podem surgir
        }
    }

    /// Ajusta o estado do tokenizer (ex: para entrar em RCDATA/RawText para tags <script>/<style>)
    pub fn set_state(&mut self, state: TokenizerState) {
        self.inner.set_state(state);
    }
}
