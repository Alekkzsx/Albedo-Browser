use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




/// TODO: add docs
pub fn run_pending(rt: &JsRuntime) -> (bool, bool) {
    let mut executed = false;

    executed |= dispatch_post_messages(rt);
    executed |= dispatch_dom_mutations(rt);
    executed |= flush_microtasks(rt);
    executed |= resolve_async_results(rt);
    executed |= dispatch_idb_events(rt);
    executed |= run_timer_tasks(rt);
    executed |= run_idle_callbacks(rt);
    executed |= dispatch_sync_events(rt);

    let stylesheet_dirty = check_stylesheet_dirty(rt);

    check_layout_observers(rt);
    check_media_query_changes(rt);

    (executed, stylesheet_dirty)
}