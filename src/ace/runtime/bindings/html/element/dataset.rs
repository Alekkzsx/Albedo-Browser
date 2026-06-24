use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DomStringMap {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    pub node_idx: usize,
}

#[rquickjs::methods]
impl DomStringMap {
    /// TODO: add docs
    pub fn get(&self, name: String) -> Option<String> {
        let attr_name = format!("data-{}", name); // Basic mapping
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.node_idx) {
                if let AceNodeType::Element(el) = &node.node_type {
                    return el.attributes.get(&attr_name).cloned();
                }
            }
        }
        None
    }

    /// TODO: add docs
    pub fn set(&self, name: String, value: String) {
        let attr_name = format!("data-{}", name);
        if let Ok(mut dom) = self.dom.lock() {
            dom.set_attribute_notify(self.node_idx, attr_name, value);
        }
    }
}

impl DomStringMap {
    /// TODO: add docs
    pub fn new(dom: Arc<Mutex<AceDOM>>, node_idx: usize) -> Self {
        Self { dom, node_idx }
    }
}

use super::Element;
/// TODO: add docs
pub fn dataset<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let ds = DomStringMap::new(el.dom.clone(), el.index);
    let instance = Class::instance(ctx, ds)?;
    Ok(instance.into_value())
}
