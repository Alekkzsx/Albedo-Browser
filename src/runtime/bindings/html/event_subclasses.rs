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
    #[qjs(get, enumerable, rename = "screenX")]
    pub screen_x: f64,
    #[qjs(get, enumerable, rename = "screenY")]
    pub screen_y: f64,
    #[qjs(get, enumerable, rename = "pageX")]
    pub page_x: f64,
    #[qjs(get, enumerable, rename = "pageY")]
    pub page_y: f64,
    #[qjs(get, enumerable, rename = "ctrlKey")]
    pub ctrl_key: bool,
    #[qjs(get, enumerable, rename = "shiftKey")]
    pub shift_key: bool,
    #[qjs(get, enumerable, rename = "altKey")]
    pub alt_key: bool,
    #[qjs(get, enumerable, rename = "metaKey")]
    pub meta_key: bool,
    #[qjs(get, enumerable)]
    pub button: i32,
    #[qjs(get, enumerable)]
    pub buttons: i32,
    #[qjs(get, enumerable)]
    pub which: i32,
}

#[rquickjs::methods]
impl MouseEvent {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut client_x = 0.0;
        let mut client_y = 0.0;
        let mut button = 0;
        let mut bubbles = false; 
        let mut ctrl_key = false;
        let mut shift_key = false;
        let mut alt_key = false;
        let mut meta_key = false;

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                client_x = obj.get("clientX").unwrap_or(0.0);
                client_y = obj.get("clientY").unwrap_or(0.0);
                button = obj.get("button").unwrap_or(0);
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
            client_x,
            client_y,
            screen_x: client_x,
            screen_y: client_y,
            page_x: client_x,
            page_y: client_y,
            ctrl_key,
            shift_key,
            alt_key,
            meta_key,
            button,
            buttons: if button == 0 { 0 } else { 1 },
            which: button + 1,
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

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct PointerEvent {
    #[qjs(get, enumerable, rename = "type")]
    pub type_: String,
    #[qjs(get, enumerable)]
    pub bubbles: bool,
    #[qjs(get, enumerable)]
    pub cancelable: bool,
    
    // MouseEvent properties
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
    #[qjs(get, enumerable, rename = "ctrlKey")]
    pub ctrl_key: bool,
    #[qjs(get, enumerable, rename = "shiftKey")]
    pub shift_key: bool,
    #[qjs(get, enumerable, rename = "altKey")]
    pub alt_key: bool,
    #[qjs(get, enumerable, rename = "metaKey")]
    pub meta_key: bool,
    #[qjs(get, enumerable)]
    pub button: i32,
    #[qjs(get, enumerable)]
    pub buttons: i32,
    #[qjs(get, enumerable)]
    pub which: i32,

    // PointerEvent specific properties
    #[qjs(get, enumerable, rename = "pointerId")]
    pub pointer_id: i32,
    #[qjs(get, enumerable)]
    pub width: f64,
    #[qjs(get, enumerable)]
    pub height: f64,
    #[qjs(get, enumerable)]
    pub pressure: f64,
    #[qjs(get, enumerable, rename = "tangentialPressure")]
    pub tangential_pressure: f64,
    #[qjs(get, enumerable, rename = "tiltX")]
    pub tilt_x: i32,
    #[qjs(get, enumerable, rename = "tiltY")]
    pub tilt_y: i32,
    #[qjs(get, enumerable)]
    pub twist: i32,
    #[qjs(get, enumerable, rename = "pointerType")]
    pub pointer_type: String,
    #[qjs(get, enumerable, rename = "isPrimary")]
    pub is_primary: bool,
}

#[rquickjs::methods]
impl PointerEvent {
    #[qjs(constructor)]
    pub fn new<'js>(type_: String, options: Option<Value<'js>>) -> Self {
        let mut client_x = 0.0;
        let mut client_y = 0.0;
        let mut button = 0;
        let mut bubbles = false; 
        let mut cancelable = false;
        let mut ctrl_key = false;
        let mut shift_key = false;
        let mut alt_key = false;
        let mut meta_key = false;
        
        let mut pointer_id = 0;
        let mut width = 1.0;
        let mut height = 1.0;
        let mut pressure = 0.0;
        let mut tangential_pressure = 0.0;
        let mut tilt_x = 0;
        let mut tilt_y = 0;
        let mut twist = 0;
        let mut pointer_type = "mouse".to_string();
        let mut is_primary = false;

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                client_x = obj.get("clientX").unwrap_or(0.0);
                client_y = obj.get("clientY").unwrap_or(0.0);
                button = obj.get("button").unwrap_or(0);
                bubbles = obj.get("bubbles").unwrap_or(false);
                cancelable = obj.get("cancelable").unwrap_or(false);
                ctrl_key = obj.get("ctrlKey").unwrap_or(false);
                shift_key = obj.get("shiftKey").unwrap_or(false);
                alt_key = obj.get("altKey").unwrap_or(false);
                meta_key = obj.get("metaKey").unwrap_or(false);

                pointer_id = obj.get("pointerId").unwrap_or(0);
                width = obj.get("width").unwrap_or(1.0);
                height = obj.get("height").unwrap_or(1.0);
                pressure = obj.get("pressure").unwrap_or(0.0);
                tangential_pressure = obj.get("tangentialPressure").unwrap_or(0.0);
                tilt_x = obj.get("tiltX").unwrap_or(0);
                tilt_y = obj.get("tiltY").unwrap_or(0);
                twist = obj.get("twist").unwrap_or(0);
                pointer_type = obj.get("pointerType").unwrap_or_else(|_| "mouse".to_string());
                is_primary = obj.get("isPrimary").unwrap_or(false);
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
            ctrl_key,
            shift_key,
            alt_key,
            meta_key,
            button,
            buttons: if button == 0 { 0 } else { 1 },
            which: button + 1,

            pointer_id,
            width,
            height,
            pressure,
            tangential_pressure,
            tilt_x,
            tilt_y,
            twist,
            pointer_type,
            is_primary,
        }
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Touch {
    #[qjs(get, enumerable)]
    pub identifier: i32,
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
    #[qjs(get, enumerable, rename = "radiusX")]
    pub radius_x: f64,
    #[qjs(get, enumerable, rename = "radiusY")]
    pub radius_y: f64,
    #[qjs(get, enumerable, rename = "rotationAngle")]
    pub rotation_angle: f64,
    #[qjs(get, enumerable)]
    pub force: f64,
}

#[rquickjs::methods]
impl Touch {
    #[qjs(constructor)]
    pub fn new<'js>(options: Option<Value<'js>>) -> Self {
        let mut identifier = 0;
        let mut client_x = 0.0;
        let mut client_y = 0.0;
        let mut radius_x = 0.0;
        let mut radius_y = 0.0;
        let mut rotation_angle = 0.0;
        let mut force = 0.0;

        if let Some(opts) = options {
            if let Some(obj) = opts.as_object() {
                identifier = obj.get("identifier").unwrap_or(0);
                client_x = obj.get("clientX").unwrap_or(0.0);
                client_y = obj.get("clientY").unwrap_or(0.0);
                radius_x = obj.get("radiusX").unwrap_or(0.0);
                radius_y = obj.get("radiusY").unwrap_or(0.0);
                rotation_angle = obj.get("rotationAngle").unwrap_or(0.0);
                force = obj.get("force").unwrap_or(0.0);
            }
        }

        Self {
            identifier,
            client_x,
            client_y,
            screen_x: client_x,
            screen_y: client_y,
            page_x: client_x,
            page_y: client_y,
            radius_x,
            radius_y,
            rotation_angle,
            force,
        }
    }
}

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
    fn trace<'a>(&self, _marker: rquickjs::class::Tracer<'a, 'js>) {}
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
