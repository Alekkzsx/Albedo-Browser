// ARQUIVO: src/net/fetch.rs

use crate::shared::json::{self, JsonValue};
use crate::network::http3::Http3Client;
use crate::network::security::{AccessControl, Origin};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

// Erros Específicos do Fetch

pub mod fetcherror; pub use fetcherror::*;
pub mod fetchmode; pub use fetchmode::*;
pub mod fetchoptions; pub use fetchoptions::*;
pub mod fetchresponse; pub use fetchresponse::*;
pub mod fetchclient; pub use fetchclient::*;
