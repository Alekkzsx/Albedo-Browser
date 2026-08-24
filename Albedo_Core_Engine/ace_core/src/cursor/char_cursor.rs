//! # Cursor de Caracteres Unicode com Rastreamento de Posição
//!
//! Cursor de texto zero-allocation para lexers e tokenizers (HTML/CSS), mantendo linha, coluna e byte offset.

use crate::error::SourceLocation;

/// Cursor para leitura sequencial de strings Unicode com rastreamento de `SourceLocation`.
#[derive(Debug, Clone)]
pub struct CharCursor<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
    column: usize,
}

impl<'a> CharCursor<'a> {
    /// Cria um novo cursor no início da string de entrada.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    /// Retorna o caractere atual sem avançar a posição (*peek*).
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Retorna o próximo caractere a uma distância `offset` de caracteres à frente.
    pub fn peek_at(&self, offset: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(offset)
    }

    /// Avança e consome o próximo caractere Unicode, atualizando linha e coluna.
    pub fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Avança `n` caracteres sequencialmente.
    pub fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            if self.advance().is_none() {
                break;
            }
        }
    }

    /// Consome caracteres contíguos enquanto o predicado retornar `true`.
    /// Retorna a fatia de texto consumida sem alocações.
    pub fn consume_while(&mut self, predicate: impl Fn(char) -> bool) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.advance();
            } else {
                break;
            }
        }
        &self.input[start..self.pos]
    }

    /// Verifica se a posição atual começa com o prefixo especificado.
    #[inline]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.input[self.pos..].starts_with(prefix)
    }

    /// Consome o prefixo caso a posição atual comece com ele. Retorna `true` se consumido.
    pub fn consume_prefix(&mut self, prefix: &str) -> bool {
        if self.starts_with(prefix) {
            for c in prefix.chars() {
                self.pos += c.len_utf8();
                if c == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
            }
            true
        } else {
            false
        }
    }

    /// Retorna `true` se o cursor atingiu o final da entrada.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Retorna a fatia restante da string de entrada.
    #[inline]
    pub fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    /// Retorna o offset atual em bytes.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.pos
    }

    /// Retorna a linha atual (1-indexada).
    #[inline]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Retorna a coluna atual (1-indexada).
    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }

    /// Gera o `SourceLocation` atual sem URL associada.
    #[inline]
    pub fn location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column, self.pos)
    }

    /// Gera o `SourceLocation` atual associado a uma URL de documento.
    #[inline]
    pub fn location_with_url(&self, url: impl Into<String>) -> SourceLocation {
        SourceLocation::with_url(url, self.line, self.column, self.pos)
    }
}
