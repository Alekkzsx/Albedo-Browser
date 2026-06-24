use super::*;
use rquickjs::Value;



#[derive(Clone)]
#[rquickjs::class]
pub struct TouchEvent {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,

    // TouchEvent specific properties - simplified for single touch
    #[qjs(get, enumerable, rename = "clientX")]
    pub client_x: f64,
    #[qjs(get, enumerable, rename = "clientY")]
    pub client_y: f64,
    #[qjs(get, enumerable, rename = "screenX")]
    pub screen_x: f64,
    #[qjs(get, enumerable, rename = "screenY")]
    pub screen_y: f64,
    #[qjs(get, enumerable, rename = "pageX")]
    pub page_x: f64,
    #[qjs(get, enumerable, rename = "pageY")]
    pub page_y: f64,
    #[qjs(get, enumerable)]
    pub identifier: i32,

    #[qjs(get, enumerable, rename = "ctrlKey")]
    pub ctrl_key: bool,
    #[qjs(get, enumerable, rename = "shiftKey")]
    pub shift_key: bool,
    #[qjs(get, enumerable, rename = "altKey")]
    pub alt_key: bool,
    #[qjs(get, enumerable, rename = "metaKey")]
    pub meta_key: bool,
}

impl<'js> rquickjs::class::Trace<'js> for TouchEvent {
pub(crate) fn trace<'a>(&self, _marker: rquickjs::class::Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl TouchEvent {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut client_x = 0.0;
        let mut client_y = 0.0;
        let mut bubbles = false;
        let mut cancelable = false;
        let mut ctrl_key = false;
        let mut shift_key = false;
        let mut alt_key = false;
        let mut meta_key = false;
        let mut identifier = 0;

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                client_x = obj.get("clientX").unwrap_or(0.0);
                client_y = obj.get("clientY").unwrap_or(0.0);
                bubbles = obj.get("bubbles").unwrap_or(false);
                cancelable = obj.get("cancelable").unwrap_or(false);
                ctrl_key = obj.get("ctrlKey").unwrap_or(false);
                shift_key = obj.get("shiftKey").unwrap_or(false);
                alt_key = obj.get("altKey").unwrap_or(false);
                meta_key = obj.get("metaKey").unwrap_or(false);
                identifier = obj.get("identifier").unwrap_or(0);
            }
        }

        Self {
            type_,
            bubbles,
            cancelable,
            client_x,
            client_y,
            screen_x: client_x,
            screen_y: client_y,
            page_x: client_x,
            page_y: client_y,
            identifier,
            ctrl_key,
            shift_key,
            alt_key,
            meta_key,
        }
    }
}
