//! # Armazenamento de Cache em Memória com Despejo LRU (`HttpCache`)
//!
//! Gerencia o ciclo de vida, cotas em bytes e algoritmo de Least Recently Used (LRU)
//! para armazenamento de respostas em conformidade com a RFC 9111.

use super::entry::CacheEntry;
use super::partition::NetworkIsolationKey;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use url::Url;
use std::path::PathBuf;
/// Estatísticas de operação do cache HTTP para telemetria.
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub current_bytes: AtomicU64,
}

/// Mecanismo de cache HTTP particionado em memória com política LRU.
#[derive(Debug)]
pub struct HttpCache {
    inner: RwLock<HttpCacheInner>,
    pub stats: CacheStats,
    pub max_capacity_bytes: usize,
    pub disk_path: Option<PathBuf>,
}

#[derive(Debug)]
struct HttpCacheInner {
    // RAM L1 Cache
    entries: HashMap<String, CacheEntry>,
    order: VecDeque<String>,
    total_bytes: usize,
    
    // Disk L2 Cache Index
    disk_entries: HashMap<u64, usize>,
    disk_order: VecDeque<u64>,
    disk_total_bytes: usize,
}

impl HttpCache {
    /// Cria uma nova instância com capacidade máxima em bytes (padrão recomendado: 64MB).
    pub fn new(max_capacity_bytes: usize) -> Self {
        Self {
            inner: RwLock::new(HttpCacheInner {
                entries: HashMap::new(),
                order: VecDeque::new(),
                total_bytes: 0,
                disk_entries: HashMap::new(),
                disk_order: VecDeque::new(),
                disk_total_bytes: 0,
            }),
            stats: CacheStats::default(),
            max_capacity_bytes,
            disk_path: None,
        }
    }

    /// Configura um diretório de cache em disco (L2).
    pub fn with_disk_path(mut self, path: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&path);
        self.disk_path = Some(path);
        self
    }

    /// Gera uma chave de cache canônica combinando NIK e URL.
    pub fn make_key(nik: Option<&NetworkIsolationKey>, url: &Url) -> String {
        match nik {
            Some(k) => format!("{}|{}", k.serialize(), url),
            None => format!("global|{}", url),
        }
    }

    /// Função de hash determinística e estável entre execuções do processo (FxHash 64-bit).
    pub fn hash_key(key: &str) -> u64 {
        use rustc_hash::FxHasher;
        use std::hash::Hasher;
        let mut hasher = FxHasher::default();
        hasher.write(key.as_bytes());
        hasher.finish()
    }

    /// Consulta rápida síncrona exclusivamente no cache L1 (Memória RAM).
    pub fn get_memory(&self, nik: Option<&NetworkIsolationKey>, url: &Url) -> Option<CacheEntry> {
        let key = Self::make_key(nik, url);
        let mut inner = self.inner.write();
        if let Some(entry) = inner.entries.get(&key).cloned() {
            if let Some(pos) = inner.order.iter().position(|k| k == &key) {
                inner.order.remove(pos);
                inner.order.push_back(key);
            }
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry)
        } else {
            None
        }
    }

    /// Busca uma entrada no cache. Se encontrada na memória (L1) ou disco (L2), move-a para o topo da ordem LRU.
    pub async fn get(&self, nik: Option<&NetworkIsolationKey>, url: &Url) -> Option<CacheEntry> {
        let key = Self::make_key(nik, url);
        
        // 1. Lookup em memória (L1)
        {
            let mut inner = self.inner.write();
            if let Some(entry) = inner.entries.get(&key).cloned() {
                if let Some(pos) = inner.order.iter().position(|k| k == &key) {
                    inner.order.remove(pos);
                    inner.order.push_back(key.clone());
                }
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry);
            }
        }
        
        // 2. Lookup no Disco (L2) com hash determinístico estável
        if let Some(disk_path) = &self.disk_path {
            let file_name = format!("{:016x}.cache", Self::hash_key(&key));
            let file_path = disk_path.join(file_name);

            if let Ok(bytes) = tokio::fs::read(&file_path).await {
                if let Some(entry) = CacheEntry::from_bytes(&bytes) {
                    let entry_size = entry.body.len() + 256;
                    
                    if entry_size <= self.max_capacity_bytes {
                        let mut inner = self.inner.write();
                        if let Some(old) = inner.entries.remove(&key) {
                            inner.total_bytes = inner.total_bytes.saturating_sub(old.body.len() + 256);
                            if let Some(pos) = inner.order.iter().position(|k| k == &key) {
                                inner.order.remove(pos);
                            }
                        }
                        while inner.total_bytes + entry_size > self.max_capacity_bytes {
                            if let Some(evicted_key) = inner.order.pop_front() {
                                if let Some(evicted_entry) = inner.entries.remove(&evicted_key) {
                                    inner.total_bytes = inner.total_bytes.saturating_sub(evicted_entry.body.len() + 256);
                                    self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                                }
                            } else {
                                break;
                            }
                        }
                        inner.total_bytes += entry_size;
                        inner.order.push_back(key.clone());
                        inner.entries.insert(key, entry.clone());
                        self.stats.current_bytes.store(inner.total_bytes as u64, Ordering::Relaxed);
                    }
                    self.stats.hits.fetch_add(1, Ordering::Relaxed);
                    return Some(entry);
                }
            }
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insere ou atualiza uma resposta no cache, aplicando desalocação LRU se necessário.
    pub fn put(&self, nik: Option<&NetworkIsolationKey>, url: Url, entry: CacheEntry) {
        let key = Self::make_key(nik, &url);
        let entry_size = entry.body.len() + 256; // Overhead aproximado de headers/metadados

        if entry_size <= self.max_capacity_bytes {
            let mut inner = self.inner.write();
            
            // Se já existia, remove o tamanho antigo
            if let Some(old) = inner.entries.remove(&key) {
                inner.total_bytes = inner.total_bytes.saturating_sub(old.body.len() + 256);
                if let Some(pos) = inner.order.iter().position(|k| k == &key) {
                    inner.order.remove(pos);
                }
            }

            // Despeja entradas antigas (LRU) até caber a nova entrada
            while inner.total_bytes + entry_size > self.max_capacity_bytes {
                if let Some(evicted_key) = inner.order.pop_front() {
                    if let Some(evicted_entry) = inner.entries.remove(&evicted_key) {
                        inner.total_bytes = inner.total_bytes.saturating_sub(evicted_entry.body.len() + 256);
                        self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    break;
                }
            }

            inner.total_bytes += entry_size;
            inner.order.push_back(key.clone());
            inner.entries.insert(key.clone(), entry.clone());

            self.stats.current_bytes.store(inner.total_bytes as u64, Ordering::Relaxed);
        }

        if let Some(disk_path) = &self.disk_path {
            let disk_path = disk_path.clone();
            let entry_bytes = entry.to_bytes();
            tokio::spawn(async move {
                let file_name = format!("{:016x}.cache", Self::hash_key(&key));
                let file_path = disk_path.join(file_name);
                let _ = tokio::fs::write(file_path, entry_bytes).await;
            });
        }
    }

    /// Atualiza uma entrada existente após validação `304 Not Modified`.
    pub fn update_304(
        &self,
        nik: Option<&NetworkIsolationKey>,
        url: &Url,
        new_headers: &http::HeaderMap,
        response_time: SystemTime,
    ) -> Option<CacheEntry> {
        let key = Self::make_key(nik, url);
        let mut inner = self.inner.write();

        if let Some(entry) = inner.entries.get_mut(&key) {
            entry.update_from_304(new_headers, response_time);
            let updated = entry.clone();

            if let Some(disk_path) = &self.disk_path {
                let disk_path = disk_path.clone();
                let entry_bytes = updated.to_bytes();
                tokio::spawn(async move {
                    let file_name = format!("{:016x}.cache", Self::hash_key(&key));
                    let file_path = disk_path.join(file_name);
                    let _ = tokio::fs::write(file_path, entry_bytes).await;
                });
            }

            Some(updated)
        } else {
            None
        }
    }

    /// Invalida entradas associadas a uma URL após métodos não seguros (POST, PUT, DELETE) - RFC 9111 §4.4.
    pub fn invalidate(&self, url: &Url) {
        let mut inner = self.inner.write();
        let url_str = url.to_string();

        let keys_to_remove: Vec<String> = inner
            .entries
            .keys()
            .filter(|k| k.ends_with(&url_str))
            .cloned()
            .collect();

        for key in &keys_to_remove {
            if let Some(entry) = inner.entries.remove(key) {
                inner.total_bytes = inner.total_bytes.saturating_sub(entry.body.len() + 256);
                if let Some(pos) = inner.order.iter().position(|k| k == key) {
                    inner.order.remove(pos);
                }
            }
        }

        self.stats.current_bytes.store(inner.total_bytes as u64, Ordering::Relaxed);

        // Exclui arquivos físicos do disco para não ressuscitar dados invalidados
        if let Some(disk_path) = &self.disk_path {
            let disk_path = disk_path.clone();
            tokio::spawn(async move {
                for key in keys_to_remove {
                    let file_name = format!("{:016x}.cache", Self::hash_key(&key));
                    let file_path = disk_path.join(file_name);
                    let _ = tokio::fs::remove_file(file_path).await;
                }
            });
        }
    }

    /// Limpa integralmente todo o cache em memória e no disco.
    pub fn clear(&self) {
        let mut inner = self.inner.write();
        inner.entries.clear();
        inner.order.clear();
        inner.total_bytes = 0;
        self.stats.current_bytes.store(0, Ordering::Relaxed);

        if let Some(disk_path) = &self.disk_path {
            let disk_path = disk_path.clone();
            tokio::spawn(async move {
                if let Ok(mut dir_entries) = tokio::fs::read_dir(disk_path).await {
                    while let Ok(Some(entry)) = dir_entries.next_entry().await {
                        if entry.file_name().to_string_lossy().ends_with(".cache") {
                            let _ = tokio::fs::remove_file(entry.path()).await;
                        }
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{HeaderMap, StatusCode};

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        // Cache com capacidade pequena para 2 itens
        let cache = HttpCache::new(600);

        let now = SystemTime::now();
        let url1 = Url::parse("https://example.com/1").unwrap();
        let url2 = Url::parse("https://example.com/2").unwrap();
        let url3 = Url::parse("https://example.com/3").unwrap();

        let e1 = CacheEntry::new(url1.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from_static(b"data1"), now, now);
        let e2 = CacheEntry::new(url2.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from_static(b"data2"), now, now);
        let e3 = CacheEntry::new(url3.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from_static(b"data3"), now, now);

        cache.put(None, url1.clone(), e1);
        cache.put(None, url2.clone(), e2);

        // Acessa url1 para torná-lo mais recentemente usado
        assert!(cache.get(None, &url1).await.is_some());

        // Inserir url3 deve despejar url2 (menos recentemente usado)
        cache.put(None, url3.clone(), e3);

        assert!(cache.get(None, &url1).await.is_some());
        assert!(cache.get(None, &url3).await.is_some());
        assert!(cache.get(None, &url2).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_unsafe_method() {
        let cache = HttpCache::new(1024 * 1024);
        let url = Url::parse("https://example.com/form").unwrap();
        let now = SystemTime::now();
        let entry = CacheEntry::new(url.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from_static(b"form"), now, now);

        cache.put(None, url.clone(), entry);
        assert!(cache.get(None, &url).await.is_some());

        // Invalida a URL
        cache.invalidate(&url);
        assert!(cache.get(None, &url).await.is_none());
    }
}
