use super::Element;
use crate::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{Class, Ctx, Object, Result, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct NamedNodeMap {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub index: usize,
}

#[rquickjs::methods]
impl NamedNodeMap {
    #[qjs(get, rename = "length")]
    pub fn length(&self) -> usize {
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(el) = &node.node_type {
                    return el.attributes.len();
                }
            }
        }
        0
    }

    #[qjs(rename = "item")]
    pub fn item<'js>(&self, ctx: Ctx<'js>, index: usize) -> Result<Value<'js>> {
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(el) = &node.node_type {
                    if let Some((name, value)) = el.attributes.iter().nth(index) {
                        return self.create_attr_object(ctx, name.clone(), value.clone());
                    }
                }
            }
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(rename = "getNamedItem")]
    pub fn get_named_item<'js>(&self, ctx: Ctx<'js>, name: String) -> Result<Value<'js>> {
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(el) = &node.node_type {
                    if let Some(value) = el.attributes.get(&name) {
                        return self.create_attr_object(ctx, name, value.clone());
                    }
                }
            }
        }
        Ok(Value::new_null(ctx))
    }
}

impl NamedNodeMap {
    fn create_attr_object<'js>(
        &self,
        ctx: Ctx<'js>,
        name: String,
        value: String,
    ) -> Result<Value<'js>> {
        let obj = Object::new(ctx)?;
        obj.set("name", name)?;
        obj.set("value", value)?;
        // In a real DOM, these are Attr nodes, but plain objects are a good start for compatibility
        Ok(obj.into_value())
    }
}

pub fn attributes<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let attrs = NamedNodeMap {
        dom: el.dom.clone(),
        index: el.index,
    };
    let instance = Class::instance(ctx, attrs)?;
    Ok(instance.into_value())
}
