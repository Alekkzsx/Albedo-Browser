use super::*;
use crate::ace::runtime::bindings::webapi::sync::{PeriodicSyncManager, SyncManager};
use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::service_worker::{
    ServiceWorkerInstance, ServiceWorkerManager, ServiceWorkerRegistration,
};
use rquickjs::{Class, Ctx, Object, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SERVICE WORKER CLIENT INFO (for clients.matchAll, etc)
// ============================================================================



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn test_sw_client_creation() {
        let client = ServiceWorkerClient {
            id: "client-1".to_string(),
            url: "http://example.com".to_string(),
            frame_type: "top-level".to_string(),
            focused: true,
        };

        assert_eq!(client.id, "client-1");
        assert_eq!(client.frame_type, "top-level");
    }
}
