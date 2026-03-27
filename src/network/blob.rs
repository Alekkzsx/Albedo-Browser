use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

/// Represents a Blob object in memory
#[derive(Debug, Clone)]
pub struct Blob {
    pub data: Vec<u8>,
    pub content_type: String,
    pub size: usize,
}

/// Thread-safe store for Blob objects
#[derive(Debug, Clone)]
pub struct BlobStore {
    blobs: Arc<Mutex<HashMap<String, Blob>>>,
}

impl BlobStore {
    pub fn new() -> Self {
        Self {
            blobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new blob and return its URL (blob:uuid)
    pub fn create_blob(&self, data: Vec<u8>, content_type: String) -> String {
        let size = data.len();
        let blob = Blob {
            data,
            content_type,
            size,
        };

        let uuid = crate::ace::util::uuid::Uuid::new_v4().to_string();
        let url = format!("blob:{}", uuid);

        let mut blobs = self.blobs.lock().unwrap();
        blobs.insert(url.clone(), blob);

        url
    }

    /// Get a blob by its URL
    pub fn get_blob(&self, url: &str) -> Option<Blob> {
        let blobs = self.blobs.lock().unwrap();
        blobs.get(url).cloned()
    }

    /// Remove a blob by its URL (revokeObjectURL)
    pub fn revoke_blob(&self, url: &str) {
        let mut blobs = self.blobs.lock().unwrap();
        blobs.remove(url);
    }
}

// Global instance helper (if needed, but usually passed via ResourceManager)
pub static GLOBAL_BLOB_STORE: LazyLock<BlobStore> = LazyLock::new(BlobStore::new);
