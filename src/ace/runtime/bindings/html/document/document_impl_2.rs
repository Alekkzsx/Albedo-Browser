use super::*;
use super::element::Element;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::ace::engine::style::Stylesheet;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Document {

    #[qjs(rename = "createDocumentFragment")]
    pub fn create_document_fragment<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let node_index = if let Ok(mut dom) = self.dom.lock() {
            let node_index = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::DocumentFragment,
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
            });
            node_index
        } else {
            return Ok(Value::new_null(ctx));
        };

        let frag = crate::ace::runtime::bindings::html::document::fragment::DocumentFragment {
            dom: self.dom.clone(),
            index: node_index,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };

        let instance = Class::instance(ctx, frag)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createTextNode")]
    pub fn create_text_node<'js>(&self, ctx: Ctx<'js>, text: String) -> Result<Value<'js>> {
        let node_index = if let Ok(mut dom) = self.dom.lock() {
            let node_index = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Text(std::sync::Arc::from(text)),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
            });
            node_index
        } else {
            return Ok(Value::new_null(ctx));
        };

        let element = Element {
            dom: self.dom.clone(),
            index: node_index,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createComment")]
    pub fn create_comment<'js>(&self, ctx: Ctx<'js>, data: String) -> Result<Value<'js>> {
        let node_index = if let Ok(mut dom) = self.dom.lock() {
            let node_index = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Comment(std::sync::Arc::from(data)),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
            });
            node_index
        } else {
            return Ok(Value::new_null(ctx));
        };

        let element = Element {
            dom: self.dom.clone(),
            index: node_index,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }
}
