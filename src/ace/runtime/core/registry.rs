use crate::ace::runtime::core::runtime::JsRuntime;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static RUNTIME_REGISTRY: LazyLock<Mutex<HashMap<usize, Arc<Mutex<JsRuntime>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// TODO: add docs
pub fn register_runtime(id: usize, rt: Arc<Mutex<JsRuntime>>) {
    let mut registry = RUNTIME_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    registry.insert(id, rt);
}

/// TODO: add docs
pub fn unregister_runtime(id: usize) {
    let mut registry = RUNTIME_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    registry.remove(&id);
}

/// TODO: add docs
pub fn get_runtime(id: usize) -> Option<Arc<Mutex<JsRuntime>>> {
    let registry = RUNTIME_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    registry.get(&id).cloned()
}
