use crate::runtime::core::runtime::JsRuntime;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static RUNTIME_REGISTRY: LazyLock<Mutex<HashMap<usize, Arc<Mutex<JsRuntime>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn register_runtime(id: usize, rt: Arc<Mutex<JsRuntime>>) {
    let mut registry = RUNTIME_REGISTRY.lock().unwrap();
    registry.insert(id, rt);
}

pub fn unregister_runtime(id: usize) {
    let mut registry = RUNTIME_REGISTRY.lock().unwrap();
    registry.remove(&id);
}

pub fn get_runtime(id: usize) -> Option<Arc<Mutex<JsRuntime>>> {
    let registry = RUNTIME_REGISTRY.lock().unwrap();
    registry.get(&id).cloned()
}
