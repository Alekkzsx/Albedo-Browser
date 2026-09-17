use crate::error::NetResult;
use crate::engine::fetcher::ResourceFetcher;
use crate::http::request::Request;
use crate::http::response::Response;
use std::time::SystemTime;

/// HSTS Auto-Upgrade Step
pub fn apply_hsts(req: &mut Request, fetcher: &ResourceFetcher, now: SystemTime) {
    if let Some(upgraded_url) = fetcher.hsts_store().upgrade_url(&req.url, now) {
        crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::Redirect, req.url.as_str(), "HSTS Upgrade");
        req.url = upgraded_url;
    }
}

/// Service Worker Interception Step
pub async fn apply_service_worker(req: &Request, fetcher: &ResourceFetcher) -> NetResult<Option<Response>> {
    let sw_hook = fetcher.service_worker_hook().read().clone();
    if let Some(sw) = sw_hook {
        if let Ok(Some(sw_response)) = sw.on_fetch(req).await {
            return Ok(Some(sw_response));
        }
    }
    Ok(None)
}

