use rquickjs::{Ctx, Result, Value, Function};
use std::collections::HashMap;
use std::cell::RefCell;

// Thread-local event registry to avoid Send/Sync issues with QuickJS values
type ListenerRegistry = HashMap<usize, HashMap<String, Vec<Listener>>>;

thread_local! {
    static REGISTRY: RefCell<ListenerRegistry> = RefCell::new(HashMap::new());
}

struct Listener {
    callback: Function<'static>, 
    // options: ...
}

// Helper to manage listeners
pub struct EventTargetImpl;

impl EventTargetImpl {
    pub fn add_listener(target_ptr: usize, type_: String, callback: Function<'static>) {
        REGISTRY.with(|registry| {
            let mut map = registry.borrow_mut();
            let node_listeners = map.entry(target_ptr).or_insert_with(HashMap::new);
            let list = node_listeners.entry(type_).or_insert_with(Vec::new);
            list.push(Listener { callback });
        });
    }

    pub fn remove_listener(target_ptr: usize, type_: String) {
        REGISTRY.with(|registry| {
            let mut map = registry.borrow_mut();
            if let Some(node_listeners) = map.get_mut(&target_ptr) {
                node_listeners.remove(&type_);
            }
        });
    }
    
    // Dispatch helper - returns callbacks to execute
    // We return Vec<Function> so the caller (Element method) can call them 
    // while holding the context, avoiding borrowing issues with REGISTRY
    pub fn get_listeners(target_ptr: usize, type_: &str) -> Vec<Function<'static>> {
        REGISTRY.with(|registry| {
            let map = registry.borrow();
            if let Some(node_listeners) = map.get(&target_ptr) {
                if let Some(list) = node_listeners.get(type_) {
                    return list.iter().map(|l| l.callback.clone()).collect();
                }
            }
            Vec::new()
        })
    }

    pub fn clear_all() {
        REGISTRY.with(|registry| {
            registry.borrow_mut().clear();
        });
    }

    // New helper for full dispatch flow (capture -> target -> bubble)
    // Returns true if event was not cancelled
    pub fn dispatch_event_with_bubbling(
        target_ptr: usize, 
        event_obj: &Event, // Rust Event struct
        get_parent_fn: impl Fn(usize) -> Option<usize>
    ) -> Vec<(usize, Function<'static>)> {
        // Collect propagation path
        let mut path = Vec::new();
        let mut curr = target_ptr;
        path.push(curr);
        
        if event_obj.bubbles {
            while let Some(parent) = get_parent_fn(curr) {
                path.push(parent);
                curr = parent;
            }
        }
        
        let mut listeners_to_call = Vec::new();
        
        // Bubbling Phase (Target -> Root)
        // For now MVP: Just bubbling phase + target
        // Standard is: Capture (Root->Target), Target, Bubble (Target->Root)
        // But we store listeners in a simple map. 
        // Let's implement bubbling order: Target -> Root
        
        for ptr in path.iter() {
             let listeners = Self::get_listeners(*ptr, &event_obj.type_);
             for l in listeners {
                 listeners_to_call.push((*ptr, l));
             }
        }
        
        listeners_to_call
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Event {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,
    
    // We can't easily store Element here because it contains NodeRef which is not Trace?
    // Actually Element is Trace. 
    // But we need to set these during dispatch.
    // Making them mutable generic Option<Value<'js>>
    #[qjs(skip_trace)] // managing manually or just ref references?
    pub target: Option<Value<'static>>, 
    #[qjs(skip_trace)]
    pub current_target: Option<Value<'static>>,
}

#[rquickjs::methods]
impl Event {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut bubbles = false;
        let mut cancelable = false;
        
        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                bubbles = obj.get("bubbles").unwrap_or(false);
                cancelable = obj.get("cancelable").unwrap_or(false);
            }
        }
        
        Self {
            type_,
            bubbles, 
            cancelable,
            target: None,
            current_target: None,
        }
    }
    
    #[qjs(get)]
    pub fn target<'js>(&self, ctx: Ctx<'js>) -> Value<'js> {
        // This is tricky. We are storing static values or need to resurrect?
        // Let's simlify: dispatch passes the element to the callback? 
        // Or we set a property on the JS object before calling the callback.
        Value::new_null(ctx) 
    }

    #[qjs(rename = "stopPropagation")]
    pub fn stop_propagation<'js>(&self, this: rquickjs::Object<'js>) {
        let _ = this.set("_propagationStopped", true);
    }

    #[qjs(rename = "stopImmediatePropagation")]
    pub fn stop_immediate_propagation<'js>(&self, this: rquickjs::Object<'js>) {
        let _ = this.set("_immediatePropagationStopped", true);
        let _ = this.set("_propagationStopped", true);
    }

    #[qjs(rename = "preventDefault")]
    pub fn prevent_default<'js>(&self, this: rquickjs::Object<'js>) {
        if self.cancelable {
            let _ = this.set("_defaultPrevented", true);
        }
    }

    #[qjs(get, rename = "defaultPrevented")]
    pub fn default_prevented<'js>(&self, this: rquickjs::Object<'js>) -> bool {
        this.get("_defaultPrevented").unwrap_or(false)
    }
}
