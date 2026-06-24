use crate::ace::runtime::bindings::html::document::Document;
use crate::ace::runtime::bindings::html::event::EventTargetImpl;
use rquickjs::{Function, Result, Value};

/// TODO: add docs
pub fn add_event_listener<'js>(doc: &Document, type_: String, listener: Function<'js>) {
    let ptr = {
        let dom = doc.dom.lock().unwrap_or_else(|e| e.into_inner());
        dom.root
    };
    // SAFETY: The listener is stored in the DOM and will be called before the QuickJS context
    // is dropped. QuickJS guarantees function references remain valid within the callback scope.
    unsafe {
        let listener_static: Function<'static> = std::mem::transmute(listener);
        EventTargetImpl::add_listener(ptr, type_, listener_static);
    }
}

/// TODO: add docs
pub fn remove_event_listener<'js>(doc: &Document, type_: String, _listener: Function<'js>) {
    let ptr = {
        let dom = doc.dom.lock().unwrap_or_else(|e| e.into_inner());
        dom.root
    };
    EventTargetImpl::remove_listener(ptr, type_);
}

/// TODO: add docs
pub fn dispatch_event<'js>(doc: &Document, event: Value<'js>) -> bool {
    let ptr = {
        let dom = doc.dom.lock().unwrap_or_else(|e| e.into_inner());
        dom.root
    };
    if let Some(obj) = event.as_object() {
        if let Ok(type_val) = obj.get::<_, String>("type") {
            let listeners_static = EventTargetImpl::get_listeners(ptr, &type_val);
            // SAFETY: listeners_static contains valid Function references from the DOM.
            // The transmute converts them back to the original lifetime for invocation.
            let listeners: Vec<Function<'js>> = unsafe { std::mem::transmute(listeners_static) };
            for listener in listeners {
                let _: Result<Value> = listener.call((event.clone(),));
            }
        }
    }
    true
}
