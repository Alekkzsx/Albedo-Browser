use rquickjs::{Class, Ctx, Function, Persistent, Result, Value, Object, Rest};
use crate::js::JsRuntime;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use serde_json;

pub enum WorkerCommand {
    Message(String),
    Error(String),
    Terminate,
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct WorkerLocation {
    pub href: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
    pub origin: String,
}

#[rquickjs::methods]
impl WorkerLocation {
    #[qjs(get)]
    pub fn href(&self) -> String { self.href.clone() }
    #[qjs(get)]
    pub fn protocol(&self) -> String { self.protocol.clone() }
    #[qjs(get)]
    pub fn host(&self) -> String { self.host.clone() }
    #[qjs(get)]
    pub fn hostname(&self) -> String { self.hostname.clone() }
    #[qjs(get)]
    pub fn port(&self) -> String { self.port.clone() }
    #[qjs(get)]
    pub fn pathname(&self) -> String { self.pathname.clone() }
    #[qjs(get)]
    pub fn search(&self) -> String { self.search.clone() }
    #[qjs(get)]
    pub fn hash(&self) -> String { self.hash.clone() }
    #[qjs(get)]
    pub fn origin(&self) -> String { self.origin.clone() }
}

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct Worker {
    #[qjs(skip_trace)]
    tx: Arc<mpsc::Sender<WorkerCommand>>,
    onmessage: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    onerror: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl Worker {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, script_url: String) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<WorkerCommand>(100);
        let tx_arc = Arc::new(tx);
        
        let worker = Worker {
            tx: tx_arc.clone(),
            onmessage: Arc::new(Mutex::new(None)),
            onerror: Arc::new(Mutex::new(None)),
        };

        let worker_clone = worker.clone();
        let main_rt = ctx.userdata::<JsRuntime>().expect("JsRuntime must be in userdata").clone();
        
        // Spawn a real OS thread for the worker
        std::thread::spawn(move || {
            let rt = match JsRuntime::new() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[Worker] Failed to create runtime: {}", e);
                    return;
                }
            };

            // Initialize worker globals
            let worker_url_for_location = script_url.clone();
            let _ = rt.with_context(|ctx| {
                ctx.with(|ctx| {
                    let global = ctx.globals();
                    
                    // self alias
                    let _ = global.set("self", global.clone());

                    // location
                    if let Ok(url) = url::Url::parse(&worker_url_for_location) {
                        let loc = WorkerLocation {
                            href: url.to_string(),
                            protocol: format!("{}:", url.scheme()),
                            host: url.host_str().unwrap_or("").to_string() + &url.port().map(|p| format!(":{}", p)).unwrap_or_default(),
                            hostname: url.host_str().unwrap_or("").to_string(),
                            port: url.port().map(|p| p.to_string()).unwrap_or_default(),
                            pathname: url.path().to_string(),
                            search: url.query().map(|q| format!("?{}", q)).unwrap_or_default(),
                            hash: url.fragment().map(|f| format!("#{}", f)).unwrap_or_default(),
                            origin: format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")),
                        };
                        let _ = global.set("location", Class::<WorkerLocation>::instance(ctx.clone(), loc).unwrap());
                    }
                    
                    // importScripts
                    let import_scripts = Function::new(ctx.clone(), move |ctx: Ctx<'_>, urls: rquickjs::Rest<String>| -> Result<()> {
                        for url_str in urls.0 {
                            let content = if url_str.starts_with("http") {
                                reqwest::blocking::get(&url_str).and_then(|r| r.text()).map_err(|e| rquickjs::Error::new_from_js("NetworkError", &e.to_string()))?
                            } else {
                                std::fs::read_to_string(&url_str).map_err(|e| rquickjs::Error::new_from_js("FileError", &e.to_string()))?
                            };
                            let _ = ctx.eval::<Value, _>(&content)?;
                        }
                        Ok(())
                    }).unwrap();
                    let _ = global.set("importScripts", import_scripts);

                    // postMessage for worker -> main
                    let main_rt_inner = main_rt.clone();
                    let worker_inner = worker_clone.clone();
                    let post_message = rquickjs::Function::new(ctx.clone(), move |ctx: Ctx<'_>, val: Value<'_>| {
                        // Use JSON for "structured clone" approximation
                        let json_obj = ctx.globals().get::<_, rquickjs::Object>("JSON").unwrap();
                        let stringify = json_obj.get::<_, rquickjs::Function>("stringify").unwrap();
                        
                        if let Ok(json_str) = stringify.call::<_, String>((val,)) {
                            let main_rt_nested = main_rt_inner.clone();
                            let worker_nested = worker_inner.clone();
                            
                            let mut el = main_rt_nested.event_loop.lock().unwrap();
                            el.queue_macro_task(move || {
                                main_rt_nested.with_context(|ctx| {
                                    ctx.with(|ctx| {
                                        if let Some(ref cb) = *worker_nested.onmessage.lock().unwrap() {
                                            if let Ok(func) = cb.restore(&ctx) {
                                                let json_obj = ctx.globals().get::<_, rquickjs::Object>("JSON").unwrap();
                                                let parse = json_obj.get::<_, rquickjs::Function>("parse").unwrap();
                                                
                                                if let Ok(data) = parse.call::<_, Value>((json_str,)) {
                                                    let event = rquickjs::Object::new(ctx.clone()).unwrap();
                                                    let _ = event.set("data", data);
                                                    let _ = func.call::<(Object,), ()>((event,));
                                                }
                                            }
                                        }
                                    });
                                });
                            });
                        }
                    }).unwrap();
                    let _ = global.set("postMessage", post_message);
                    
                    // Basic APIs
                    let _ = crate::js::console::Console::register_in_ctx(&ctx);
                    let _ = crate::js::bindings::timers::register_in_rt(&rt, &ctx);
                    let _ = crate::js::bindings::fetch::register_in_rt(&rt, &ctx);
                });
            });

            // Helper to send error back to main thread
            let main_rt_error = main_rt.clone();
            let worker_error = worker_clone.clone();
            let send_error = move |err: String| {
                let main_rt_nested = main_rt_error.clone();
                let worker_nested = worker_error.clone();
                let mut el = main_rt_nested.event_loop.lock().unwrap();
                el.queue_macro_task(move || {
                    main_rt_nested.with_context(|ctx| {
                        ctx.with(|ctx| {
                            if let Some(ref cb) = *worker_nested.onerror.lock().unwrap() {
                                if let Ok(func) = cb.restore(&ctx) {
                                    let event = rquickjs::Object::new(ctx.clone()).unwrap();
                                    let _ = event.set("message", err);
                                    let _ = func.call::<(Object,), ()>((event,));
                                }
                            }
                        });
                    });
                });
            };

            // Load and execute script
            let script_content = if script_url.starts_with("http") {
                reqwest::blocking::get(&script_url).and_then(|r| r.text()).unwrap_or_else(|_| "".to_string())
            } else {
                std::fs::read_to_string(&script_url).unwrap_or_else(|_| "".to_string())
            };

            if let Err(e) = rt.execute_script(&script_content) {
                send_error(e.to_string());
            }

            // Worker Event Loop
            loop {
                let mut activity = false;

                // 1. Run Worker's microtasks and timers
                let (executed, _) = rt.run_pending();
                if executed { activity = true; }

                // 2. Process incoming commands from main thread
                while let Ok(cmd) = rx.try_recv() {
                    activity = true;
                    match cmd {
                        WorkerCommand::Message(json_str) => {
                            let _ = rt.with_context(|ctx| {
                                ctx.with(|ctx| {
                                    let global = ctx.globals();
                                    if let Ok(onmsg) = global.get::<_, rquickjs::Function>("onmessage") {
                                        let json_obj = ctx.globals().get::<_, rquickjs::Object>("JSON").unwrap();
                                        let parse = json_obj.get::<_, rquickjs::Function>("parse").unwrap();
                                        
                                        if let Ok(data) = parse.call::<_, Value>((json_str,)) {
                                            let event = rquickjs::Object::new(ctx.clone()).unwrap();
                                            let _ = event.set("data", data);
                                            if let Err(e) = onmsg.call::<(Object,), ()>((event,)) {
                                                send_error(format!("[onmessage] {}", e));
                                            }
                                        }
                                    }
                                });
                            });
                        }
                        WorkerCommand::Error(err) => {
                             send_error(err);
                        }
                        WorkerCommand::Terminate => return,
                    }
                }

                if !activity {
                    // Reduce CPU usage when idle
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        Ok(worker)
    }

    pub fn postMessage(&self, ctx: Ctx<'_>, message: Value<'_>) -> Result<()> {
        let json_obj = ctx.globals().get::<_, rquickjs::Object>("JSON")?;
        let stringify = json_obj.get::<_, rquickjs::Function>("stringify")?;
        if let Ok(json_str) = stringify.call::<_, String>((message,)) {
            let _ = self.tx.try_send(WorkerCommand::Message(json_str));
        }
        Ok(())
    }

    pub fn terminate(&self) -> Result<()> {
        let _ = self.tx.try_send(WorkerCommand::Terminate);
        Ok(())
    }

    #[qjs(get)]
    pub fn onmessage<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onmessage.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onmessage(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onmessage.lock().unwrap() = Some(Persistent::save(ctx, func));
        Ok(())
    }

    #[qjs(get)]
    pub fn onerror<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onerror.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onerror(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onerror.lock().unwrap() = Some(Persistent::save(ctx, func));
        Ok(())
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            globals.set("Worker", Class::<Worker>::register(ctx.clone())?)?;
            Class::<WorkerLocation>::register(ctx.clone())?;
            Ok(())
        })
    })
}
