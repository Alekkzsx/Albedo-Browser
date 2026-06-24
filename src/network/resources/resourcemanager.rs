use super::*;
use super::http3::Http3Client;
use super::security::{AccessControl, CookieJar, Origin};
use crate::shared::intercept::InterceptResult;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;



#[derive(Clone)]
pub struct ResourceManager {
    pub client: Client,
    pub http3_client: Option<Http3Client>,
    pub tx: mpsc::UnboundedSender<ResourceResponse>,
    pub cookie_jar: Arc<Mutex<CookieJar>>,
    pub access_control: Arc<Mutex<AccessControl>>,
    pub response_cache: Arc<Mutex<std::collections::HashMap<String, ResourceResponse>>>,
    pub sw_manager: Arc<crate::ace::runtime::core::service_worker::ServiceWorkerManager>,
}
