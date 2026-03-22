use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{Ctx, Function, Object, Persistent, Result, Value};
use std::cell::RefCell;

#[derive(Clone)]
#[rquickjs::class]
pub struct ResizeObserver {
    _rt: JsRuntime,
    _callback: Persistent<Function<'static>>,
    targets: RefCell<Vec<Value<'static>>>,
}

#[rquickjs::methods]
impl ResizeObserver {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, callback: Function<'js>) -> Result<Self> {
        let rt = ctx
            .globals()
            .get::<_, JsRuntime>("__albedo_rt__")
            .expect("JsRuntime required");

        let _id = {
            let mut el = rt.event_loop.lock().unwrap();
            el.next_observer_id += 1;
            el.next_observer_id
        };

        let callback_persistent = Persistent::save(&ctx, callback);

        Ok(Self {
            _rt: rt,
            _callback: callback_persistent,
            targets: RefCell::new(Vec::new()),
        })
    }

    pub fn observe<'js>(&self, target: Value<'js>, _options: Option<Object<'js>>) {
        // Mock implementation
        self.targets
            .borrow_mut()
            .push(unsafe { std::mem::transmute(target) });
    }

    pub fn unobserve<'js>(&self, _target: Value<'js>) {
        // Mock implementation
    }

    pub fn disconnect(&self) {
        self.targets.borrow_mut().clear();
    }
}

impl rquickjs::class::Trace<'_> for ResizeObserver {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, '_>) {}
}
