{
        // 0. Service Worker Interception
        // Convert URL to string for safety
        let url_str = url.clone();
        let origin_str = if let Ok(parsed) = crate::ace::url::parse(&url_str, None) {
            parsed.origin()
        } else {
            String::new()
        };

        if !origin_str.is_empty() {
            if let Ok(Some(reg)) = sw_manager.find_for_url(&origin_str, &url_str) {
                if let Ok(Some(active)) = reg.get_active() {
                    // Dispatch fetch event to Service Worker
                    // This is a simplified version of dispatch_fetch_event that handles the
                    // interception logic as requested in the plan.
                    let req_ctx = crate::ace::runtime::core::service_worker::RequestContext {
                        method: "GET".to_string(), // ResourceManager mostly does GET
                        url: url_str.clone(),
                        headers: HashMap::new(),
                        body: None,
                        mode: "navigate".to_string(),
                        credentials: "omit".to_string(),
                        cache_mode: crate::ace::runtime::core::service_worker::CacheMode::Default,
                        redirect: crate::ace::runtime::core::service_worker::RedirectMode::Follow,
                    };

                    // Try to intercept
                    if let Ok(InterceptResult::Handled(res_ctx)) =
                        sw_manager.dispatch_fetch_event(&active, req_ctx)
                    {
                        tracing::info!(url = %url_str, "Intercepted by Service Worker");
                        let response = ResourceResponse {
                            url: url_str.clone(),
                            data: res_ctx.body,
                            resource_type: resource_type.clone(),
                            etag: res_ctx.headers.get("etag").cloned(),
                            cache_control: res_ctx.headers.get("cache-control").cloned(),
                            last_modified: res_ctx.headers.get("last-modified").cloned(),
                            expires: res_ctx.headers.get("expires").cloned(),
                            timestamp: std::time::SystemTime::now(),
                            content_type: res_ctx
                                .headers
                                .get("content-type")
                                .cloned()
                                .unwrap_or_else(|| "text/html".to_string()),
                            status_code: res_ctx.status,
                            original_size: 0,
                            compressed_with: crate::network::cache::CompressionMethod::None,
                            decoded_image: None,
                        };
                        Self::send_response(&tx, response);
                        return;
                    }
                }
            }
        }
}
