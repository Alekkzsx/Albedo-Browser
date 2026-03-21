use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct History {
    #[qjs(skip_trace)]
    pub rt: JsRuntime,
}

#[rquickjs::methods]
impl History {
    #[qjs(get)]
    pub fn length(&self) -> usize {
        self.rt.history_stack.lock().unwrap().len()
    }

    #[qjs(rename = "pushState")]
    pub fn push_state(
        &self,
        ctx: Ctx<'_>,
        _state: Value<'_>,
        _title: String,
        url: Option<String>,
    ) -> Result<()> {
        let json: String = ctx
            .eval(format!("JSON.stringify(arguments[0])").as_bytes())
            .unwrap_or_else(|_| "null".to_string());
        let mut stack = self.rt.history_stack.lock().unwrap();
        let mut index = self.rt.history_index.lock().unwrap();

        stack.truncate(*index + 1);
        stack.push(crate::runtime::core::runtime::HistoryEntry {
            url: url.unwrap_or_default(),
            state_json: Some(json),
            referrer: "".into(),
        });
        *index = stack.len() - 1;

        Ok(())
    }

    #[qjs(rename = "replaceState")]
    pub fn replace_state(
        &self,
        ctx: Ctx<'_>,
        _state: Value<'_>,
        _title: String,
        url: Option<String>,
    ) -> Result<()> {
        let json: String = ctx
            .eval(format!("JSON.stringify(arguments[0])").as_bytes())
            .unwrap_or_else(|_| "null".to_string());
        let mut stack = self.rt.history_stack.lock().unwrap();
        let index = self.rt.history_index.lock().unwrap();

        if let Some(entry) = stack.get_mut(*index) {
            entry.url = url.unwrap_or(entry.url.clone());
            entry.state_json = Some(json);
        }

        Ok(())
    }

    pub fn back(&self, ctx: Ctx<'_>) -> Result<()> {
        self.go(ctx, -1)
    }

    pub fn forward(&self, ctx: Ctx<'_>) -> Result<()> {
        self.go(ctx, 1)
    }

    pub fn go(&self, ctx: Ctx<'_>, delta: i32) -> Result<()> {
        let mut index = self.rt.history_index.lock().unwrap();
        let stack = self.rt.history_stack.lock().unwrap();

        let new_index = (*index as i32 + delta).max(0).min(stack.len() as i32 - 1) as usize;
        if new_index != *index {
            *index = new_index;
            let entry = &stack[new_index];
            if let Some(ref json) = entry.state_json {
                let state: Value = ctx
                    .eval(format!("({})", json).as_bytes())
                    .unwrap_or(Value::new_null(ctx.clone()));
                dispatch_popstate(&ctx, state)?;
            }
        }
        Ok(())
    }
}

fn dispatch_popstate<'js>(ctx: &Ctx<'js>, state: Value<'js>) -> Result<()> {
    let global = ctx.globals();
    if let Ok(window) = global.get::<_, Object>("window") {
        if let Ok(dispatch) = window.get::<_, Function>("dispatchEvent") {
            if let Ok(popstate_event_obj) = global.get::<_, Object>("PopStateEvent") {
                let event_init = rquickjs::Object::new(ctx.clone())?;
                event_init.set("state", state.clone())?;

                // Get constructor as function
                if let Some(_ctor) = popstate_event_obj.as_function() {
                    let script = format!(
                        "new PopStateEvent('popstate', {{ state: {} }})",
                        state
                            .as_string()
                            .unwrap_or(&rquickjs::String::from_str(ctx.clone(), "null").unwrap())
                            .to_string()
                            .unwrap_or("null".to_string())
                    );
                    if let Ok(event) = ctx.eval::<Value, _>(script) {
                        let _ = dispatch.call::<(Value,), ()>((event,));
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct PopStateEvent {
    #[qjs(skip_trace)]
    pub state: rquickjs::Persistent<Value<'static>>,
}

#[rquickjs::methods]
impl PopStateEvent {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, _type: String, init: Option<Object<'js>>) -> Result<Self> {
        let state = if let Some(init_obj) = init {
            init_obj
                .get("state")
                .unwrap_or(Value::new_null(ctx.clone()))
        } else {
            Value::new_null(ctx.clone())
        };
        Ok(Self {
            state: rquickjs::Persistent::save(&ctx, state),
        })
    }

    #[qjs(get)]
    pub fn state<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.state.clone().restore(&ctx)
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            if !ctx.globals().contains_key("PopStateEvent")? {
                Class::<PopStateEvent>::register(&ctx)?;
                ctx.globals()
                    .set("PopStateEvent", Class::<PopStateEvent>::register(&ctx)?)?;
            }

            let history = History { rt: rt.clone() };
            let instance = Class::instance(ctx.clone(), history)?;
            ctx.globals().set("history", instance)?;
            Ok(())
        })
    })
}
