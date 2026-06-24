use rquickjs::Result as JsResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::utils::uuid::Uuid;

use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::sw_db::{ServiceWorkerDatabase, SwRegistrationData, SwSyncTaskData};

// ============================================================================
// ENUMS & BASIC TYPES
// ============================================================================


pub mod serviceworkerstate; pub use serviceworkerstate::*;
pub mod updateviacache; pub use updateviacache::*;
pub mod cachemode; pub use cachemode::*;
pub mod redirectmode; pub use redirectmode::*;
pub mod cachesource; pub use cachesource::*;
pub mod requestcontext; pub use requestcontext::*;
pub mod responsecontext; pub use responsecontext::*;
pub mod interceptresult; pub use interceptresult::*;
pub mod fetchinterceptor; pub use fetchinterceptor::*;
pub mod fetchinterceptorchain; pub use fetchinterceptorchain::*;
pub mod synctask; pub use synctask::*;
pub mod backgroundsyncqueue; pub use backgroundsyncqueue::*;
pub mod periodicsynctask; pub use periodicsynctask::*;
pub mod periodicsyncscheduler; pub use periodicsyncscheduler::*;
pub mod serviceworkerinstance; pub use serviceworkerinstance::*;
pub mod swevent; pub use swevent::*;
pub mod clientinfo; pub use clientinfo::*;
pub mod serviceworkerregistration; pub use serviceworkerregistration::*;
pub mod serviceworkermanager; pub use serviceworkermanager::*;
