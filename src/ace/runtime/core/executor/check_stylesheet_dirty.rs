use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn check_stylesheet_dirty(rt: &JsRuntime) -> bool {
    if let Ok(mut sd) = rt.stylesheet_dirty.lock() {
        if *sd {
            *sd = false;
            return true;
        }
    }
    false
}
