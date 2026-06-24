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


pub mod serviceworkerclient; pub use serviceworkerclient::*;
pub mod clients; pub use clients::*;
pub mod serviceworkerregistrationjs; pub use serviceworkerregistrationjs::*;
pub mod serviceworkercontainer; pub use serviceworkercontainer::*;
pub mod register_service_worker_container; pub use register_service_worker_container::*;
pub mod test_sw_client_creation; pub use test_sw_client_creation::*;
