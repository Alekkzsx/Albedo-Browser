use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



// ============================================================================
// PUBLIC API REGISTRATION
// ============================================================================

/// TODO: add docs
pub fn register_sync_events(rt: &JsRuntime) -> JsResult<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            // Register class definitions with globals object
            let globals = ctx.globals();
            Class::<SyncEvent>::define(&globals)?;
            Class::<PeriodicSyncEvent>::define(&globals)?;
            Class::<SyncManager>::define(&globals)?;
            Class::<PeriodicSyncManager>::define(&globals)?;

            // Define global polyfill for sync support (if no SW)
            ctx.eval::<(), _>(
                r#"
                if (!globalThis.onsync) {
                    globalThis.onsync = null;
                }
                if (!globalThis.onperiodicsync) {
                    globalThis.onperiodicsync = null;
                }
                "#,
            )?;

            Ok(())
        })
    })
}
