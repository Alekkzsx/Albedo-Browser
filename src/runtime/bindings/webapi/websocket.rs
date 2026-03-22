use crate::network::protocols::websocket::{WebSocketClient, WsEvent};
use crate::runtime::core::event_loop::UnsafeSendVal;
use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct WebSocket {
    #[qjs(skip_trace)]
    client: Arc<Mutex<Option<WebSocketClient>>>,
}

#[rquickjs::methods]
impl WebSocket {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, url: String) -> Result<Self> {
        let rt_val = ctx.globals().get::<_, Value>("__albedo_rt__")?;
        let rt = Class::<JsRuntime>::from_object(rt_val.as_object().unwrap())
            .unwrap()
            .borrow()
            .clone();
        let client = Arc::new(Mutex::new(None));
        let client_clone = client.clone();

        let event_loop = rt.event_loop.clone();

        let rt_send = UnsafeSendVal(rt);

        let url_parsed = url::Url::parse(&url)
            .map_err(|_| rquickjs::Error::new_from_js("URL", "Invalid WebSocket URL"))?;

        tokio::spawn(async move {
            let (ws_client_opt, mut events) =
                WebSocketClient::connect(url_parsed.as_str(), "web_socket".to_string());
            if let Some(ws_client) = ws_client_opt {
                *client_clone.lock().unwrap() = Some(ws_client);

                while let Some(event) = events.recv().await {
                    match event {
                        WsEvent::Connected => {
                            let el_clone = event_loop.clone();
                            let rt_wrapper = rt_send.clone();

                            let _ = el_clone.lock().unwrap().queue_macro_task(move || {
                                let rt = rt_wrapper.0.clone();
                                rt.with_context(|ctx: &rquickjs::Context| {
                                    ctx.with(|_ctx: Ctx<'_>| {
                                        // Fire 'open'
                                    })
                                });
                            });
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(WebSocket { client })
    }

    pub fn send(&self, data: Value<'_>) -> Result<()> {
        if let Some(ref client) = *self.client.lock().unwrap() {
            if let Some(s) = data.as_string() {
                client.send_text(&s.to_string()?);
            } else if let Some(obj) = data.as_object() {
                if let Some(ab) = obj.as_array_buffer() {
                    client.send_binary(AsRef::<[u8]>::as_ref(&ab).to_vec());
                } else if let Some(ta) = obj.as_typed_array::<u8>() {
                    client.send_binary(ta.as_bytes().unwrap_or(&[]).to_vec());
                }
            } else {
                return Err(rquickjs::Error::new_from_js(
                    "WebSocket",
                    "Unsupported data type for send",
                ));
            }
        }
        Ok(())
    }

    pub fn close(&self) {
        if let Some(client) = self.client.lock().unwrap().take() {
            client.close();
        }
    }

    #[qjs(get, rename = "onopen")]
    pub fn onopen_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onopen")]
    pub fn onopen_setter<'js>(&self, _f: Function<'js>) {}

    #[qjs(get, rename = "onmessage")]
    pub fn onmessage_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onmessage")]
    pub fn onmessage_setter<'js>(&self, _f: Function<'js>) {}

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&self, _f: Function<'js>) {}

    #[qjs(get, rename = "onclose")]
    pub fn onclose_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onclose")]
    pub fn onclose_setter<'js>(&self, _f: Function<'js>) {}
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            ctx.globals()
                .set("WebSocket", Class::<WebSocket>::register(&ctx)?)?;
            Ok(())
        })
    })
}
