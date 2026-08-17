//! # Buffer de Texto Segmentado em Chunks (WHATWG HTML5 Streaming Input)
//!
//! Abstração de leitura contínua sobre streams fragmentados de texto,
//! permitindo consumo zero-copy, devolução de caracteres (*unconsume / push-front*) e rastreamento $O(1)$ de `SourceLocation`.

use crate::error::SourceLocation;
use smol_str::SmolStr;
use std::collections::VecDeque;

/// Buffer de texto segmentado com suporte a chunks dinâmicos e devolução de caracteres (*unconsume*).
#[derive(Debug, Clone, Default)]
pub struct SegmentedString {
    /// Caracteres devolvidos (*unconsumed*) prontos para re-leitura prioritária.
    pushed_back: Vec<char>,
    /// Chunks de texto enfileirados provenientes da rede ou de blocos em memória.
    chunks: VecDeque<SmolStr>,
    /// Posição do cursor em bytes dentro do chunk frontal.
    current_chunk_offset: usize,
    /// Linha atual (1-indexada).
    line: usize,
    /// Coluna atual (1-indexada).
    column: usize,
    /// Offset total acumulado em bytes.
    byte_offset: usize,
    /// URL do documento de origem associada, se houver.
    url: Option<String>,
}

impl SegmentedString {
    /// Cria uma nova `SegmentedString` vazia.
    pub fn new() -> Self {
        Self {
            pushed_back: Vec::new(),
            chunks: VecDeque::new(),
            current_chunk_offset: 0,
            line: 1,
            column: 1,
            byte_offset: 0,
            url: None,
        }
    }

    /// Cria uma `SegmentedString` a partir de uma fatia de texto estática.
    pub fn from_static_str(text: &str) -> Self {
        let mut s = Self::new();
        if !text.is_empty() {
            s.chunks.push_back(SmolStr::new(text));
        }
        s
    }


    /// Associa uma URL de documento para rastreabilidade de erros.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Adiciona um novo chunk de texto ao final do stream de entrada.
    pub fn append_chunk(&mut self, chunk: impl Into<SmolStr>) {
        let s = chunk.into();
        if !s.is_empty() {
            self.chunks.push_back(s);
        }
    }

    /// Devolve um caractere para a frente do buffer (*unconsume / push-front*).
    ///
    /// Essencial para o Tokenizer WHATWG ao avaliar entidades HTML ou lookaheads que falham.
    pub fn push_front_char(&mut self, c: char) {
        self.pushed_back.push(c);
        self.byte_offset = self.byte_offset.saturating_sub(c.len_utf8());
        if self.column > 1 {
            self.column -= 1;
        }
    }

    /// Devolve uma sequência de texto para a frente do buffer.
    pub fn push_front_str(&mut self, s: &str) {
        for c in s.chars().rev() {
            self.push_front_char(c);
        }
    }

    /// Retorna o próximo caractere sem avançar a posição do cursor (*peek*).
    pub fn peek(&self) -> Option<char> {
        if let Some(&c) = self.pushed_back.last() {
            return Some(c);
        }

        for chunk in &self.chunks {
            let slice = &chunk[self.current_chunk_offset..];
            if let Some(c) = slice.chars().next() {
                return Some(c);
            }
        }

        None
    }

    /// Retorna o caractere na posição relativa `offset` adiante (*lookahead*).
    pub fn peek_at(&self, mut offset: usize) -> Option<char> {
        if offset < self.pushed_back.len() {
            let idx = self.pushed_back.len() - 1 - offset;
            return Some(self.pushed_back[idx]);
        }
        offset -= self.pushed_back.len();

        let mut skipped = 0;
        for (i, chunk) in self.chunks.iter().enumerate() {
            let start = if i == 0 { self.current_chunk_offset } else { 0 };
            let slice = &chunk[start..];

            for c in slice.chars() {
                if skipped == offset {
                    return Some(c);
                }
                skipped += 1;
            }
        }

        None
    }

    /// Avança e consome o próximo caractere do buffer, atualizando linha e coluna.
    pub fn advance(&mut self) -> Option<char> {
        let c = if let Some(c) = self.pushed_back.pop() {
            c
        } else {
            self.clean_empty_chunks();
            let chunk = self.chunks.front_mut()?;
            let slice = &chunk[self.current_chunk_offset..];
            let c = slice.chars().next()?;
            self.current_chunk_offset += c.len_utf8();
            c
        };

        self.byte_offset += c.len_utf8();
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
    pub fn consume_while(&mut self, predicate: impl Fn(char) -> bool) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.advance();
                out.push(c);
            } else {
                break;
            }
        }
        out
    }

    /// Verifica se o stream atual começa com o prefixo especificado.
    pub fn starts_with(&self, prefix: &str) -> bool {
        for (i, target_c) in prefix.chars().enumerate() {
            if self.peek_at(i) != Some(target_c) {
                return false;
            }
        }
        true
    }

    /// Consome o prefixo caso o stream comece com ele. Retorna `true` se consumido.
    pub fn consume_prefix(&mut self, prefix: &str) -> bool {
        if self.starts_with(prefix) {
            self.advance_n(prefix.chars().count());
            true
        } else {
            false
        }
    }

    /// Retorna `true` se o stream estiver completamente vazio e sem chunks pendentes.
    pub fn is_eof(&self) -> bool {
        self.pushed_back.is_empty() && (self.chunks.is_empty() || (self.chunks.len() == 1 && self.current_chunk_offset >= self.chunks[0].len()))
    }

    /// Retorna a localização de código-fonte atual (`SourceLocation`).
    pub fn location(&self) -> SourceLocation {
        if let Some(ref url) = self.url {
            SourceLocation::with_url(url, self.line, self.column, self.byte_offset)
        } else {
            SourceLocation::new(self.line, self.column, self.byte_offset)
        }
    }

    /// Remove chunks frontais que já foram completamente consumidos.
    fn clean_empty_chunks(&mut self) {
        while let Some(chunk) = self.chunks.front() {
            if self.current_chunk_offset >= chunk.len() {
                self.chunks.pop_front();
                self.current_chunk_offset = 0;
            } else {
                break;
            }
        }
    }
}
