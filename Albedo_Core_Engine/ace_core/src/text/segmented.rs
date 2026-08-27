//! # Buffer de Texto Segmentado em Chunks (WHATWG HTML5 Streaming Input)
//!
//! Abstração de leitura contínua sobre streams fragmentados de texto,
//! permitindo consumo zero-copy, devolução de caracteres (*unconsume / push-front*),
//! pré-processamento WHATWG (normalização CRLF/Null) e rastreamento $O(1)$ de `SourceLocation`.

use crate::error::SourceLocation;
use smol_str::SmolStr;
use std::collections::VecDeque;

/// Pré-processa um fragmento de texto conforme o padrão WHATWG HTML:
/// - Normaliza `\r\n` (CRLF) e `\r` (CR) isolados para `\n` (LF).
/// - Substitui `\0` (NULL) pelo caractere substituto Unicode `\u{FFFD}`.
pub fn preprocess_html_input(input: &str) -> SmolStr {
    if !input.contains('\r') && !input.contains('\0') {
        return SmolStr::new(input);
    }

    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut last = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                result.push_str(&input[last..i]);
                result.push('\n');
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    i += 1;
                }
                last = i + 1;
            }
            0 => {
                result.push_str(&input[last..i]);
                result.push('\u{FFFD}');
                last = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if last < input.len() {
        result.push_str(&input[last..]);
    }

    SmolStr::new(result)
}

/// Buffer de texto segmentado com suporte a chunks dinâmicos, inserção no fluxo (document.write) e devolução de caracteres (*unconsume*).
#[derive(Debug, Clone, Default)]
pub struct SegmentedString {
    /// Caracteres devolvidos (*unconsumed*) prontos para re-leitura prioritária.
    pushed_back: Vec<char>,
    /// Chunks de texto enfileirados provenientes da rede ou de blocos em memória.
    chunks: VecDeque<SmolStr>,
    /// Posição do cursor em bytes dentro do chunk frontal.
    current_chunk_offset: usize,
    /// Pilha de índices de pontos de inserção dinâmicos (WHATWG §12.2.3).
    insertion_points: Vec<usize>,
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
            insertion_points: Vec::new(),
            line: 1,
            column: 1,
            byte_offset: 0,
            url: None,
        }
    }

    /// Cria uma `SegmentedString` a partir de uma fatia de texto estática sem pré-processamento.
    pub fn from_static_str(text: &str) -> Self {
        let mut s = Self::new();
        if !text.is_empty() {
            s.chunks.push_back(SmolStr::new(text));
        }
        s
    }

    /// Cria uma `SegmentedString` com pré-processamento de fluxo de entrada WHATWG HTML5.
    pub fn from_preprocessed_str(text: &str) -> Self {
        let mut s = Self::new();
        if !text.is_empty() {
            s.chunks.push_back(preprocess_html_input(text));
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

    /// Adiciona um novo chunk aplicando normalização WHATWG CRLF/Null.
    pub fn append_preprocessed_chunk(&mut self, chunk: &str) {
        if !chunk.is_empty() {
            self.chunks.push_back(preprocess_html_input(chunk));
        }
    }

    /// Devolve um caractere para a frente do buffer (*unconsume / push-front*).
    ///
    /// Essencial para o Tokenizer WHATWG ao avaliar entidades HTML ou lookaheads que falham.
    pub fn push_front_char(&mut self, c: char) {
        self.pushed_back.push(c);
        self.byte_offset = self.byte_offset.saturating_sub(c.len_utf8());
        if c == '\n' {
            self.line = self.line.saturating_sub(1).max(1);
        } else if self.column > 1 {
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

        for (i, chunk) in self.chunks.iter().enumerate() {
            let start = if i == 0 { self.current_chunk_offset } else { 0 };
            if start < chunk.len() {
                let slice = &chunk[start..];
                if let Some(c) = slice.chars().next() {
                    return Some(c);
                }
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
            if start < chunk.len() {
                let slice = &chunk[start..];
                for c in slice.chars() {
                    if skipped == offset {
                        return Some(c);
                    }
                    skipped += 1;
                }
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
        if self.pushed_back.is_empty() {
            if let Some(chunk) = self.chunks.front() {
                let slice = &chunk[self.current_chunk_offset..];
                if slice.len() >= prefix.len() {
                    return slice.starts_with(prefix);
                }
            }
        }

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

    /// Retorna uma referência direta à fatia contígua na frente do buffer, se não houver caracteres devolvidos.
    pub fn current_contiguous_slice(&mut self) -> Option<&str> {
        if !self.pushed_back.is_empty() {
            return None;
        }
        self.clean_empty_chunks();
        let chunk = self.chunks.front()?;
        if self.current_chunk_offset < chunk.len() {
            Some(&chunk[self.current_chunk_offset..])
        } else {
            None
        }
    }

    /// Avança o cursor em `n` bytes contíguos atualizando métricas de linha/coluna (Fast-Path SIMD).
    pub fn advance_bytes(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if let Some(chunk) = self.chunks.front() {
            let end = (self.current_chunk_offset + n).min(chunk.len());
            let slice = &chunk[self.current_chunk_offset..end];
            for c in slice.chars() {
                if c == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
            }
            let consumed = end - self.current_chunk_offset;
            self.current_chunk_offset += consumed;
            self.byte_offset += consumed;
            self.clean_empty_chunks();
        }
    }

    /// Retorna `true` se o stream estiver completamente vazio e sem chunks pendentes.
    pub fn is_eof(&self) -> bool {
        if !self.pushed_back.is_empty() {
            return false;
        }
        for (i, chunk) in self.chunks.iter().enumerate() {
            let start = if i == 0 { self.current_chunk_offset } else { 0 };
            if start < chunk.len() {
                return false;
            }
        }
        true
    }

    /// Retorna a localização de código-fonte atual (`SourceLocation`).
    pub fn location(&self) -> SourceLocation {
        if let Some(ref url) = self.url {
            SourceLocation::with_url(url, self.line, self.column, self.byte_offset)
        } else {
            SourceLocation::new(self.line, self.column, self.byte_offset)
        }
    }

    /// Estabelece um novo ponto de inserção dinâmico (WHATWG HTML §12.2.3) antes do próximo caractere não consumido.
    pub fn push_insertion_point(&mut self) {
        self.normalize_front();
        self.insertion_points.push(0);
    }

    /// Remove o ponto de inserção dinâmico corrente no topo da pilha.
    pub fn pop_insertion_point(&mut self) -> Option<usize> {
        self.insertion_points.pop()
    }

    /// Retorna `true` se houver pelo menos um ponto de inserção ativo na pilha.
    pub fn has_insertion_point(&self) -> bool {
        !self.insertion_points.is_empty()
    }

    /// Retorna a profundidade de aninhamento de pontos de inserção ativos.
    pub fn insertion_point_depth(&self) -> usize {
        self.insertion_points.len()
    }

    /// Limpa todos os pontos de inserção ativos.
    pub fn clear_insertion_points(&mut self) {
        self.insertion_points.clear();
    }

    /// Insere texto pré-processado no ponto de inserção ativo ou no cabeçalho do fluxo (WHATWG §12.2.3).
    ///
    /// Preserva a ordem cronológica FIFO para escritas consecutivas no mesmo ponto de inserção,
    /// e a semântica LIFO para chamadas reentrantes/aninhadas com novos pontos de inserção.
    pub fn insert_stream_content(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        let preprocessed = preprocess_html_input(content);
        if preprocessed.is_empty() {
            return;
        }

        self.normalize_front();

        if let Some(&target_idx) = self.insertion_points.last() {
            let insert_idx = target_idx.min(self.chunks.len());
            self.chunks.insert(insert_idx, preprocessed);

            // Avança este ponto de inserção para depois do chunk recém-inserido
            if let Some(top) = self.insertion_points.last_mut() {
                *top += 1;
            }

            // Ajusta outros pontos de inserção na pilha cujo índice seja posterior
            let stack_len = self.insertion_points.len();
            if stack_len > 1 {
                for ip in &mut self.insertion_points[..stack_len - 1] {
                    if *ip >= insert_idx {
                        *ip += 1;
                    }
                }
            }
        } else {
            // Sem ponto de inserção explícito: insere diretamente no cabeçalho
            self.chunks.push_front(preprocessed);
        }
    }

    /// Insere texto no ponto de inserção ativo ou no cabeçalho do fluxo (alias para `insert_stream_content`).
    pub fn insert_at_current(&mut self, content: &str) {
        self.insert_stream_content(content);
    }

    /// Retorna a quantidade de caracteres restantes a serem consumidos no buffer.
    pub fn remaining_chars(&self) -> usize {
        let mut count = self.pushed_back.len();
        for (i, chunk) in self.chunks.iter().enumerate() {
            let start = if i == 0 { self.current_chunk_offset } else { 0 };
            if start < chunk.len() {
                count += chunk[start..].chars().count();
            }
        }
        count
    }

    /// Retorna uma representação em `String` de todos os caracteres ainda não consumidos no buffer.
    pub fn unconsumed_str(&self) -> String {
        let mut s = String::new();
        for &c in self.pushed_back.iter().rev() {
            s.push(c);
        }
        for (i, chunk) in self.chunks.iter().enumerate() {
            let start = if i == 0 { self.current_chunk_offset } else { 0 };
            if start < chunk.len() {
                s.push_str(&chunk[start..]);
            }
        }
        s
    }

    /// Normaliza `current_chunk_offset` e `pushed_back` para que o cabeçalho fique alinhado no chunk 0.
    fn normalize_front(&mut self) {
        self.clean_empty_chunks();
        if self.current_chunk_offset > 0 {
            if let Some(front) = self.chunks.pop_front() {
                if self.current_chunk_offset < front.len() {
                    let remainder = &front[self.current_chunk_offset..];
                    self.chunks.push_front(SmolStr::new(remainder));
                }
            }
            self.current_chunk_offset = 0;
        }
        if !self.pushed_back.is_empty() {
            let mut s = String::with_capacity(self.pushed_back.len());
            while let Some(c) = self.pushed_back.pop() {
                s.push(c);
            }
            self.chunks.push_front(SmolStr::new(s));
            for ip in &mut self.insertion_points {
                *ip += 1;
            }
        }
    }

    /// Remove chunks frontais que já foram completamente consumidos, ajustando pontos de inserção.
    fn clean_empty_chunks(&mut self) {
        while let Some(chunk) = self.chunks.front() {
            if self.current_chunk_offset >= chunk.len() {
                self.chunks.pop_front();
                self.current_chunk_offset = 0;
                for ip in &mut self.insertion_points {
                    *ip = ip.saturating_sub(1);
                }
            } else {
                break;
            }
        }
    }
}

impl From<&str> for SegmentedString {
    fn from(s: &str) -> Self {
        Self::from_static_str(s)
    }
}

impl std::str::FromStr for SegmentedString {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_static_str(s))
    }
}
