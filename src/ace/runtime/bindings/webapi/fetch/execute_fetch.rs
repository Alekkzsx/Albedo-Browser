use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



pub(crate) async fn execute_fetch(
    url: String,
    method: String,
    body: Option<String>,
    headers_map: std::collections::HashMap<String, String>,
    id: u32,
    sender: tokio::sync::mpsc::UnboundedSender<AsyncResult>,
    origin_arc: std::sync::Arc<std::sync::Mutex<Option<Origin>>>,
    resource_manager: std::sync::Arc<std::sync::Mutex<Option<crate::network::resources::ResourceManager>>>,
    sw_manager: std::sync::Arc<std::sync::Mutex<crate::ace::runtime::core::service_worker::ServiceWorkerManager>>,
) {
    let origin_str = {
        let lock = origin_arc.lock().unwrap_or_else(|e| e.into_inner());
        lock.as_ref()
            .map(|o| o.to_string())
            .unwrap_or_else(|| "null".to_string())
    };

    if let Some(sw_resp) = try_service_worker_intercept(
        &sw_manager, &origin_str, &url, &method, &headers_map,
    ) {
        let _ = sender.send(AsyncResult {
            id,
            result: Ok((
                sw_resp.status,
                String::from_utf8_lossy(&sw_resp.body).to_string(),
            )),
        });
        return;
    }

    let (rm_opt, org_opt) = {
        let rm_lock = resource_manager.lock().unwrap_or_else(|e| e.into_inner());
        let org_lock = origin_arc.lock().unwrap_or_else(|e| e.into_inner());
        ((*rm_lock).clone(), (*org_lock).clone())
    };

    let client = if let Some(ref rm) = rm_opt {
        rm.client.clone()
    } else {
        reqwest::Client::new()
    };

    let target_origin = Origin::from_url(&url);
    let is_cross_origin = match (&org_opt, &target_origin) {
        (Some(o), Some(t)) => !o.is_same_origin(t),
        _ => true,
    };

    let mut req_builder = match method.as_str() {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        _ => client.get(&url),
    };

    if let Some(ref rm) = rm_opt {
        let cookies = rm.cookie_jar.lock().unwrap_or_else(|e| e.into_inner()).get_cookies_for_url(&url);
        if !cookies.is_empty() {
            req_builder = req_builder.header("Cookie", cookies);
        }
    }

    for (k, v) in headers_map {
        req_builder = req_builder.header(k, v);
    }

    if let Some(b) = body {
        req_builder = req_builder.body(b);
    }

    let response_result = req_builder.send().await;

    let final_result: std::result::Result<(u16, String), String> =
        match response_result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let mut resp_headers = std::collections::HashMap::new();
                for (k, v) in resp.headers().iter() {
                    resp_headers.insert(
                        k.to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    );
                }

                if is_cross_origin {
                    let allowed = validate_cors_response(&rm_opt, &org_opt, &url, &resp_headers);

                    if !allowed {
                        let _ = sender.send(AsyncResult {
                            id,
                            result: Err(
                                "CORS Error: Origin not allowed".to_string()
                            ),
                        });
                        return;
                    }
                }

                if let Some(ref rm) = rm_opt {
                    if let Some(cookie_header) =
                        resp.headers().get("set-cookie")
                    {
                        if let Ok(c_str) = cookie_header.to_str() {
                            rm.cookie_jar
                                .lock().unwrap_or_else(|e| e.into_inner())
                                .set_cookie(&url, c_str);
                        }
                    }
                }

                let body = resp.text().await.unwrap_or_default();
                Ok((status, body))
            }
            Err(e) => Err(e.to_string()),
        };

    let _ = sender.send(AsyncResult {
        id,
        result: final_result,
    });
}
