use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, ArrayBuffer, Class, Ctx, Function, Object, Persistent, Result, Value};
// No UnsafeSendVal needed here after synchronous refactor
use std::sync::{Arc, Mutex};


pub mod blob; pub use blob::*;
pub mod file; pub use file::*;
pub mod filereader; pub use filereader::*;
pub mod register; pub use register::*;
