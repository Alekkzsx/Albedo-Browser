//! # Chunks de Tokens do Background Parser (Blink ParsedChunk pattern)
//!
//! Agrupamento em lote de `CompactHTMLToken`s com recursos de preload especulativo
//! para despacho lock-free entre a thread de tokenização e a thread do TreeBuilder.

use crate::preload_scanner::PreloadRequest;
use crate::tokenizer::token::CompactHTMLToken;
use crate::tokenizer::state::TokenizerState;

/// Tamanho padrão normativo de um lote de tokens para trânsito entre threads.
pub const DEFAULT_PARSER_CHUNK_SIZE: usize = 256;

/// Representa um lote consolidado de tokens pré-processados em background.
#[derive(Debug, Clone)]
pub struct ParsedChunk {
    /// Vetor contíguo de tokens compactos pertencentes a este lote.
    pub tokens: Vec<CompactHTMLToken>,
    /// Sub-recursos críticos (CSS, JS, Fontes, Imagens) descobertos especulativamente durante a tokenização.
    pub preloads: Vec<PreloadRequest>,
    /// Estado do tokenizer ao iniciar este chunk.
    pub starting_state: TokenizerState,
    /// Estado do tokenizer ao concluir este chunk.
    pub ending_state: TokenizerState,
    /// Indica se este chunk contém o final do stream (EOF).
    pub is_last: bool,
}

impl ParsedChunk {
    /// Cria um novo `ParsedChunk` vazio.
    pub fn new(starting_state: TokenizerState) -> Self {
        Self {
            tokens: Vec::with_capacity(DEFAULT_PARSER_CHUNK_SIZE),
            preloads: Vec::new(),
            starting_state,
            ending_state: starting_state,
            is_last: false,
        }
    }

    /// Retorna `true` se o lote não contiver tokens nem preloads.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty() && self.preloads.is_empty()
    }

    /// Retorna a quantidade de tokens no lote.
    #[inline]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
}
