use rquickjs::{Class, Ctx, Object, Result, Value, atom::PredefinedAtom, IntoJs, FromJs};
use std::sync::{Arc, Mutex};
use crate::engine::dom::{AceDOM, MutationObserverInit};

#[derive(Clone)]
#[rquickjs::class]
pub struct MutationObserver {
    callback: rquickjs::Persistent<rquickjs::Function<'static>>,
    dom: Arc<Mutex<AceDOM>>,
    id: usize,
}

#[rquickjs::methods]
impl MutationObserver {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, callback: Value<'_>) -> Result<Self> {
        let dom = ctx.userdata::<Arc<Mutex<AceDOM>>>().unwrap().clone();
        // Generate a simple unique ID for this observer instance (could be improved)
        let id = {
            let d = dom.lock().unwrap();
            d.observers.len() + 1000 // Offset to avoid collision with node IDs if any
        };
        
        let func = callback.into_function().ok_or(rquickjs::Error::new_from_js("Callback must be a function", "TypeError"))?;
        let persistent_callback = rquickjs::Persistent::save(ctx, func);

        Ok(MutationObserver {
            callback: persistent_callback,
            dom,
            id,
        })
    }

    pub fn observe(&mut self, target: Value<'_>, options: Object<'_>) -> Result<()> {
        // Extract node ID from target (assuming target is an Element wrapper with node_idx)
        // This part needs to align with how Elements are exposed. 
        // For now, let's assume target has a node_idx property or we can get it from userdata if it's a Class.
        // But since we don't have the Element class definition here, we might need to assume it's passed as an object with node_idx.
        // Or better, we should accept the Element class if possible.
        
        // Simplified: expect target to have "nodeId" property
        let target_id: usize = if let Some(obj) = target.as_object() {
             // In a real implementation this would unwrap the Element class
             // For now let's hope it has a property we can read, or we need a way to unwrap Class<Element>
             // Let's assume we can get it from a property "node_idx" which we should expose on Element
             obj.get("node_idx").unwrap_or(0)
        } else {
            0
        };

        let init = MutationObserverInit {
            child_list: options.get("childList").unwrap_or(false),
            attributes: options.get("attributes").unwrap_or(false),
            character_data: options.get("characterData").unwrap_or(false),
            subtree: options.get("subtree").unwrap_or(false),
            attribute_old_value: options.get("attributeOldValue").unwrap_or(false),
            character_data_old_value: options.get("characterDataOldValue").unwrap_or(false),
        };

        let mut dom = self.dom.lock().unwrap();
        dom.observe(target_id, init, self.id);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        // TODO: Implement removal of observer from DOM
    }

    pub fn take_records(&self) -> Vec<Value<'static>> {
        // TODO: Implement taking records
        Vec::new()
    }
}
