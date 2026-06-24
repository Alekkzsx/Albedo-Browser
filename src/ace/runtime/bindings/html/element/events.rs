use super::Element;
use crate::ace::runtime::bindings::html::event::EventTargetImpl;
use rquickjs::{Ctx, Function, Result, Value};

/// TODO: add docs
pub fn add_event_listener<'js>(el: &Element, type_: String, listener: Function<'js>) {
    let ptr = el.index;
    // SAFETY: The listener is stored in the DOM and will be called before the QuickJS context
    // is dropped. QuickJS guarantees function references remain valid within the callback scope.
    unsafe {
        let listener_static: Function<'static> = std::mem::transmute(listener);
        EventTargetImpl::add_listener(ptr, type_, listener_static);
    }
}

/// TODO: add docs
pub fn remove_event_listener<'js>(el: &Element, type_: String, _listener: Function<'js>) {
    let ptr = el.index;
    EventTargetImpl::remove_listener(ptr, type_);
}

/// TODO: add docs
pub fn dispatch_event_internal<'js>(el: &Element, ctx: &Ctx<'js>, event: Value<'js>) -> bool {
    dispatch_event(el, ctx, event)
}

/// TODO: add docs
pub fn dispatch_event<'js>(el: &Element, _ctx: &Ctx<'js>, event: Value<'js>) -> bool {
    let ptr = el.index;
    let dom = el.dom.clone();

    // Helper to find parent
    let get_parent = move |p: usize| -> Option<usize> {
        if let Ok(d) = dom.lock() {
            if let Some(node) = d.get_node(p) {
                return node.parent;
            }
        }
        None
    };

    if let Some(event_obj_js) = event.as_object() {
        if let Ok(type_val) = event_obj_js.get::<_, String>("type") {
            let bubbles = event_obj_js.get::<_, bool>("bubbles").unwrap_or(false);

            let event_data = crate::ace::runtime::bindings::html::event::Event {
                type_: type_val.clone(),
                bubbles,
                cancelable: false,
                target: None,
                current_target: None,
                cancel_bubble: false,
            };

            // We should set target to the element, but we can't easily pass Element object here
            // because it's wrapped in Class and we are in engine logic.
            // The JS listener wrapper usually handles `this`?
            // For now, ignoring `target` setting on JS object perfectly.

            let listeners_chain =
                EventTargetImpl::dispatch_event_with_bubbling(ptr, &event_data, get_parent);

            let mut last_ptr = 0;
            if let Some((first, _)) = listeners_chain.first() {
                last_ptr = *first;
            }

            for (curr_ptr, listener_static) in listeners_chain {
                let event_obj = event_obj_js.clone();

                let propagation_stopped = event_obj
                    .get::<_, bool>("_propagationStopped")
                    .unwrap_or(false);
                let immediate_stopped = event_obj
                    .get::<_, bool>("_immediatePropagationStopped")
                    .unwrap_or(false);

                if curr_ptr != last_ptr {
                    if propagation_stopped {
                        break;
                    }
                    last_ptr = curr_ptr;
                }

                if immediate_stopped {
                    continue;
                }

                // SAFETY: listener_static contains valid Function references from the DOM.
                // The transmute converts them back to the original lifetime for invocation.
                let listener: Function<'js> = unsafe { std::mem::transmute(listener_static) };
                let _: Result<Value> = listener.call((event.clone(),));
            }
        }
    }
    true
}
