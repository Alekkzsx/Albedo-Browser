#![allow(dead_code)]

use super::entry::CacheEntry;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
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

/// Registro esparso de fatias de bytes contíguas armazenadas para um recurso parcial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseRangeIndex {
    pub ranges: Vec<(u64, u64)>, // (start, end) inclusivos
    pub total_length: Option<u64>,
}

impl SparseRangeIndex {
    pub fn new() -> Self {
        Self { ranges: Vec::new(), total_length: None }
    }

    /// Adiciona uma nova faixa e funde faixas sobrepostas/adjacentes.
    pub fn add_range(&mut self, start: u64, end: u64) {
        self.ranges.push((start, end));
        self.ranges.sort_unstable_by_key(|k| k.0);
        
        let mut merged: Vec<(u64, u64)> = Vec::with_capacity(self.ranges.len());
        for current in &self.ranges {
            if let Some(last) = merged.last_mut() {
                if current.0 <= last.1 + 1 {
                    last.1 = std::cmp::max(last.1, current.1);
                } else {
                    merged.push(*current);
                }
            } else {
                merged.push(*current);
            }
        }
        self.ranges = merged;
    }

    /// Verifica se a faixa requisitada está completamente armazenada no cache.
    pub fn contains_range(&self, req_start: u64, req_end: u64) -> bool {
        self.ranges.iter().any(|&(s, e)| s <= req_start && req_end <= e)
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
        
        // Clean up orphan .tmp files
        let mut read_dir = tokio::fs::read_dir(&base_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if let Some(ext) = entry.path().extension() {
                if ext == "tmp" {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
        
        let wal = WalEngine::new(&base_path).await?;
        
        let mut inner = DiskCacheInner {
            entries: FxHashMap::default(),
            order: VecDeque::new(),
            total_bytes: 0,
        };
        let mut sparse = FxHashMap::default();

        // 1. Tenta carregar o índice binário primeiro
        if let Ok((loaded_inner, loaded_sparse)) = Self::load_index(&base_path).await {
            inner = loaded_inner;
            sparse = loaded_sparse;
        }

        // 2. Toca os eventos do WAL
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
                        sparse.remove(&hash);
                    }
                    WalOperation::Clear => {
                        inner.entries.clear();
                        inner.order.clear();
                        inner.total_bytes = 0;
                        sparse.clear();
                    }
                }
            }
        }

        let engine = Self {
            base_path,
            max_capacity_bytes,
            wal,
            inner: RwLock::new(inner),
            sparse_indices: RwLock::new(sparse),
        };
        
        // 3. Ao inicializar, podemos forçar o despejo do índice e zerar o WAL para economizar tempo na próxima inicialização
        let _ = engine.save_index().await;

        Ok(engine)
    }

    /// Carrega o índice binário `cache.idx`
    async fn load_index(base_path: &Path) -> std::io::Result<(DiskCacheInner, FxHashMap<u64, SparseRangeIndex>)> {
        let mut inner = DiskCacheInner {
            entries: FxHashMap::default(),
            order: VecDeque::new(),
            total_bytes: 0,
        };
        let mut sparse = FxHashMap::default();

        let idx_path = base_path.join("cache.idx");
        let bytes = tokio::fs::read(&idx_path).await?;
        let mut buf = bytes.as_slice();

        if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
        let magic = buf.get_u64_le();
        if magic != 0xACE1_DF20_C001_CACA {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Bad Magic"));
        }

        if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
        inner.total_bytes = buf.get_u64_le();

        if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
        let num_entries = buf.get_u64_le() as usize;
        for _ in 0..num_entries {
            if buf.remaining() < 16 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
            let hash = buf.get_u64_le();
            let size = buf.get_u64_le();
            inner.entries.insert(hash, size);
        }

        if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
        let num_order = buf.get_u64_le() as usize;
        for _ in 0..num_order {
            if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
            let hash = buf.get_u64_le();
            inner.order.push_back(hash);
        }

        if buf.remaining() >= 8 {
            let num_sparse = buf.get_u64_le() as usize;
            for _ in 0..num_sparse {
                if buf.remaining() < 17 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
                let hash = buf.get_u64_le();
                let has_total = buf.get_u8() == 1;
                let total_length = if has_total {
                    if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
                    Some(buf.get_u64_le())
                } else {
                    None
                };
                if buf.remaining() < 8 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
                let num_ranges = buf.get_u64_le() as usize;
                let mut ranges = Vec::with_capacity(num_ranges);
                for _ in 0..num_ranges {
                    if buf.remaining() < 16 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "EOF")); }
                    let start = buf.get_u64_le();
                    let end = buf.get_u64_le();
                    ranges.push((start, end));
                }
                sparse.insert(hash, SparseRangeIndex { total_length, ranges });
            }
        }

        Ok((inner, sparse))
    }

    /// Salva o estado atual em um índice binário e zera o WAL.
    pub async fn save_index(&self) -> std::io::Result<()> {
        let inner = self.inner.read().await;
        let sparse = self.sparse_indices.read().await;
        
        let mut buf = BytesMut::new();
        // Magic Number
        buf.put_u64_le(0xACE1_DF20_C001_CACA);
        buf.put_u64_le(inner.total_bytes);
        
        buf.put_u64_le(inner.entries.len() as u64);
        for (hash, size) in &inner.entries {
            buf.put_u64_le(*hash);
            buf.put_u64_le(*size);
        }
        
        buf.put_u64_le(inner.order.len() as u64);
        for hash in &inner.order {
            buf.put_u64_le(*hash);
        }

        buf.put_u64_le(sparse.len() as u64);
        for (hash, index) in sparse.iter() {
            buf.put_u64_le(*hash);
            if let Some(total) = index.total_length {
                buf.put_u8(1);
                buf.put_u64_le(total);
            } else {
                buf.put_u8(0);
            }
            buf.put_u64_le(index.ranges.len() as u64);
            for &(start, end) in &index.ranges {
                buf.put_u64_le(start);
                buf.put_u64_le(end);
            }
        }

        let bytes = buf.freeze();
        let tmp_path = self.base_path.join("cache.idx.tmp");
        let target_path = self.base_path.join("cache.idx");
        tokio::fs::write(&tmp_path, bytes).await?;
        tokio::fs::rename(tmp_path, target_path).await?;
        
        // Zera o WAL agora que temos o dump perfeito
        self.wal.clear().await?;
        
        Ok(())
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

    /// Grava uma fatia de bytes parciais (HTTP 206) no arquivo de cache via offset.
    pub async fn put_range(&self, hash: u64, start: u64, end: u64, total_size: Option<u64>, bytes: Bytes) -> std::io::Result<()> {
        let size = bytes.len() as u64;
        let file_path = self.base_path.join(format!("{:016x}.cache", hash));

        // Open with create and write modes
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)
            .await?;

        // Seek to the start offset
        file.seek(std::io::SeekFrom::Start(start)).await?;
        file.write_all(&bytes).await?;
        file.sync_all().await?;

        // Update sparse index
        {
            let mut sparse_lock = self.sparse_indices.write().await;
            let entry = sparse_lock.entry(hash).or_insert_with(|| SparseRangeIndex::new());
            entry.add_range(start, end);
            if let Some(t) = total_size {
                entry.total_length = Some(t);
            }
        }

        // Add to WAL and inner state
        let mut evictions = Vec::new();
        {
            let mut inner = self.inner.write().await;
            
            while inner.total_bytes + size > self.max_capacity_bytes {
                if let Some(evicted_hash) = inner.order.pop_front() {
                    if let Some(evicted_size) = inner.entries.remove(&evicted_hash) {
                        inner.total_bytes = inner.total_bytes.saturating_sub(evicted_size);
                        evictions.push(evicted_hash);
                    }
                } else {
                    break;
                }
            }

            if !inner.entries.contains_key(&hash) {
                inner.entries.insert(hash, size);
                inner.order.push_back(hash);
                inner.total_bytes += size;
            } else {
                if let Some(s) = inner.entries.get_mut(&hash) {
                    *s += size;
                    inner.total_bytes += size;
                }
            }
        }
        
        for evicted in evictions {
            let _ = self.wal.append(&WalOperation::Delete { hash: evicted }).await;
            let p = self.base_path.join(format!("{:016x}.cache", evicted));
            let _ = tokio::fs::remove_file(p).await;
            let mut sparse_lock = self.sparse_indices.write().await;
            sparse_lock.remove(&evicted);
        }

        let _ = self.wal.append(&WalOperation::Put { hash, size }).await;

        Ok(())
    }

    /// Lê uma fatia específica de bytes do cache (se disponível)
    pub async fn get_range(&self, hash: u64, start: u64, end: u64) -> Option<Bytes> {
        // Verifica se a fatia solicitada está coberta pelos registros esparsos
        {
            let sparse_lock = self.sparse_indices.read().await;
            if let Some(index) = sparse_lock.get(&hash) {
                if !index.contains_range(start, end) {
                    return None;
                }
            } else {
                return None;
            }
        }

        let inner = self.inner.read().await;
        if !inner.entries.contains_key(&hash) {
            return None;
        }
        
        let file_path = self.base_path.join(format!("{:016x}.cache", hash));
        let mut file = tokio::fs::OpenOptions::new().read(true).open(&file_path).await.ok()?;
        
        let length = (end - start + 1) as usize;
        let mut buf = vec![0u8; length];
        
        file.seek(std::io::SeekFrom::Start(start)).await.ok()?;
        file.read_exact(&mut buf).await.ok()?;
        
        Some(Bytes::from(buf))
    }
}


