use crate::ace::engine::dom::AceDOM;
use crate::network::resources::ResourceManager;
use crate::shared::security::Origin;
use rquickjs::function::IntoJsFunc;
use rquickjs::{Context, Ctx, Runtime, Value};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};


pub mod jsresult; pub use jsresult::*;
pub mod next_runtime_id; pub use next_runtime_id::*;
pub mod historyentry; pub use historyentry::*;
pub mod resizeregistry; pub use resizeregistry::*;
pub mod intersectionregistry; pub use intersectionregistry::*;
pub mod jsruntime; pub use jsruntime::*;
pub mod jsruntime_impl_1; pub use jsruntime_impl_1::*;
pub mod jsruntime_impl_2; pub use jsruntime_impl_2::*;
pub mod jsruntime_impl_3; pub use jsruntime_impl_3::*;
pub mod jsruntime_impl_4; pub use jsruntime_impl_4::*;
pub mod jsruntime_impl_5; pub use jsruntime_impl_5::*;
