use super::entry::CacheEntry;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, RwLock};

/// Operação do Write-Ahead Log (WAL)
#[derive(Debug, Clone)]
pub enum WalOperation {
    Put { hash: u64, size: u64 },
    Delete { hash: u64 },
    Clear,
}

impl WalOperation {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::new();
        match self {
            Self::Put { hash, size } => {
                buf.put_u8(1);
                buf.put_u64_le(*hash);
                buf.put_u64_le(*size);
            }
            Self::Delete { hash } => {
                buf.put_u8(2);
                buf.put_u64_le(*hash);
            }
            Self::Clear => {
                buf.put_u8(3);
            }
        }
        buf.freeze()
    }

    fn from_bytes(mut buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }
        let op = buf.get_u8();
        match op {
            1 => {
                if buf.remaining() < 16 {
                    return None;
                }
                let hash = buf.get_u64_le();
                let size = buf.get_u64_le();
                Some(Self::Put { hash, size })
            }
            2 => {
                if buf.remaining() < 8 {
                    return None;
                }
                let hash = buf.get_u64_le();
                Some(Self::Delete { hash })
            }
            3 => Some(Self::Clear),
            _ => None,
        }
    }
}

/// Motor de gravação segura no disco usando Write-Ahead Log.
#[derive(Debug, Clone)]
pub struct WalEngine {
    wal_path: PathBuf,
    file: Arc<Mutex<Option<File>>>,
}

impl WalEngine {
    pub async fn new(base_path: &Path) -> std::io::Result<Self> {
        let wal_path = base_path.join("cache.wal");
        let file = OpenOptions::new().create(true).append(true).open(&wal_path).await?;
        Ok(Self {
            wal_path,
            file: Arc::new(Mutex::new(Some(file))),
        })
    }

    pub async fn append(&self, op: &WalOperation) -> std::io::Result<()> {
        let mut guard = self.file.lock().await;
        if let Some(file) = guard.as_mut() {
            let bytes = op.to_bytes();
            // Formato: [Tamanho: 4 bytes][Payload]
            file.write_u32_le(bytes.len() as u32).await?;
            file.write_all(&bytes).await?;
            file.sync_data().await?; // Sincronização atômica
        }
        Ok(())
    }

    pub async fn recover(&self) -> std::io::Result<Vec<WalOperation>> {
        let mut file = File::open(&self.wal_path).await?;
        let mut ops = Vec::new();
        loop {
            let size = match file.read_u32_le().await {
                Ok(s) => s as usize,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };
            if size > 1024 * 1024 {
                break; // Corrupção detectada
            }
            let mut buf = vec![0u8; size];
            if file.read_exact(&mut buf).await.is_err() {
                break; // EOF ou Corrupção
            }
            if let Some(op) = WalOperation::from_bytes(&buf) {
                ops.push(op);
            }
        }
        Ok(ops)
    }
    
    pub async fn clear(&self) -> std::io::Result<()> {
        let mut guard = self.file.lock().await;
        *guard = None; // Fecha arquivo atual
        let _ = tokio::fs::remove_file(&self.wal_path).await;
        *guard = Some(OpenOptions::new().create(true).append(true).open(&self.wal_path).await?);
        Ok(())
    }
}

/// Índice esparso para controle de intervalos de bytes (HTTP 206 Partial Content).
#[derive(Debug, Clone, Default)]
pub struct SparseRangeIndex {
    pub total_length: Option<u64>,
    pub ranges: Vec<(u64, u64)>, // (start, end) inclusivo
}

impl SparseRangeIndex {
    /// Adiciona um range de bytes recebido.
    pub fn add_range(&mut self, start: u64, end: u64) {
        self.ranges.push((start, end));
        self.ranges.sort_by_key(|r| r.0);
        self.merge_ranges();
    }

    /// Mescla ranges sobrepostos.
    fn merge_ranges(&mut self) {
        if self.ranges.is_empty() {
            return;
        }
        let mut merged = Vec::new();
        let mut current = self.ranges[0];

        for &range in &self.ranges[1..] {
            if range.0 <= current.1 + 1 {
                current.1 = current.1.max(range.1);
            } else {
                merged.push(current);
                current = range;
            }
        }
        merged.push(current);
        self.ranges = merged;
    }

    /// Verifica se a requisição possui as partes necessárias.
    pub fn has_range(&self, start: u64, end: u64) -> bool {
        self.ranges.iter().any(|r| r.0 <= start && r.1 >= end)
    }

    /// Verifica se o recurso está completamente baixado.
    pub fn is_complete(&self) -> bool {
        if let Some(total) = self.total_length {
            self.ranges.len() == 1 && self.ranges[0].0 == 0 && self.ranges[0].1 >= total.saturating_sub(1)
        } else {
            false
        }
    }
}

#[derive(Debug)]
struct DiskCacheInner {
    entries: FxHashMap<u64, u64>, // Hash -> Size
    order: VecDeque<u64>,
    total_bytes: u64,
}

/// Motor de Cache em Disco com LRU, WAL e Sparse Range.
#[derive(Debug)]
pub struct DiskCacheEngine {
    base_path: PathBuf,
    max_capacity_bytes: u64,
    wal: WalEngine,
    inner: RwLock<DiskCacheInner>,
    sparse_indices: RwLock<FxHashMap<u64, SparseRangeIndex>>,
}

impl DiskCacheEngine {
    pub async fn new(base_path: PathBuf, max_capacity_bytes: u64) -> std::io::Result<Self> {
        tokio::fs::create_dir_all(&base_path).await?;
        let wal = WalEngine::new(&base_path).await?;
        
        let mut inner = DiskCacheInner {
            entries: FxHashMap::default(),
            order: VecDeque::new(),
            total_bytes: 0,
        };

        // Recovery via WAL
        if let Ok(ops) = wal.recover().await {
            for op in ops {
                match op {
                    WalOperation::Put { hash, size } => {
                        if !inner.entries.contains_key(&hash) {
                            inner.entries.insert(hash, size);
                            inner.order.push_back(hash);
                            inner.total_bytes += size;
                        }
                    }
                    WalOperation::Delete { hash } => {
                        if let Some(size) = inner.entries.remove(&hash) {
                            inner.total_bytes = inner.total_bytes.saturating_sub(size);
                            if let Some(pos) = inner.order.iter().position(|&x| x == hash) {
                                inner.order.remove(pos);
                            }
                        }
                    }
                    WalOperation::Clear => {
                        inner.entries.clear();
                        inner.order.clear();
                        inner.total_bytes = 0;
                    }
                }
            }
        }

        Ok(Self {
            base_path,
            max_capacity_bytes,
            wal,
            inner: RwLock::new(inner),
            sparse_indices: RwLock::new(FxHashMap::default()),
        })
    }

    /// Lê a entrada do cache no disco.
    pub async fn get(&self, hash: u64) -> Option<CacheEntry> {
        let file_path = self.base_path.join(format!("{:016x}.cache", hash));
        if let Ok(bytes) = tokio::fs::read(&file_path).await {
            if let Some(entry) = CacheEntry::from_bytes(&bytes) {
                // Atualiza LRU order
                let mut inner = self.inner.write().await;
                if let Some(pos) = inner.order.iter().position(|&x| x == hash) {
                    inner.order.remove(pos);
                    inner.order.push_back(hash);
                }
                return Some(entry);
            }
        }
        None
    }

    /// Grava uma nova entrada no cache de disco atômico com WAL.
    pub async fn put(&self, hash: u64, entry: CacheEntry) {
        let entry_bytes = entry.to_bytes();
        let entry_size = entry_bytes.len() as u64;

        if entry_size > self.max_capacity_bytes {
            return;
        }

        let mut evictions = Vec::new();
        {
            let mut inner = self.inner.write().await;
            
            if let Some(old_size) = inner.entries.remove(&hash) {
                inner.total_bytes = inner.total_bytes.saturating_sub(old_size);
                if let Some(pos) = inner.order.iter().position(|&x| x == hash) {
                    inner.order.remove(pos);
                }
            }

            while inner.total_bytes + entry_size > self.max_capacity_bytes {
                if let Some(evicted_hash) = inner.order.pop_front() {
                    if let Some(evicted_size) = inner.entries.remove(&evicted_hash) {
                        inner.total_bytes = inner.total_bytes.saturating_sub(evicted_size);
                        evictions.push(evicted_hash);
                    }
                } else {
                    break;
                }
            }

            inner.total_bytes += entry_size;
            inner.order.push_back(hash);
            inner.entries.insert(hash, entry_size);
        }

        // Processa as evictions fisicamente
        for evicted in evictions {
            let _ = self.wal.append(&WalOperation::Delete { hash: evicted }).await;
            let file_path = self.base_path.join(format!("{:016x}.cache", evicted));
            let _ = tokio::fs::remove_file(file_path).await;
        }

        // Grava no WAL
        let _ = self.wal.append(&WalOperation::Put { hash, size: entry_size }).await;

        // Gravação Atômica Física
        let tmp_path = self.base_path.join(format!("{:016x}.tmp", hash));
        let cache_path = self.base_path.join(format!("{:016x}.cache", hash));
        
        if tokio::fs::write(&tmp_path, entry_bytes).await.is_ok() {
            let _ = tokio::fs::rename(tmp_path, cache_path).await;
        }
    }

    pub async fn delete(&self, hash: u64) {
        let _ = self.wal.append(&WalOperation::Delete { hash }).await;
        let mut inner = self.inner.write().await;
        if let Some(size) = inner.entries.remove(&hash) {
            inner.total_bytes = inner.total_bytes.saturating_sub(size);
            if let Some(pos) = inner.order.iter().position(|&x| x == hash) {
                inner.order.remove(pos);
            }
        }
        let file_path = self.base_path.join(format!("{:016x}.cache", hash));
        let _ = tokio::fs::remove_file(file_path).await;
    }

    pub async fn clear(&self) {
        let _ = self.wal.clear().await;
        let mut inner = self.inner.write().await;
        inner.entries.clear();
        inner.order.clear();
        inner.total_bytes = 0;
        
        if let Ok(mut dir_entries) = tokio::fs::read_dir(&self.base_path).await {
            while let Ok(Some(entry)) = dir_entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".cache") || name.ends_with(".tmp") {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
    }
}
