use rquickjs::{Class, Ctx, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct MouseEvent {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,
    #[qjs(get, enumerable, rename = "clientX")]
    pub client_x: f64,
    #[qjs(get, enumerable, rename = "clientY")]
    pub client_y: f64,
    #[qjs(get, enumerable)]
    pub button: i32,
}

#[rquickjs::methods]
impl MouseEvent {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut client_x = 0.0;
        let mut client_y = 0.0;
        let mut button = 0;
        let mut bubbles = false; 

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                client_x = obj.get("clientX").unwrap_or(0.0);
                client_y = obj.get("clientY").unwrap_or(0.0);
                button = obj.get("button").unwrap_or(0);
                bubbles = obj.get("bubbles").unwrap_or(false);
            }
        }

        Self {
            type_,
            bubbles,
            cancelable: false,
            client_x,
            client_y,
            button,
        }
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct KeyboardEvent {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,
    #[qjs(get, enumerable)]
    pub key: String,
    #[qjs(get, enumerable)]
    pub code: String,
    #[qjs(get, enumerable, rename = "ctrlKey")]
    pub ctrl_key: bool,
    #[qjs(get, enumerable, rename = "shiftKey")]
    pub shift_key: bool,
    #[qjs(get, enumerable, rename = "altKey")]
    pub alt_key: bool,
    #[qjs(get, enumerable, rename = "metaKey")]
    pub meta_key: bool,
}

#[rquickjs::methods]
impl KeyboardEvent {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut key = "".to_string();
        let mut code = "".to_string();
        let mut ctrl_key = false;
        let mut shift_key = false;
        let mut alt_key = false;
        let mut meta_key = false;
        let mut bubbles = false;

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                key = obj.get("key").unwrap_or("".to_string());
                code = obj.get("code").unwrap_or("".to_string());
                ctrl_key = obj.get("ctrlKey").unwrap_or(false);
                shift_key = obj.get("shiftKey").unwrap_or(false);
                alt_key = obj.get("altKey").unwrap_or(false);
                meta_key = obj.get("metaKey").unwrap_or(false);
                bubbles = obj.get("bubbles").unwrap_or(false);
            }
        }

        Self {
            type_,
            bubbles,
            cancelable: false,
            key,
            code,
            ctrl_key,
            shift_key,
            alt_key,
            meta_key,
        }
    }
}
