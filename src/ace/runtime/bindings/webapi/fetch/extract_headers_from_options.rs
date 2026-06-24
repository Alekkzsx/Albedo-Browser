use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



pub(crate) fn extract_headers_from_options(
    options: &rquickjs::Object,
) -> (String, Option<String>, std::collections::HashMap<String, String>) {
    let method = options
        .get::<_, String>("method")
        .unwrap_or_else(|_| "GET".to_string())
        .to_uppercase();
    let body = options.get::<_, Option<String>>("body").unwrap_or(None);
    let headers_obj = options
        .get::<_, Option<rquickjs::Object>>("headers")
        .unwrap_or(None);

    let mut headers_map = std::collections::HashMap::new();
    if let Some(h_obj) = headers_obj {
        for key in h_obj.keys::<String>() {
            if let Ok(k) = key {
                if let Ok(v) = h_obj.get::<_, String>(k.clone()) {
                    headers_map.insert(k, v);
                }
            }
        }
    }
    (method, body, headers_map)
}
