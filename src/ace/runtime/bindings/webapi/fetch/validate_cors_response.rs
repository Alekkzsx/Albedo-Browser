use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



pub(crate) fn validate_cors_response(
    rm: &Option<crate::network::resources::ResourceManager>,
    org: &Option<Origin>,
    url: &str,
    resp_headers: &std::collections::HashMap<String, String>,
) -> bool {
    if let Some(ref rm) = rm {
        if let Some(ref org) = org {
            return rm.access_control.lock().unwrap_or_else(|e| e.into_inner()).validate_cors(org, url, resp_headers);
        }
    }
    false
}
