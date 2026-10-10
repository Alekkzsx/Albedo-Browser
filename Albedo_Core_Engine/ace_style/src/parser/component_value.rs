//! # Component Values e Blocos de Preservação CSS (CSS Syntax L3 §5.4)
//!
//! Representação de valores de componentes, blocos simples `{...}`, `[...]`, `(...)` e funções.
//! Possui limite estrito de profundidade de aninhamento para defesa contra ataques de estouro de pilha.

use super::token::Token;
use smol_str::SmolStr;

/// Limite de recursão de profundidade de aninhamento contra ataques adversariais de Stack Overflow.
pub const MAX_NESTING_DEPTH: usize = 128;

/// Representação de um valor de componente conforme CSS Syntax Level 3.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentValue {
    /// Token preservado individual
    PreservedToken(Token),
    /// Chamada de função com lista de argumentos
    Function {
        name: SmolStr,
        value: Vec<ComponentValue>,
    },
    /// Bloco delimitado simples `{...}`, `[...]` ou `(...)`
    SimpleBlock {
        open: char,
        close: char,
        value: Vec<ComponentValue>,
    },
}

impl ComponentValue {
    /// Serializa o valor do componente de volta para texto CSS legível.
    pub fn to_css_string(&self) -> String {
        match self {
            Self::PreservedToken(t) => match t {
                Token::Ident(s) => s.to_string(),
                Token::Function(s) => format!("{}(", s),
                Token::AtKeyword(s) => format!("@{}", s),
                Token::Hash { value, is_id: _ } => format!("#{}", value),
                Token::String(s) => format!("\"{}\"", s),
                Token::Number { value, .. } => value.to_string(),
                Token::Percentage(v) => format!("{}%", v),
                Token::Dimension { value, unit } => format!("{}{}", value, unit),
                Token::Whitespace => " ".to_string(),
                Token::Colon => ":".to_string(),
                Token::Semicolon => ";".to_string(),
                Token::Comma => ",".to_string(),
                Token::Delim(c) => c.to_string(),
                Token::Url(u) => format!("url(\"{}\")", u),
                _ => String::new(),
            },
            Self::Function { name, value } => {
                let inner: String = value.iter().map(|v| v.to_css_string()).collect();
                format!("{}({})", name, inner)
            }
            Self::SimpleBlock { open, close, value } => {
                let inner: String = value.iter().map(|v| v.to_css_string()).collect();
                format!("{}{}{}", open, inner, close)
            }
        }
    }
}

/// Parser de valores de componentes a partir de uma sequência de tokens.
pub struct ComponentValueParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
}

impl<'a> ComponentValueParser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            depth: 0,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    /// Faz o parse de toda a lista de tokens em valores de componentes.
    pub fn parse_all(&mut self) -> Vec<ComponentValue> {
        let mut values = Vec::new();
        while let Some(tok) = self.peek() {
            if tok.is_eof() {
                break;
            }
            values.push(self.consume_component_value());
        }
        values
    }

    /// Consome um valor de componente, tratando abertura e fechamento de blocos.
    pub fn consume_component_value(&mut self) -> ComponentValue {
        let tok = match self.next() {
            Some(t) => t.clone(),
            None => return ComponentValue::PreservedToken(Token::Eof),
        };

        match tok {
            Token::OpenCurly => self.consume_simple_block('{', '}'),
            Token::OpenSquare => self.consume_simple_block('[', ']'),
            Token::OpenParen => self.consume_simple_block('(', ')'),
            Token::Function(name) => self.consume_function(name),
            other => ComponentValue::PreservedToken(other),
        }
    }

    fn consume_simple_block(&mut self, open: char, close: char) -> ComponentValue {
        if self.depth >= MAX_NESTING_DEPTH {
            // Defesa contra ataque de estouro de pilha por profundidade extrema
            return ComponentValue::SimpleBlock {
                open,
                close,
                value: Vec::new(),
            };
        }

        self.depth += 1;
        let mut values = Vec::new();

        while let Some(tok) = self.peek() {
            if tok.is_eof() {
                break;
            }
            match (close, tok) {
                ('}', Token::CloseCurly) | (']', Token::CloseSquare) | (')', Token::CloseParen) => {
                    self.next();
                    break;
                }
                _ => {
                    values.push(self.consume_component_value());
                }
            }
        }

        self.depth -= 1;
        ComponentValue::SimpleBlock {
            open,
            close,
            value: values,
        }
    }

    fn consume_function(&mut self, name: SmolStr) -> ComponentValue {
        if self.depth >= MAX_NESTING_DEPTH {
            return ComponentValue::Function {
                name,
                value: Vec::new(),
            };
        }

        self.depth += 1;
        let mut values = Vec::new();

        while let Some(tok) = self.peek() {
            if tok.is_eof() {
                break;
            }
            if matches!(tok, Token::CloseParen) {
                self.next();
                break;
            }
            values.push(self.consume_component_value());
        }

        self.depth -= 1;
        ComponentValue::Function { name, value: values }
    }
}
