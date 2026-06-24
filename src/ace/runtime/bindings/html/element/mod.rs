use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};


pub mod element; pub use element::*;
pub mod mark_mutation; pub use mark_mutation::*;
pub mod element_impl_1; pub use element_impl_1::*;
pub mod element_impl_2; pub use element_impl_2::*;
pub mod element_impl_3; pub use element_impl_3::*;
pub mod element_impl_4; pub use element_impl_4::*;
pub mod element_impl_5; pub use element_impl_5::*;
pub mod element_impl_6; pub use element_impl_6::*;
pub mod element_impl_7; pub use element_impl_7::*;
