use rquickjs::{Class, Ctx, Function, Persistent, Result, Value, Object, ArrayBuffer};
use crate::network::protocols::websocket::{WebSocketClient, WsEvent, WsCommand};
use crate::runtime::core::runtime::JsRuntime;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct WebSocket {
    #[qjs(skip_trace)]
    client: Arc<Mutex<Option<WebSocketClient>>>,
    onopen: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    onmessage: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    onerror: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    onclose: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    url: String,
    ready_state: Arc<Mutex<u16>>,
}

#[rquickjs::methods]
impl WebSocket {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, url: String) -> Result<Self> {
        let ws = WebSocket {
            client: Arc::new(Mutex::new(None)),
            onopen: Arc::new(Mutex::new(None)),
            onmessage: Arc::new(Mutex::new(None)),
            onerror: Arc::new(Mutex::new(None)),
            onclose: Arc::new(Mutex::new(None)),
            url: url.clone(),
            ready_state: Arc::new(Mutex::new(0)), // 0 = CONNECTING
        };

        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime must be in userdata").clone();
        let ws_clone = ws.clone();
        let url_str = url.clone();

        // Start connection in background
        tokio::spawn(async move {
            let id = Uuid::new_v4().to_string();
            let (client_opt, mut rx) = WebSocketClient::connect(&url_str, id);
            
            if let Some(client) = client_opt {
                *ws_clone.client.lock().unwrap() = Some(client);
                
                while let Some(event) = rx.recv().await {
                    let ws_inner = ws_clone.clone();
                    let rt_inner = rt.clone();
                    
                    // Macro-task to bridge between Tokio and JS thread
                    let mut el = rt_inner.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt_inner.with_context(|ctx| {
                            let _ = ctx.with(|ctx| {
                                match event {
                                    WsEvent::Connected => {
                                        *ws_inner.ready_state.lock().unwrap() = 1; // 1 = OPEN
                                        if let Some(ref cb) = *ws_inner.onopen.lock().unwrap() {
                                            if let Ok(func) = cb.restore(&ctx) {
                                                let _ = func.call::<(), ()>(());
                                            }
                                        }
                                    }
                                    WsEvent::Message(text) => {
                                        if let Some(ref cb) = *ws_inner.onmessage.lock().unwrap() {
                                            if let Ok(func) = cb.restore(&ctx) {
                                                let event_obj = rquickjs::Object::new(ctx.clone()).unwrap();
                                                let _ = event_obj.set("data", text);
                                                let _ = func.call::<(Object,), ()>((event_obj,));
                                            }
                                        }
                                    }
                                    WsEvent::Binary(bin) => {
                                        if let Some(ref cb) = *ws_inner.onmessage.lock().unwrap() {
                                            if let Ok(func) = cb.restore(&ctx) {
                                                let event_obj = rquickjs::Object::new(ctx.clone()).unwrap();
                                                if let Ok(ab) = ArrayBuffer::new(ctx.clone(), bin) {
                                                    let _ = event_obj.set("data", ab);
                                                    let _ = func.call::<(Object,), ()>((event_obj,));
                                                }
                                            }
                                        }
                                    }
                                    WsEvent::Error(err) => {
                                        if let Some(ref cb) = *ws_inner.onerror.lock().unwrap() {
                                            if let Ok(func) = cb.restore(&ctx) {
                                                let event_obj = rquickjs::Object::new(ctx.clone()).unwrap();
                                                let _ = event_obj.set("message", err);
                                                let _ = func.call::<(Object,), ()>((event_obj,));
                                            }
                                        }
                                    }
                                    WsEvent::Disconnected => {
                                        *ws_inner.ready_state.lock().unwrap() = 3; // 3 = CLOSED
                                        if let Some(ref cb) = *ws_inner.onclose.lock().unwrap() {
                                            if let Ok(func) = cb.restore(&ctx) {
                                                let _ = func.call::<(), ()>(());
                                            }
                                        }
                                    }
                                }
                            });
                        });
                    });
                }
            } else {
                *ws_clone.ready_state.lock().unwrap() = 3; // CLOSED
            }
        });

        Ok(ws)
    }

    pub fn send(&self, data: Value<'_>) -> Result<()> {
        if *self.ready_state.lock().unwrap() != 1 {
            return Err(rquickjs::Error::new_from_js("WebSocket is not open", "Error"));
        }

        if let Some(ref client) = *self.client.lock().unwrap() {
            if let Some(s) = data.as_string() {
                client.send_text(&s.to_string()?);
            } else if let Some(ab) = data.as_object().and_then(|obj| obj.as_array_buffer()) {
                client.send_binary(ab.as_ref().to_vec());
            } else if let Some(ta) = data.as_object().and_then(|obj| obj.as_typed_array::<u8>().ok()) {
                client.send_binary(ta.as_ref().to_vec());
            } else {
                return Err(rquickjs::Error::new_from_js("WebSocket", "Unsupported data type for send"));
            }
        }
        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        *self.ready_state.lock().unwrap() = 2; // 2 = CLOSING
        if let Some(ref client) = *self.client.lock().unwrap() {
            client.close();
        }
        Ok(())
    }

    #[qjs(get)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    #[qjs(get, rename = "readyState")]
    pub fn ready_state(&self) -> u16 {
        *self.ready_state.lock().unwrap()
    }

    #[qjs(get)]
    pub fn onopen<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onopen.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onopen(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onopen.lock().unwrap() = Some(Persistent::save(ctx, func));
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

    #[qjs(get)]
    pub fn onclose<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onclose.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onclose(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onclose.lock().unwrap() = Some(Persistent::save(ctx, func));
        Ok(())
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            globals.set("WebSocket", Class::<WebSocket>::register(ctx.clone())?)?;
            Ok(())
        })
    })
}
