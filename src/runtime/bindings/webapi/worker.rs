use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, Class, Ctx, Function, Result, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub enum WorkerMessage {
    PostMessage(String), // JSON serialized
    Terminate,
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Worker {
    #[qjs(skip_trace)]
    pub(crate) parent_rt_id: usize,
    #[qjs(skip_trace)]
    pub(crate) worker_rt_id: usize,
    #[qjs(skip_trace)]
    pub(crate) sender: Arc<Mutex<Sender<WorkerMessage>>>,
    #[qjs(skip_trace)]
    pub(crate) is_terminated: Arc<AtomicBool>,
}

#[rquickjs::methods]
impl Worker {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, url: String) -> Result<Self> {
        let rt_val = ctx.globals().get::<_, Value>("__albedo_rt__")?;
        let parent_rt = Class::<JsRuntime>::from_object(rt_val.as_object().unwrap())
            .unwrap()
            .borrow()
            .clone();
        let parent_rt_id = parent_rt.id;

        let rt = JsRuntime::new().map_err(|_| {
            rquickjs::Error::new_from_js("Worker", "Failed to create worker runtime")
        })?;
        let worker_rt_id = rt.id;

        let (tx, rx): (Sender<WorkerMessage>, Receiver<WorkerMessage>) = channel();
        let tx_arc = Arc::new(Mutex::new(tx));
        let is_terminated = Arc::new(AtomicBool::new(false));

        // Register child runtime to global registry
        crate::runtime::core::registry::register_runtime(
            worker_rt_id,
            Arc::new(Mutex::new(rt.clone())),
        );

        // Save origin if needed
        *rt.origin.lock().unwrap() = parent_rt.origin.lock().unwrap().clone();

        // Setup worker context
        let rt_clone = rt.clone();
        let parent_rt_id_captured = parent_rt_id;
        rt.with_context(move |ctx: &rquickjs::Context| {
            ctx.with(|ctx: Ctx<'_>| {
                let _ = crate::runtime::bindings::utils::console::Console::register(&rt_clone);
                let _ = crate::runtime::bindings::webapi::timers::register(&rt_clone);
                let _ = crate::runtime::core::init::register_events(&rt_clone);

                // Polyfill for EventTarget in worker scope, self and window aliases
                let _ = ctx.eval::<(), _>(
                    r#"
                    globalThis._listeners = {};
                    globalThis.addEventListener = function(type, fn) {
                        if (!globalThis._listeners[type]) globalThis._listeners[type] = [];
                        globalThis._listeners[type].push(fn);
                    };
                    globalThis.dispatchEvent = function(event) {
                        var ls = globalThis._listeners[event.type];
                        if (ls) for (var i = 0; i < ls.length; i++) ls[i](event);
                        return true;
                    };
                    if (!globalThis.onmessage) globalThis.onmessage = null;
                    if (!globalThis.onerror) globalThis.onerror = null;
                    
                    // Route MessageEvent directly to handler if registered
                    globalThis.addEventListener('message', function(e) {
                        if (typeof globalThis.onmessage === 'function') {
                            globalThis.onmessage(e);
                        }
                    });
                    
                    globalThis.self = globalThis;
                "#,
                );

                // Implement postMessage from Worker to Parent
                let post_message = Function::new(ctx.clone(), move |msg: Value<'_>| {
                    let m_ctx = msg.ctx();
                    let msg_json = match m_ctx.json_stringify(&msg) {
                        Ok(Some(s)) => s
                            .as_string()
                            .unwrap()
                            .to_string()
                            .unwrap_or_else(|_| "null".to_string()),
                        _ => "null".to_string(),
                    };

                    if let Some(parent_arc) =
                        crate::runtime::core::registry::get_runtime(parent_rt_id_captured)
                    {
                        let parent_lock = parent_arc.lock().unwrap();
                        parent_lock.event_loop.lock().unwrap().enqueue_message(
                            msg_json,
                            "worker".to_string(),
                            Some(worker_rt_id),
                        );
                    }
                })
                .unwrap();
                ctx.globals().set("postMessage", post_message).unwrap();
            });
        });

        // TODO: Resource Manager to Fetch `url` and execute script
        // For MVP, we will print a message. Proper network integration will fetch URL text.
        let _resource_manager = parent_rt.resource_manager.lock().unwrap().clone();
        let thread_rt = rt.clone();
        let thread_is_terminated = is_terminated.clone();

        thread::spawn(move || {
            // Worker run loop
            while !thread_is_terminated.load(Ordering::Relaxed) {
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        WorkerMessage::PostMessage(json) => {
                            thread_rt.dispatch_message_event(
                                json,
                                "parent".to_string(),
                                Some(parent_rt_id),
                            );
                        }
                        WorkerMessage::Terminate => {
                            break;
                        }
                    }
                }

                thread_rt.run_pending();
                thread::sleep(std::time::Duration::from_millis(5));
            }
            // Cleanup
            crate::runtime::core::registry::unregister_runtime(worker_rt_id);
        });

        // Fetch user script logic
        let url_str = url.clone();
        let worker_rt_id_fetch = worker_rt_id;
        let origin = parent_rt.origin.lock().unwrap().clone();

        thread::spawn(move || {
            // O script do Worker DEVE respeitar Same-Origin e passa pelo pipeline FetchClient (HTTP/3 + Seguranças)
            let client = crate::network::client::FetchClient::new();
            let mut opts = crate::network::client::FetchOptions::default();
            opts.mode = crate::network::client::FetchMode::SameOrigin;

            if let Ok(response) = client.fetch(&url_str, Some(opts), origin) {
                if response.ok() && !response.opaque {
                    let content = response.text();
                    if let Some(w_arc) =
                        crate::runtime::core::registry::get_runtime(worker_rt_id_fetch)
                    {
                        let w_rt = w_arc.lock().unwrap();
                        let _ = w_rt.execute_script(&content);
                    }
                } else {
                    eprintln!(
                        "🛑 [Worker] Recusado iniciar script em {} (SOP/Network bloqueou)",
                        url_str
                    );
                }
            }
        });

        Ok(Self {
            parent_rt_id,
            worker_rt_id,
            sender: tx_arc,
            is_terminated,
        })
    }

    #[qjs(rename = "postMessage")]
    pub fn post_message<'js>(&self, ctx: Ctx<'js>, msg: Value<'js>) -> Result<()> {
        let msg_json = match ctx.json_stringify(msg)? {
            Some(s) => s.as_string().unwrap().to_string()?,
            None => "null".to_string(),
        };

        let tx = self.sender.lock().unwrap();
        let _ = tx.send(WorkerMessage::PostMessage(msg_json));
        Ok(())
    }

    pub fn terminate(&self) {
        self.is_terminated.store(true, Ordering::Relaxed);
        let _ = self.sender.lock().unwrap().send(WorkerMessage::Terminate);
    }

    #[qjs(get, rename = "onmessage")]
    pub fn onmessage_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // We evaluate global onmessage in the Parent's own object map
        // but typically a Worker instance fires events directly.
        // We'll manage EventTarget logic here for simplicity or attach to prototype.
        Ok(Value::new_null(ctx))
    }

    #[qjs(set, rename = "onmessage")]
    pub fn onmessage_setter<'js>(&self, _f: Function<'js>) {
        // Need EventTarget emulation. To keep it simple, we store it in JS object.
        // QuickJS doesn't expose easy instance properties from Rust struct cleanly without properties hack.
        // It's handled in Parent's runtime loop when `message` events arrive.
    }

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&self, _f: Function<'js>) {}
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            ctx.globals()
                .set("Worker", Class::<Worker>::register(&ctx)?)?;
            Ok(())
        })
    })
}
