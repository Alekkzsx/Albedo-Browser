use crate::js::bindings::document::Document;
use crate::js::bindings::event::EventTargetImpl;
use rquickjs::{Function, Result, Value};

pub fn add_event_listener<'js>(doc: &Document, type_: String, listener: Function<'js>) {
    let ptr = {
        let dom = doc.dom.lock().unwrap();
        dom.root
    };
    unsafe {
        let listener_static: Function<'static> = std::mem::transmute(listener);
        EventTargetImpl::add_listener(ptr, type_, listener_static);
    }
}

pub fn remove_event_listener<'js>(doc: &Document, type_: String, _listener: Function<'js>) {
    let ptr = {
        let dom = doc.dom.lock().unwrap();
        dom.root
    };
    EventTargetImpl::remove_listener(ptr, type_);
}

pub fn dispatch_event<'js>(doc: &Document, event: Value<'js>) -> bool {
    let ptr = {
        let dom = doc.dom.lock().unwrap();
        dom.root
    };
    if let Some(obj) = event.as_object() {
            if let Ok(type_val) = obj.get::<_, String>("type") {
                let listeners_static = EventTargetImpl::get_listeners(ptr, &type_val);
                let listeners: Vec<Function<'js>> = unsafe { std::mem::transmute(listeners_static) };
                for listener in listeners {
                    let _: Result<Value> = listener.call((event.clone(),));
                }
            }
    }
    true 
}
