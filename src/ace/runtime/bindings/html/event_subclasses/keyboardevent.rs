use super::*;
use rquickjs::Value;



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
        let mut key = String::new();
        let mut code = String::new();
        let mut bubbles = false;
        let mut ctrl_key = false;
        let mut shift_key = false;
        let mut alt_key = false;
        let mut meta_key = false;

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                key = obj.get("key").unwrap_or_default();
                code = obj.get("code").unwrap_or_default();
                bubbles = obj.get("bubbles").unwrap_or(false);
                ctrl_key = obj.get("ctrlKey").unwrap_or(false);
                shift_key = obj.get("shiftKey").unwrap_or(false);
                alt_key = obj.get("altKey").unwrap_or(false);
                meta_key = obj.get("metaKey").unwrap_or(false);
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
