//! # Compression Dictionary Transport (RFC 9290)
//!
//! Suporte a dicionários compartilhados de compressão Brotli/Zstd.
//! Reduz drasticamente o payload (até 70%) em atualizações de bibliotecas JavaScript ou SPAs,
//! usando a versão anterior como dicionário de compressão.

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metadados de um dicionário registrado via cabeçalho `Use-As-Dictionary`.
#[derive(Clone, Debug)]
pub struct DictionaryMetadata {
    pub match_pattern: String,
    pub dictionary_hash: String,
    pub content: Bytes,
}

/// Gerenciador de Dicionários em memória (delegará para DiskCache no futuro).
#[derive(Clone, Default)]
pub struct DictionaryManager {
    // Key: hash do dicionário
    dictionaries: Arc<RwLock<HashMap<String, DictionaryMetadata>>>,
}

impl DictionaryManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um novo dicionário baixado.
    pub async fn register_dictionary(&self, meta: DictionaryMetadata) {
        let mut map = self.dictionaries.write().await;
        map.insert(meta.dictionary_hash.clone(), meta);
    }

    /// Procura o dicionário mais adequado baseado na URL e no padrão (match_pattern).
    pub async fn find_dictionary_for_url(&self, _url: &str) -> Option<DictionaryMetadata> {
        // TODO: Mapeamento de URL via wildcard/pattern match (RFC 9290)
        None
    }
}
