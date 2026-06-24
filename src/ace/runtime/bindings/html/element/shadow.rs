use super::Element;
use crate::ace::runtime::bindings::html::document::fragment::DocumentFragment;
use rquickjs::{Class, Ctx, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct ShadowRoot {
    pub fragment: DocumentFragment,
}

#[rquickjs::methods]
impl ShadowRoot {
    #[qjs(get, rename = "host")]
    pub fn host<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // Find element that has this shadow root
        if let Ok(dom) = self.fragment.dom.lock() {
            for (idx, node) in dom.nodes.iter().enumerate() {
                if node.shadow_root == Some(self.fragment.index) {
                    let element = Element {
                        dom: self.fragment.dom.clone(),
                        index: idx,
                        mutations: self.fragment.mutations.clone(),
                        stylesheet_dirty: self.fragment.stylesheet_dirty.clone(),
                        primitives: self.fragment.primitives.clone(),
                        canvas_contexts: self.fragment.canvas_contexts.clone(),
                        pending_scroll: self.fragment.pending_scroll.clone(),
                        element_geometry: self.fragment.element_geometry.clone(),
                        element_scroll: self.fragment.element_scroll.clone(),
                    };
                    let instance = Class::instance(ctx, element)?;
                    return Ok(instance.into_value());
                }
            }
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, ctx: Ctx<'js>, child: Value<'js>) -> Result<Value<'js>> {
        super::hierarchy::append_child_generic(
            &self.fragment.dom,
            self.fragment.index,
            &self.fragment.mutations,
            ctx,
            child,
        )
    }
}

/// TODO: add docs
pub fn attach_shadow<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let shadow_idx = if let Ok(mut dom) = el.dom.lock() {
        dom.attach_shadow(el.index)
    } else {
        return Err(rquickjs::Error::new_from_js("DOM Lock error", "Error"));
    };

    let shadow = ShadowRoot {
        fragment: DocumentFragment {
            dom: el.dom.clone(),
            index: shadow_idx,
            mutations: el.mutations.clone(),
            stylesheet_dirty: el.stylesheet_dirty.clone(),
            primitives: el.primitives.clone(),
            canvas_contexts: el.canvas_contexts.clone(),
            pending_scroll: el.pending_scroll.clone(),
            element_geometry: el.element_geometry.clone(),
            element_scroll: el.element_scroll.clone(),
        },
    };

    let instance = Class::instance(ctx, shadow)?;
    Ok(instance.into_value())
}

/// TODO: add docs
pub fn get_shadow_root<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let Some(shadow_idx) = node.shadow_root {
                let shadow = ShadowRoot {
                    fragment: DocumentFragment {
                        dom: el.dom.clone(),
                        index: shadow_idx,
                        mutations: el.mutations.clone(),
                        stylesheet_dirty: el.stylesheet_dirty.clone(),
                        primitives: el.primitives.clone(),
                        canvas_contexts: el.canvas_contexts.clone(),
                        pending_scroll: el.pending_scroll.clone(),
                        element_geometry: el.element_geometry.clone(),
                        element_scroll: el.element_scroll.clone(),
                    },
                };
                let instance = Class::instance(ctx, shadow)?;
                return Ok(instance.into_value());
            }
        }
    }
    Ok(Value::new_null(ctx))
}
