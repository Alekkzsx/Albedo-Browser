use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};


pub mod jsresult; pub use jsresult::*;
pub mod init_js_for_url; pub use init_js_for_url::*;
pub mod init_sw_runtime; pub use init_sw_runtime::*;
pub mod get_origin; pub use get_origin::*;
pub mod init_stdlib; pub use init_stdlib::*;
pub mod init_storage; pub use init_storage::*;
pub mod register_events; pub use register_events::*;
