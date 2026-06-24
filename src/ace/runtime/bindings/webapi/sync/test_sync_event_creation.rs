use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn test_sync_event_creation() {
        let sync_event = SyncEvent {
            tag: "upload-data".to_string(),
            last_chance: false,
            pending_promises: Arc::new(Mutex::new(Vec::new())),
        };

        assert_eq!(sync_event.tag, "upload-data");
        assert!(!sync_event.last_chance);
    }

    #[test]
pub(crate) fn test_periodic_sync_event_creation() {
        let periodic_event = PeriodicSyncEvent {
            tag: "cleanup".to_string(),
            min_interval: 24 * 60 * 60 * 1000,
            pending_promises: Arc::new(Mutex::new(Vec::new())),
        };

        assert_eq!(periodic_event.tag, "cleanup");
        assert_eq!(periodic_event.min_interval, 24 * 60 * 60 * 1000);
    }
}
