use super::*;
use super::http3::Http3Client;
use super::security::{AccessControl, CookieJar, Origin};
use crate::shared::intercept::InterceptResult;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;


#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Html,
    Css,
    Image,
}
