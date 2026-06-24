use super::collection::TabCollection;
use super::tab::TabMode;
use crate::ace::engine::AceEngine;
use crate::network::resources::ResourceManager;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;


pub mod tabmanager; pub use tabmanager::*;
pub mod tabmanager_impl_1; pub use tabmanager_impl_1::*;
pub mod tabmanager_impl_2; pub use tabmanager_impl_2::*;
pub mod tabmanager_impl_3; pub use tabmanager_impl_3::*;
pub mod tabmanager_impl_4; pub use tabmanager_impl_4::*;
pub mod tabmanager_impl_5; pub use tabmanager_impl_5::*;
pub mod tabmanager_impl_6; pub use tabmanager_impl_6::*;
pub mod tabmanager_impl_7; pub use tabmanager_impl_7::*;
