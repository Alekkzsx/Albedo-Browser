use super::http3::Http3Client;
use super::security::{AccessControl, CookieJar, Origin};
use crate::shared::intercept::InterceptResult;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;


pub mod resourcetype; pub use resourcetype::*;
pub mod resourceresponse; pub use resourceresponse::*;
pub mod resourcemanager; pub use resourcemanager::*;
pub mod resourcemanager_impl_1; pub use resourcemanager_impl_1::*;
