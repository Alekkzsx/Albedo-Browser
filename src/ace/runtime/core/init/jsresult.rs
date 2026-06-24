use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};


pub(crate) type JsResult<T> = Result<T, rquickjs::Error>;
