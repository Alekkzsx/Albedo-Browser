use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};


static NEXT_REQ_ID: AtomicUsize = AtomicUsize::new(1000);
