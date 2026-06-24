use super::*;
use rquickjs::Value;



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct MessageEvent {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,
    #[qjs(get, enumerable)]
    pub data: String, // Stringified for simplicity across runtimes right now
    #[qjs(get, enumerable)]
    pub origin: String,
    #[qjs(get, enumerable, rename = "lastEventId")]
    pub last_event_id: String,
    // Note: source and ports are complex cross-context objects, skipping for MVP
}

#[rquickjs::methods]
impl MessageEvent {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut data = String::new();
        let mut origin = String::new();
        let mut bubbles = false;
        let mut last_event_id = String::new();

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                if let Ok(data_val) = obj.get::<_, String>("data") {
                    data = data_val;
                }
                origin = obj.get("origin").unwrap_or_default();
                bubbles = obj.get("bubbles").unwrap_or(false);
                last_event_id = obj.get("lastEventId").unwrap_or_default();
            }
        }

        Self {
            type_,
            bubbles,
            cancelable: false,
            data,
            origin,
            last_event_id,
        }
    }
}
