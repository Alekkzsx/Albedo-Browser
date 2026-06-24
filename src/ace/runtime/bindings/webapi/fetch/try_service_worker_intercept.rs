use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



use crate::shared::security::Origin;
use crate::ace::runtime::core::runtime::JsRuntime;

pub(crate) fn try_service_worker_intercept(
    sw_manager: &std::sync::Arc<std::sync::Mutex<crate::ace::runtime::core::service_worker::ServiceWorkerManager>>,
    origin_str: &str,
    url: &str,
    method: &str,
    headers: &std::collections::HashMap<String, String>,
) -> Option<crate::ace::runtime::core::service_worker::ResponseContext> {
    if let Ok(Some(reg)) = sw_manager.lock().unwrap_or_else(|e| e.into_inner()).find_for_url(origin_str, url) {
        if let Ok(Some(active)) = reg.get_active() {
            let req_ctx = RequestContext {
                method: method.to_string(),
                url: url.to_string(),
                headers: headers.clone(),
                body: None,
                mode: "cors".to_string(),
                credentials: "omit".to_string(),
                cache_mode: CacheMode::Default,
                redirect: RedirectMode::Follow,
            };

            if let Ok(InterceptResult::Handled(sw_resp)) = sw_manager.lock().unwrap_or_else(|e| e.into_inner()).dispatch_fetch_event(&active, req_ctx) {
                return Some(sw_resp);
            }
        }
    }
    None
}
