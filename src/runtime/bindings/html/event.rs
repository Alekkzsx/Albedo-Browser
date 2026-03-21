use rquickjs::{Ctx, Function, Persistent, Result, Value};
use std::cell::RefCell;
use std::collections::HashMap;

type ListenerRegistry = HashMap<usize, HashMap<String, Vec<Listener>>>;

thread_local! {
    static REGISTRY: RefCell<ListenerRegistry> = RefCell::new(HashMap::new());
}

struct Listener {
    callback: Function<'static>,
}

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

    pub fn dispatch_event_with_bubbling(
        target_ptr: usize,
        event_obj: &Event,
        get_parent_fn: impl Fn(usize) -> Option<usize>,
    ) -> Vec<(usize, Function<'static>)> {
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
        for ptr in path.iter() {
            let listeners = Self::get_listeners(*ptr, &event_obj.type_);
            for l in listeners {
                listeners_to_call.push((*ptr, l));
            }
        }
        listeners_to_call
    }
}

#[derive(Clone)]
#[rquickjs::class]
pub struct Event {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,

    pub target: Option<Persistent<Value<'static>>>,
    pub current_target: Option<Persistent<Value<'static>>>,
    #[qjs(get, rename = "cancelBubble")]
    pub cancel_bubble: bool,
}

// Implement Trace for Event (skip tracing Persistent fields)
impl<'js> rquickjs::class::Trace<'js> for Event {
    fn trace<'a>(&self, _marker: rquickjs::class::Tracer<'a, 'js>) {
        // Persistent fields handle their own tracing, skip for now
    }
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
            cancel_bubble: false,
        }
    }

    #[qjs(rename = "initEvent")]
    pub fn init_event(&mut self, type_: String, bubbles: bool, cancelable: bool) {
        self.type_ = type_;
        self.bubbles = bubbles;
        self.cancelable = cancelable;
    }

    #[qjs(get)]
    pub fn target<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref t) = self.target {
            t.clone().restore(&ctx)
        } else {
            Ok(Value::new_null(ctx))
        }
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

    #[qjs(rename = "composedPath")]
    pub fn composed_path<'js>(
        &self,
        ctx: Ctx<'js>,
        _this: rquickjs::Object<'js>,
    ) -> Result<Value<'js>> {
        let array = rquickjs::Array::new(ctx.clone())?;
        if let Some(ref target) = self.target {
            array.set(0, target.clone().restore(&ctx)?)?;
        }
        Ok(array.into_value())
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
