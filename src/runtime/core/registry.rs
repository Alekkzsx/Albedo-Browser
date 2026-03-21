use crate::runtime::core::runtime::JsRuntime;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref RUNTIME_REGISTRY: Mutex<HashMap<usize, Arc<Mutex<JsRuntime>>>> =
        Mutex::new(HashMap::new());
}

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
