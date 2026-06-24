use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn dispatch_post_messages(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let messages: std::collections::VecDeque<_> =
        { rt.event_loop.lock().unwrap_or_else(|e| e.into_inner()).take_pending_messages() };
    if !messages.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for msg in messages {
                    let safe_data = msg.data_json.replace('\'', "\\'");
                    let safe_origin = msg.origin.replace('\'', "\\'");
                    let script = format!(
                        "globalThis.dispatchEvent(new MessageEvent('message', {{ data: {}, origin: '{}' }}))",
                        safe_data, safe_origin
                    );
                    let _ = ctx.eval::<(), _>(script);
                    executed = true;
                }
            })
        });
    }
    executed
}
