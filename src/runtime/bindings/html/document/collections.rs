use rquickjs::{Ctx, Result, Value, Class, Object, Array};
use crate::engine::dom::{AceDOM, AceNodeType};
use crate::runtime::bindings::html::element::Element;
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct HtmlCollection {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub selector_fn: Arc<dyn Fn(&AceDOM, usize) -> bool + Send + Sync>,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
}

#[rquickjs::methods]
impl HtmlCollection {
    #[qjs(get, rename = "length")]
    pub fn length(&self) -> usize {
        self.get_elements_indices().len()
    }

    #[qjs(rename = "item")]
    pub fn item<'js>(&self, ctx: Ctx<'js>, index: usize) -> Result<Value<'js>> {
        let indices = self.get_elements_indices();
        if let Some(&node_idx) = indices.get(index) {
            let element = Element {
                dom: self.dom.clone(),
                index: node_idx,
                mutations: self.mutations.clone(),
                stylesheet_dirty: self.stylesheet_dirty.clone(),
                primitives: self.primitives.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
        Ok(Value::new_null(ctx))
    }

    // Support array-like access via a helper or just JS proxy (if we could easily do Proxy in Rust-QuickJS)
    // For now, we'll provide a toArray() or similar, but the spec says it's indexed.
    // QuickJS classes can have custom get/set if we implement them, but rquickjs simplifies things.
}

impl HtmlCollection {
    fn get_elements_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        if let Ok(dom) = self.dom.lock() {
            for (i, node) in dom.nodes.iter().enumerate() {
                if (self.selector_fn)(&dom, i) {
                    indices.push(i);
                }
            }
        }
        indices
    }

    pub fn to_array<'js>(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let array = Array::new(ctx.clone())?;
        let indices = self.get_elements_indices();
        for (i, &node_idx) in indices.iter().enumerate() {
            let element = Element {
                dom: self.dom.clone(),
                index: node_idx,
                mutations: self.mutations.clone(),
                stylesheet_dirty: self.stylesheet_dirty.clone(),
                primitives: self.primitives.clone(),
            };
            let instance = Class::instance(ctx.clone(), element)?;
            array.set(i, instance)?;
        }
        Ok(array)
    }
}
