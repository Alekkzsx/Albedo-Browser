use super::*;
use rquickjs::Value;



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
