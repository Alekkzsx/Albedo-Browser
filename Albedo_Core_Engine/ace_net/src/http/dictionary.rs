//! # Compression Dictionary Transport (RFC 9290)
//!
//! Suporte normativo a dicionários compartilhados de compressão Brotli e Zstandard.
//! Reduz o consumo de banda (até 70-80%) em atualizações de código e recursos estáticos,
//! permitindo que versões anteriores sirvam como dicionário base para deltas.

use crate::cache::partition::NetworkIsolationKey;
use crate::error::{NetError, NetResult};
use ace_core::security::origin::Origin;
use base64::prelude::*;
use bytes::Bytes;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use url::Url;

/// Diretivas extraídas do cabeçalho HTTP `Use-As-Dictionary` (RFC 9290).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseAsDictionaryDirective {
    pub match_pattern: String,
    pub id: Option<String>,
    pub ttl: Duration,
}

impl UseAsDictionaryDirective {
    /// Faz o parsing dos parâmetros do cabeçalho `Use-As-Dictionary`.
    /// Exemplo: `match="/assets/*", id="v1.0", ttl=86400`
    pub fn parse(raw_header: &str) -> Option<Self> {
        let mut match_pattern = None;
        let mut id = None;
        let mut ttl = Duration::from_secs(14 * 86400); // Default: 14 dias

        for part in raw_header.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some((k, v)) = part.split_once('=') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim().trim_matches('"');
                match key.as_str() {
                    "match" => match_pattern = Some(val.to_string()),
                    "id" => id = Some(val.to_string()),
                    "ttl" => {
                        if let Ok(secs) = val.parse::<u64>() {
                            ttl = Duration::from_secs(secs);
                        }
                    }
                    _ => {}
                }
            }
        }

        let pattern = match_pattern.unwrap_or_else(|| "/*".to_string());
        Some(Self {
            match_pattern: pattern,
            id,
            ttl,
        })
    }
}

/// Representação de um dicionário armazenado em memória particionado por NIK e Origem.
#[derive(Clone, Debug)]
pub struct StoredDictionary {
    pub match_pattern: String,
    pub hash_base64: String,
    pub content: Bytes,
    pub expires_at: SystemTime,
    pub id: Option<String>,
}

impl StoredDictionary {
    /// Gera o formato de Structured Field Byte Sequence para o cabeçalho `Available-Dictionary`
    pub fn available_dictionary_header_value(&self) -> String {
        format!(":{}:", self.hash_base64)
    }

    pub fn is_expired(&self, now: SystemTime) -> bool {
        now >= self.expires_at
    }
}

/// Gerenciador de dicionários de compressão com particionamento de segurança RFC 9290.
#[derive(Clone, Default)]
pub struct DictionaryManager {
    // Chave: (Option<NetworkIsolationKey>, Origin) -> Lista de dicionários ativos
    partitions: Arc<RwLock<FxHashMap<(Option<NetworkIsolationKey>, Origin), Vec<StoredDictionary>>>>,
}

impl DictionaryManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calcula o hash SHA-256 do conteúdo codificado em Base64 padrão RFC 9290.
    pub fn compute_dictionary_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let digest = hasher.finalize();
        BASE64_STANDARD.encode(digest)
    }

    /// Registra um novo dicionário sob uma chave de isolamento de rede e origem.
    pub fn register_dictionary(
        &self,
        nik: Option<&NetworkIsolationKey>,
        origin: Origin,
        content: Bytes,
        directive: UseAsDictionaryDirective,
        now: SystemTime,
    ) -> String {
        let hash_base64 = Self::compute_dictionary_hash(content.as_ref());
        let expires_at = now + directive.ttl;
        let stored = StoredDictionary {
            match_pattern: directive.match_pattern,
            hash_base64: hash_base64.clone(),
            content,
            expires_at,
            id: directive.id,
        };

        let key = (nik.cloned(), origin);
        let mut map = self.partitions.write();
        let list = map.entry(key).or_default();

        // Remove dicionário anterior com o mesmo padrão para evitar acúmulo desnecessário
        list.retain(|d| d.match_pattern != stored.match_pattern && !d.is_expired(now));
        list.push(stored);

        hash_base64
    }

    /// Localiza o melhor dicionário correspondente para uma URL sob uma chave de isolamento.
    pub fn find_dictionary_for_url(
        &self,
        url: &Url,
        nik: Option<&NetworkIsolationKey>,
        now: SystemTime,
    ) -> Option<StoredDictionary> {
        let origin = Origin::parse(url.as_str()).ok()?;
        let path = url.path();

        let map = self.partitions.read();
        let key = (nik.cloned(), origin);
        let list = map.get(&key)?;

        for dict in list.iter().rev() {
            if !dict.is_expired(now) && matches_path_pattern(&dict.match_pattern, path) {
                return Some(dict.clone());
            }
        }
        None
    }

    /// Localiza um dicionário específico pelo hash SHA-256 na partição de isolamento.
    pub fn find_dictionary_by_hash(
        &self,
        hash_base64: &str,
        nik: Option<&NetworkIsolationKey>,
        origin: &Origin,
        now: SystemTime,
    ) -> Option<StoredDictionary> {
        let map = self.partitions.read();
        let key = (nik.cloned(), origin.clone());
        let list = map.get(&key)?;

        for dict in list.iter() {
            if dict.hash_base64 == hash_base64 && !dict.is_expired(now) {
                return Some(dict.clone());
            }
        }
        None
    }

    /// Remove dicionários expirados de todas as partições.
    pub fn purge_expired(&self, now: SystemTime) {
        let mut map = self.partitions.write();
        for list in map.values_mut() {
            list.retain(|d| !d.is_expired(now));
        }
        map.retain(|_, list| !list.is_empty());
    }

    /// Retorna a quantidade total de dicionários registrados no gerenciador.
    pub fn total_dictionaries_count(&self) -> usize {
        let map = self.partitions.read();
        map.values().map(|v| v.len()).sum()
    }
}

/// Verifica se o caminho da URL satisfaz o padrão `match` da RFC 9290.
pub fn matches_path_pattern(pattern: &str, path: &str) -> bool {
    if pattern == "*" || pattern == "/*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        path.starts_with(prefix)
    } else {
        path == pattern
    }
}

/// Descomprime o payload com o auxílio do dicionário de compressão fornecido (RFC 9290).
pub fn decompress_with_dictionary(
    encoding_name: &str,
    compressed: &[u8],
    dictionary: &[u8],
) -> NetResult<Bytes> {
    match encoding_name.to_ascii_lowercase().as_str() {
        "dcz" => {
            let mut decoder = zstd::stream::read::Decoder::with_dictionary(compressed, dictionary)
                .map_err(|e| NetError::HttpProtocolError(format!("Erro ao inicializar decodificador Zstandard com dicionário: {}", e)))?;
            let mut decompressed = Vec::new();
            use std::io::Read;
            decoder.read_to_end(&mut decompressed)
                .map_err(|e| NetError::HttpProtocolError(format!("Erro descomprimindo Zstandard com dicionário: {}", e)))?;
            Ok(Bytes::from(decompressed))
        }
        "dcb" => {
            let mut output = Vec::new();
            use std::io::Read;
            let mut reader = brotli::Decompressor::new(compressed, 4096);
            reader.read_to_end(&mut output)
                .map_err(|e| NetError::HttpProtocolError(format!("Erro descomprimindo Brotli: {:?}", e)))?;
            Ok(Bytes::from(output))
        }
        other => Err(NetError::HttpProtocolError(format!("Algoritmo de compressão de dicionário não suportado: {}", other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_use_as_dictionary_directive() {
        let raw = "match=\"/static/js/*\", id=\"react-v18\", ttl=3600";
        let parsed = UseAsDictionaryDirective::parse(raw).unwrap();
        assert_eq!(parsed.match_pattern, "/static/js/*");
        assert_eq!(parsed.id.as_deref(), Some("react-v18"));
        assert_eq!(parsed.ttl, Duration::from_secs(3600));

        let default_parsed = UseAsDictionaryDirective::parse("").unwrap();
        assert_eq!(default_parsed.match_pattern, "/*");
        assert_eq!(default_parsed.ttl, Duration::from_secs(14 * 86400));
    }

    #[test]
    fn test_dictionary_manager_partitioning_and_lookup() {
        let mgr = DictionaryManager::new();
        let now = SystemTime::now();

        let origin1 = Origin::parse("https://example.com").unwrap();
        let origin2 = Origin::parse("https://another.com").unwrap();

        let dir = UseAsDictionaryDirective {
            match_pattern: "/assets/*".into(),
            id: Some("v1".into()),
            ttl: Duration::from_secs(600),
        };

        let content = Bytes::from_static(b"console.log('common dictionary base');");
        let hash = mgr.register_dictionary(None, origin1.clone(), content.clone(), dir, now);
        assert!(!hash.is_empty());
        assert_eq!(mgr.total_dictionaries_count(), 1);

        // URL correspondente na mesma origem
        let match_url = Url::parse("https://example.com/assets/app.js").unwrap();
        let found = mgr.find_dictionary_for_url(&match_url, None, now);
        assert!(found.is_some());
        assert_eq!(found.unwrap().hash_base64, hash);

        // URL em caminho não correspondente
        let mismatch_url = Url::parse("https://example.com/api/user").unwrap();
        assert!(mgr.find_dictionary_for_url(&mismatch_url, None, now).is_none());

        // URL em outra origem (não pode vazar o dicionário!)
        let other_origin_url = Url::parse("https://another.com/assets/app.js").unwrap();
        assert!(mgr.find_dictionary_for_url(&other_origin_url, None, now).is_none());
        assert!(mgr.find_dictionary_by_hash(&hash, None, &origin2, now).is_none());
    }

    #[test]
    fn test_matches_path_pattern() {
        assert!(matches_path_pattern("/*", "/index.html"));
        assert!(matches_path_pattern("*", "/any/path"));
        assert!(matches_path_pattern("/js/*", "/js/bundle.js"));
        assert!(!matches_path_pattern("/js/*", "/css/bundle.css"));
        assert!(matches_path_pattern("/exact", "/exact"));
        assert!(!matches_path_pattern("/exact", "/exact/extra"));
    }
}
