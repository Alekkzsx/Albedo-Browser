            // 2. Verificar cache primeiro
            let mut cached_response = None;
            let mut etag_for_validation = None;
            let mut last_modified_for_validation = None;

            {
                let cache = response_cache.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(cached) = cache.get(&url_clone) {
                    if cached.is_cache_valid() {
                        // Cache ainda é válido, usar imediatamente
                        Self::send_response(&tx, cached.clone());
                        return;
                    } else {
                        // Cache expirou, mas podemos revalidar
                        etag_for_validation = cached.etag.clone();
                        last_modified_for_validation = cached.last_modified.clone();
                        cached_response = Some(cached.clone());
                    }
                }
            }

            // ─── ESTRATÉGIA HTTP/3 → HTTP/2 FALLBACK ─────────────────────
            // Para URLs HTTPS: tenta HTTP/3 (QUIC) primeiro.
            // Se HTTP/3 falhar (servidor não suporta, timeout, erro) → fallback
            // para HTTP/2 via reqwest (TCP + TLS).
            // Para URLs HTTP: usa reqwest diretamente (QUIC requer TLS).
            let is_https = url_clone.starts_with("https://");
            let mut used_h3 = false;

            if is_https {
                if let Some(ref h3) = http3_client {
                    // Tentar HTTP/3 via QUIC
                    match h3.get(&url_clone).await {
                        Ok(h3_resp) => {
                            used_h3 = true;
                            tracing::info!(url = %url_clone, "HTTP/3 success");

                            // Processar resposta HTTP/3
                            let status_code = h3_resp.status;

                            // Handle 304 Not Modified (revalidação via cache)
                            if status_code == 304 {
                                if let Some(cached) = cached_response {
                                    Self::send_response(&tx, cached);
                                }
                                return;
                            }

                            if status_code >= 200 && status_code < 300 {
                                let etag = h3_resp.headers.get("etag").cloned();
                                let cache_control = h3_resp.headers.get("cache-control").cloned();
                                let last_modified = h3_resp.headers.get("last-modified").cloned();
                                let expires = h3_resp.headers.get("expires").cloned();
                                let content_type = h3_resp
                                    .headers
                                    .get("content-type")
                                    .cloned()
                                    .unwrap_or_else(|| "application/octet-stream".to_string());

                                // Handle Set-Cookie
                                if let Some(cookie_val) = h3_resp.headers.get("set-cookie") {
                                    cookie_jar
                                        .lock().unwrap_or_else(|e| e.into_inner())
                                        .set_cookie(&url_clone, cookie_val);
                                }

                                let body_len = h3_resp.body.len();
                                let response = ResourceResponse {
                                    url: url_clone.clone(),
                                    data: h3_resp.body,
                                    resource_type,
                                    etag,
                                    cache_control,
                                    last_modified,
                                    expires,
                                    timestamp: std::time::SystemTime::now(),
                                    content_type,
                                    status_code,
                                    original_size: body_len,
                                    compressed_with: crate::network::cache::CompressionMethod::None,
                                    decoded_image: None,
                                };

                                // Armazenar em cache
                                {
                                    let mut cache = response_cache.lock().unwrap_or_else(|e| e.into_inner());
                                    if cache.len() >= 100 {
                                        if let Some(oldest_key) = cache.keys().next().cloned() {
                                            cache.remove(&oldest_key);
                                        }
                                    }
                                    cache.insert(url_clone, response.clone());
                                }

                                Self::send_response(&tx, response);
                                return;
                            }
                            // Se status não é sucesso (4xx, 5xx), tentar fallback HTTP/2
                        }
                        Err(e) => {
                            // HTTP/3 falhou — fallback para HTTP/2
                            tracing::warn!(url = %url_clone, ?e, "HTTP/3 failed, falling back to HTTP/2");
                        }
                    }
                }
            }
            // ─── FIM HTTP/3 ATTEMPT ──────────────────────────────────────

            // Fallback: HTTP/2 via reqwest (ou primário para HTTP URLs)
            if !used_h3 || !is_https {
                // 3. Construir requisição com headers condicionais
                let cookies = cookie_jar.lock().unwrap_or_else(|e| e.into_inner()).get_cookies_for_url(&url_clone);

                let mut req_builder = client.get(&url_clone);
                if !cookies.is_empty() {
                    req_builder = req_builder.header("Cookie", cookies);
                }

                // Se temos ETag ou Last-Modified em cache, usar para revalidação
                if let Some(etag) = etag_for_validation {
                    req_builder = req_builder.header("If-None-Match", etag);
                }
                if let Some(last_mod) = last_modified_for_validation {
                    req_builder = req_builder.header("If-Modified-Since", last_mod);
                }

                match req_builder.send().await {
                    Ok(resp) => {
                        let status = resp.status();

                        // Update CookieJar
                        if let Some(cookie_header) = resp.headers().get("set-cookie") {
                            if let Ok(c_str) = cookie_header.to_str() {
                                cookie_jar.lock().unwrap_or_else(|e| e.into_inner()).set_cookie(&url_clone, c_str);
                            }
                        }

                        // 4. Handle 304 Not Modified
                        if status == 304 {
                            // Usar cached response
                            if let Some(cached) = cached_response {
                                Self::send_response(&tx, cached);
                            }
                            return;
                        }

                        if status.is_success() {
                            // 5. Extrair headers de cache ANTES de consumir resp.bytes()
                            let etag = resp
                                .headers()
                                .get("etag")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());

                            let cache_control = resp
                                .headers()
                                .get("cache-control")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());

                            let last_modified = resp
                                .headers()
                                .get("last-modified")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());

                            let expires = resp
                                .headers()
                                .get("expires")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());

                            let content_type = resp
                                .headers()
                                .get("content-type")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("application/octet-stream")
                                .to_string();

                            let status_code = status.as_u16();

                            if let Ok(bytes) = resp.bytes().await {
                                let response = ResourceResponse {
                                    url: url_clone.clone(),
                                    data: bytes.to_vec(),
                                    resource_type,
                                    etag,
                                    cache_control,
                                    last_modified,
                                    expires,
                                    timestamp: std::time::SystemTime::now(),
                                    content_type,
                                    status_code,
                                    original_size: bytes.len(),
                                    compressed_with: crate::network::cache::CompressionMethod::None,
                                    decoded_image: None,
                                };

                                // 6. Armazenar em cache
                                {
                                    let mut cache = response_cache.lock().unwrap_or_else(|e| e.into_inner());
                                    if cache.len() >= 100 {
                                        if let Some(oldest_key) = cache.keys().next().cloned() {
                                            cache.remove(&oldest_key);
                                        }
                                    }
                                    cache.insert(url_clone, response.clone());
                                }

                                // 7. Enviar resposta
                                Self::send_response(&tx, response);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(url = %url_clone, ?e, "Failed to fetch resource");
                    }
                }
            }
        });
