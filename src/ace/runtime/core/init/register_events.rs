use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn register_events(rt: &JsRuntime) -> JsResult<()> {
    let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
    ctx.with(|ctx: rquickjs::Ctx| {
        let global = ctx.globals();

        // Register base Event
        use crate::ace::runtime::bindings::html::event::Event;
        rquickjs::Class::<Event>::define(&global)?;

        // Register subclasses
        use crate::ace::runtime::bindings::html::event_subclasses::{
            KeyboardEvent, MessageEvent, MouseEvent, PointerEvent, TouchEvent,
        };
        rquickjs::Class::<MouseEvent>::define(&global)?;
        rquickjs::Class::<PointerEvent>::define(&global)?;
        rquickjs::Class::<TouchEvent>::define(&global)?;
        rquickjs::Class::<KeyboardEvent>::define(&global)?;
        rquickjs::Class::<MessageEvent>::define(&global)?;

        // Setup prototype chain (basic inheritance simulation)

        let event_ctor: rquickjs::Function = global.get("Event")?;
        let mouse_ctor: rquickjs::Function = global.get("MouseEvent")?;
        let pointer_ctor: rquickjs::Function = global.get("PointerEvent")?;
        let touch_ctor: rquickjs::Function = global.get("TouchEvent")?;
        let kbd_ctor: rquickjs::Function = global.get("KeyboardEvent")?;

        let event_proto: rquickjs::Object = event_ctor.get("prototype")?;
        let mouse_proto: rquickjs::Object = mouse_ctor.get("prototype")?;
        let pointer_proto: rquickjs::Object = pointer_ctor.get("prototype")?;
        let touch_proto: rquickjs::Object = touch_ctor.get("prototype")?;
        let kbd_proto: rquickjs::Object = kbd_ctor.get("prototype")?;
        let msg_ctor: rquickjs::Function = global.get("MessageEvent")?;
        let msg_proto: rquickjs::Object = msg_ctor.get("prototype")?;

        mouse_proto.set_prototype(Some(&event_proto))?;
        pointer_proto.set_prototype(Some(&mouse_proto))?; // PointerEvent herda de MouseEvent (que herda de Event)
        touch_proto.set_prototype(Some(&event_proto))?;
        kbd_proto.set_prototype(Some(&event_proto))?;
        msg_proto.set_prototype(Some(&event_proto))?;

        Ok(())
    })
}
