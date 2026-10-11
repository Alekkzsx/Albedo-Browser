//! # Adaptador de Streaming para Tokenização Incremental (`StreamingHtmlParser`)
//!
//! Permite que respostas da rede em streaming (`BodyStream`) alimentem o
//! parser diretamente em pedaços contínuos sem alocações desnecessárias.

use crate::tree::Document;

/// Parser incremental de HTML que consome fatias de bytes recebidas da rede.
#[derive(Debug, Default)]
pub struct StreamingHtmlParser {
    buffer: Vec<u8>,
}

impl StreamingHtmlParser {
    /// Cria uma nova instância vazia do parser streaming.
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Cria uma nova instância pré-alocando capacidade esperada para reduzir realocações.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Alimenta o parser com um novo pedaço binário de bytes recebido do transporte de rede.
    pub fn feed_chunk(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }

    /// Retorna a quantidade de bytes acumulados até o momento.
    pub fn bytes_received(&self) -> usize {
        self.buffer.len()
    }

    /// Finaliza o streaming e executa a construção da árvore gerando o `Document` final.
    pub fn finish(self) -> Document {
        let text = String::from_utf8_lossy(&self.buffer);
        crate::parse_html(&text)
    }
}
