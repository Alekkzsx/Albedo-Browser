use rquickjs::{Class, Ctx, Result};
use crate::ace::json::{self, JsonValue};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Storage {
    #[qjs(skip_trace)]
    data: Arc<Mutex<StorageData>>,
}

struct StorageData {
    items: HashMap<String, String>,
    persistence_path: Option<PathBuf>,
}

impl StorageData {
    /// TODO: add docs
    fn new(path: Option<PathBuf>) -> Self {
        let mut items = HashMap::new();
        if let Some(ref path) = path {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(JsonValue::Object(map)) = json::parse(&content) {
                        for (k, v) in map {
                            if let Some(s) = v.as_string() {
                                items.insert(k, s.to_string());
                            }
                        }
                    }
                }
            }
        }
        Self {
            items,
            persistence_path: path,
        }
    }

    /// TODO: add docs
    fn save(&self) {
        if let Some(ref path) = self.persistence_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut map = HashMap::new();
            for (k, v) in &self.items {
                map.insert(k.clone(), JsonValue::String(v.clone()));
            }
            let json = json::stringify(&JsonValue::Object(map));
            let _ = std::fs::write(path, json);
        }
    }
}

#[rquickjs::methods]
impl Storage {
    #[qjs(rename = "getItem")]
    pub fn get_item(&self, key: String) -> Option<String> {
        self.data.lock().unwrap_or_else(|e| e.into_inner()).items.get(&key).cloned()
    }

    #[qjs(rename = "setItem")]
    pub fn set_item(&self, key: String, value: String) {
        let mut data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        data.items.insert(key, value);
        data.save();
    }

    #[qjs(rename = "removeItem")]
    pub fn remove_item(&self, key: String) {
        let mut data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        data.items.remove(&key);
        data.save();
    }

    /// TODO: add docs
    pub fn clear(&self) {
        let mut data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        data.items.clear();
        data.save();
    }

    /// TODO: add docs
    pub fn key(&self, index: usize) -> Option<String> {
        let data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        data.items.keys().nth(index).cloned()
    }

    #[qjs(get)]
    pub fn length(&self) -> usize {
        self.data.lock().unwrap_or_else(|e| e.into_inner()).items.len()
    }
}

impl Storage {
    /// TODO: add docs
    pub fn new_local(path: PathBuf) -> Self {
        Self {
            data: Arc::new(Mutex::new(StorageData::new(Some(path)))),
        }
    }

    /// TODO: add docs
    pub fn new_session() -> Self {
        Self {
            data: Arc::new(Mutex::new(StorageData::new(None))),
        }
    }

    /// TODO: add docs
    pub fn register(ctx: &Ctx<'_>, name: &str, storage: Self) -> Result<()> {
        let global = ctx.globals();
        let instance = Class::instance(ctx.clone(), storage)?;
        global.set(name, instance)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    #[test]
    fn test_storage_basic() {
        let rt = Runtime::new().expect("Albedo Engine: internal invariant violated");
        let ctx = Context::full(&rt).expect("Albedo Engine: internal invariant violated");

        ctx.with(|ctx| {
            let storage = Storage::new_session();
            storage.set_item("foo".to_string(), "bar".to_string());
            assert_eq!(storage.get_item("foo".to_string()), Some("bar".to_string()));
            assert_eq!(storage.length(), 1);

            storage.remove_item("foo".to_string());
            assert_eq!(storage.get_item("foo".to_string()), None);
            assert_eq!(storage.length(), 0);
        });
    }

    #[test]
    fn test_storage_persistence() {
        let path = std::env::current_dir()
            .expect("Albedo Engine: internal invariant violated")
            .join("target")
            .join("albedo_test_storage.json");
        if path.exists() {
            std::fs::remove_file(&path).expect("Albedo Engine: internal invariant violated");
        }

        {
            let storage = Storage::new_local(path.clone());
            storage.set_item("persistent".to_string(), "data".to_string());
        } // data dropped, should save

        {
            let storage = Storage::new_local(path.clone());
            assert_eq!(
                storage.get_item("persistent".to_string()),
                Some("data".to_string())
            );
        }

        std::fs::remove_file(path).expect("Albedo Engine: internal invariant violated");
    }
}
