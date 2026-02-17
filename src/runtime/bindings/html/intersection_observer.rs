use rquickjs::{Ctx, Result, Value, Function, Object, Persistent, Class, prelude::*};
use std::cell::RefCell;
use crate::runtime::core::runtime::JsRuntime;

#[derive(Clone)]
#[rquickjs::class]
pub struct IntersectionObserver {
    rt: JsRuntime,
    callback: Persistent<Function<'static>>,
    options: Option<Persistent<Object<'static>>>,
    // store simple target identifiers to avoid moving JS `Value` across threads/lifetimes
    targets: RefCell<Vec<usize>>,
}

#[rquickjs::methods]
impl IntersectionObserver {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, callback: Function<'js>, options: Option<Object<'js>>) -> Result<Self> {
        let rt = ctx.globals().get::<_, JsRuntime>("__albedo_rt__").expect("JsRuntime required");
        let _threshold = options.as_ref().and_then(|o| o.get::<_, f32>("threshold").ok()).unwrap_or(0.0);
        
        let _id = {
            let mut el = rt.event_loop.lock().unwrap();
            el.next_observer_id += 1;
            el.next_observer_id
        };

        let callback_persistent = Persistent::save(&ctx, callback);
        let options_persistent = options.map(|o| Persistent::save(&ctx, o));

        // Transmute the Persistent handle itself to static lifetime for storage
        let callback_stored: Persistent<Function<'static>> = unsafe { std::mem::transmute(callback_persistent) };
        let options_stored: Option<Persistent<Object<'static>>> = unsafe { std::mem::transmute(options_persistent) };

        Ok(Self {
            rt,
            callback: callback_stored,
            options: options_stored,
            targets: RefCell::new(Vec::new()),
        })
    }

    pub fn observe<'js>(&self, _target: Value<'js>) {
        // Mock implementation: simply record a placeholder target id
        let id = 0usize;
        self.targets.borrow_mut().push(id);
    }

    pub fn unobserve<'js>(&self, target: Value<'js>) {
        // Mock implementation
    }

    pub fn disconnect(&self) {
        self.targets.borrow_mut().clear();
    }

    #[qjs(get)]
    pub fn root(&self) -> Option<String> {
        None
    }

    #[qjs(get)]
    pub fn root_margin(&self) -> String {
        "0px".to_string()
    }

    #[qjs(get)]
    pub fn thresholds(&self) -> Vec<f32> {
        vec![0.0]
    }
}

impl rquickjs::class::Trace<'_> for IntersectionObserver {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, '_>) {}
}
