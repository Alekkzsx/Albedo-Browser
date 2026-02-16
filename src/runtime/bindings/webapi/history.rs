use rquickjs::{Class, Ctx, Result, Value, Object, Function};
use crate::runtime::core::runtime::JsRuntime;
use std::sync::{Arc, Mutex};
use serde_json;

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct History {
    #[qjs(skip_trace)]
    rt: JsRuntime,
}

#[rquickjs::methods]
impl History {
    #[qjs(get)]
    pub fn length(&self) -> usize {
        self.rt.history_stack.lock().unwrap().len()
    }

    #[qjs(get)]
    pub fn state<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let stack = self.rt.history_stack.lock().unwrap();
        let index = self.rt.history_index.lock().unwrap();
        
        if *index < stack.len() {
            if let Some(json) = &stack[*index].state_json {
                // In a real app we'd use a better way than JSON parsing every time
                // but for stability it's okay here.
                return ctx.eval(format!("({})", json));
            }
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(rename = "pushState")]
    pub fn push_state(&self, ctx: Ctx<'_>, state: Value<'_>, _title: String, url: Option<String>) -> Result<()> {
        let state_json = if !state.is_null() && !state.is_undefined() {
            // Simple serialization
            let json = ctx.eval::<String, _>(&format!("JSON.stringify({:?})", state)).unwrap_or_else(|_| "null".to_string());
            Some(json)
        } else {
            None
        };

        let mut stack = self.rt.history_stack.lock().unwrap();
        let mut index = self.rt.history_index.lock().unwrap();

        // Remove forward entries
        if !stack.is_empty() {
            stack.truncate(*index + 1);
        }

        let new_url = url.unwrap_or_else(|| "".to_string()); // Real impl would use current URL if None
        stack.push(crate::runtime::core::runtime::HistoryEntry {
            url: new_url,
            state_json,
        });
        *index = stack.len() - 1;

        println!("[History] pushState: current index {}", *index);
        Ok(())
    }

    #[qjs(rename = "replaceState")]
    pub fn replace_state(&self, ctx: Ctx<'_>, state: Value<'_>, _title: String, url: Option<String>) -> Result<()> {
        let state_json = if !state.is_null() && !state.is_undefined() {
            let json = ctx.eval::<String, _>(&format!("JSON.stringify({:?})", state)).unwrap_or_else(|_| "null".to_string());
            Some(json)
        } else {
            None
        };

        let mut stack = self.rt.history_stack.lock().unwrap();
        let index = self.rt.history_index.lock().unwrap();

        if !stack.is_empty() && *index < stack.len() {
            let entry = &mut stack[*index];
            if let Some(u) = url { entry.url = u; }
            entry.state_json = state_json;
        } else {
            stack.push(crate::runtime::core::runtime::HistoryEntry {
                url: url.unwrap_or_default(),
                state_json,
            });
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
        let stack = self.rt.history_stack.lock().unwrap();
        let mut index = self.rt.history_index.lock().unwrap();
        
        let new_index = (*index as i32) + delta;
        if new_index >= 0 && new_index < (stack.len() as i32) {
            *index = new_index as usize;
            
            // Fire popstate event
            let state_val = if let Some(json) = &stack[*index].state_json {
                ctx.eval::<Value, _>(format!("({})", json)).unwrap_or(Value::new_null(ctx.clone()))
            } else {
                Value::new_null(ctx.clone())
            };
            
            self.dispatch_popstate(ctx, state_val)?;
        }
        Ok(())
    }

    fn dispatch_popstate(&self, ctx: Ctx<'_>, state: Value<'_>) -> Result<()> {
        let global = ctx.globals();
        let event_init = rquickjs::Object::new(ctx.clone())?;
        event_init.set("state", state)?;
        
        // We'll use a script to construct and dispatch for simplicity 
        // given our partial Event API
        let code = "window.dispatchEvent(new PopStateEvent('popstate', { state: arguments[0] }))";
        let _: Value = ctx.eval_with_scope(&global, code)?;
        
        Ok(())
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            // Register PopStateEvent if not exists (stub for now)
            ctx.globals().set("PopStateEvent", ctx.globals().get::<_, Function>("Event")?)?;

            let history = History { rt: rt.clone() };
            let instance = Class::instance(ctx.clone(), history)?;
            ctx.globals().set("history", instance)?;
            Ok(())
        })
    })
}
