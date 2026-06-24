use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn init_storage(rt: &JsRuntime, origin: &str) -> JsResult<()> {
    let storage_dir = if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".local/share/albedo/storage")
    } else {
        std::path::PathBuf::from("./storage")
    };

    // Sanitize origin for filename
    let sanitized_origin = origin
        .replace("://", "_")
        .replace(".", "_")
        .replace("/", "_")
        .replace(":", "_");

    let local_storage_path = storage_dir.join(format!("{}.json", sanitized_origin));

    use crate::ace::runtime::bindings::webapi::storage::Storage;

    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            Storage::register(&ctx, "localStorage", Storage::new_local(local_storage_path))?;
            Storage::register(&ctx, "sessionStorage", Storage::new_session())?;
            Ok(())
        })
    })
}
