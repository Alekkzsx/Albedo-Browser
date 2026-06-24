use super::*;
use crate::ace::engine::dom::AceDOM;
use crate::network::resources::ResourceManager;
use crate::shared::security::Origin;
use rquickjs::function::IntoJsFunc;
use rquickjs::{Context, Ctx, Runtime, Value};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



use super::event_loop::EventLoop;

impl JsRuntime {

    /// TODO: add docs
    pub fn run_pending(&self) -> (bool, bool) {
        super::executor::run_pending(self)
    }

    /// TODO: add docs
    pub fn run_raf_callbacks(&self, timestamp: f64) -> bool {
        let callbacks = {
            let mut event_loop = self.event_loop.lock().unwrap_or_else(|e| e.into_inner());
            event_loop.take_raf_callbacks()
        };

        if callbacks.is_empty() {
            return false;
        }

        self.with_context(|ctx| {
            ctx.with(|ctx| {
                for callback in callbacks {
                    if let Ok(func) = callback.0.restore(&ctx) {
                        let _: rquickjs::Result<rquickjs::Value> = func.call((timestamp,));
                    }
                }
            });
        });

        true
    }

    /// TODO: add docs
    pub fn run_idle_callbacks(&self, frame_deadline: std::time::Instant) -> bool {
        let callbacks = {
            let mut el = self.event_loop.lock().unwrap_or_else(|e| e.into_inner());
            el.take_idle_callbacks(frame_deadline)
        };

        if callbacks.is_empty() {
            return false;
        }

        self.with_context(|ctx| {
            ctx.with(|ctx| {
                for task in callbacks {
                    let now = std::time::Instant::now();

                    // Calcular timeRemaining em double (ms)
                    let time_remaining_ms = if frame_deadline > now {
                        frame_deadline.duration_since(now).as_secs_f64() * 1000.0
                    } else {
                        0.0
                    };

                    // Verificar se foi timeout
                    let did_timeout = task.timeout_deadline.map(|d| now >= d).unwrap_or(false);

                    // Criar um IdleDeadline faked com JS wrapper
                    let script = format!(
                        "(function(cb) {{ 
                            var deadline = {{ 
                                timeRemaining: function() {{ return {:.3}; }}, 
                                didTimeout: {} 
                            }};
                            cb(deadline);
                        }})",
                        time_remaining_ms.max(0.0),
                        did_timeout
                    );

                    if let Ok(wrapper_fn) = ctx.eval::<rquickjs::Function, _>(script) {
                        if let Ok(cb) = task.callback.0.restore(&ctx) {
                            let _: rquickjs::Result<Value> = wrapper_fn.call((cb,));
                        }
                    }
                }
            })
        });

        true
    }
}
