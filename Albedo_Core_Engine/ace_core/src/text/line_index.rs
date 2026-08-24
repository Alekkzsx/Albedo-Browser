//! # Mapeamento O(log N) de Posições no Código-Fonte (LineIndex Pattern)
//!
//! Estrutura de indexação de quebras de linha em passagem única, permitindo conversão rápida
//! entre `byte_offset` e coordenadas `(linha, coluna)` para compiladores, lexers e DevTools.

/// Índice de linhas de um documento de texto para consultas de posição em $O(\log N)$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    /// Offsets em bytes do início de cada linha (a linha 1 sempre começa no offset 0).
    line_starts: Vec<usize>,
    /// Tamanho total do documento em bytes.
    total_len: usize,
}

impl LineIndex {
    /// Constrói um novo `LineIndex` a partir do texto de entrada em uma única passagem $O(N)$.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, byte) in text.as_bytes().iter().enumerate() {
            if *byte == b'\n' {
                line_starts.push(i + 1);
            }
        }

        Self {
            line_starts,
            total_len: text.len(),
        }
    }

    /// Retorna a quantidade total de linhas no documento (1-indexed).
    #[inline]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Retorna o tamanho total do documento em bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.total_len
    }

    /// Retorna `true` se o documento for vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    /// Converte um `byte_offset` nas coordenadas `(linha, coluna)` (ambas 1-indexadas) em $O(\log N)$.
    pub fn line_col(&self, byte_offset: usize) -> (usize, usize) {
        let offset = byte_offset.min(self.total_len);

        // Busca binária para encontrar a linha
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        let line_start = self.line_starts[line_idx];
        let col = offset - line_start + 1;
        (line_idx + 1, col)
    }

    /// Converte coordenadas `(linha, coluna)` (1-indexadas) de volta para o `byte_offset`.
    pub fn offset(&self, line: usize, column: usize) -> Option<usize> {
        if line == 0 || line > self.line_starts.len() || column == 0 {
            return None;
        }

        let line_idx = line - 1;
        let line_start = self.line_starts[line_idx];
        let target_offset = line_start + (column - 1);

        let next_line_start = if line_idx + 1 < self.line_starts.len() {
            self.line_starts[line_idx + 1]
        } else {
            self.total_len + 1
        };

        if target_offset < next_line_start {
            Some(target_offset.min(self.total_len))
        } else {
            None
        }
    }

    /// Retorna o intervalo em bytes `(start, end)` de uma linha específica (1-indexada).
    pub fn line_range(&self, line: usize) -> Option<(usize, usize)> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }

        let line_idx = line - 1;
        let start = self.line_starts[line_idx];
        let end = if line_idx + 1 < self.line_starts.len() {
            self.line_starts[line_idx + 1]
        } else {
            self.total_len
        };

        Some((start, end))
    }
}
