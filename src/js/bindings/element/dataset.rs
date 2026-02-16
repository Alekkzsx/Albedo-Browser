use rquickjs::{Ctx, Result, Value, Class};
use crate::engine::dom::{AceDOM, AceNodeType};
use std::sync::{Arc, Mutex};
use super::mark_mutation;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DomStringMap {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    pub node_idx: usize,
}

#[rquickjs::methods]
impl DomStringMap {
    // Custom get/set logic to handle dataset property access
    // Unfortunately rquickjs::methods doesn't easily support arbitrary property traps like JS Proxy.
    // We might need to use ctx.new_proxy or implement a custom getter in the JS layer.
    // For now, let's implement basic getters/setters and potentially a helper to populate from attributes.
}

impl DomStringMap {
    pub fn new(dom: Arc<Mutex<AceDOM>>, node_idx: usize) -> Self {
        Self { dom, node_idx }
    }
}
