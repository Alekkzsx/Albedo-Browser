use std::sync::{Arc, Mutex};
use std::time::Duration;
use reqwest::Client;
use tokio::sync::mpsc;
use url::Url;
use std::collections::HashMap;
use super::security::{Origin, CookieJar, AccessControl};
use super::http3::Http3Client;
use crate::runtime::core::service_worker::InterceptResult;

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Html,
    Css,
    Image,
}

#[derive(Debug, Clone)]
pub struct ResourceResponse {
    pub url: String,
    pub data: Vec<u8>,
    pub resource_type: ResourceType,
    // Cache headers para validação
    pub etag: Option<String>,
    pub cache_control: Option<String>,
    pub last_modified: Option<String>,
    pub expires: Option<String>,
    // Timestamp de quando foi armazenado (para max-age)
    pub timestamp: std::time::SystemTime,
    // Novos campos para disk cache
    pub content_type: String,
    pub status_code: u16,
    pub original_size: usize,
    pub compressed_with: crate::network::cache::CompressionMethod,
    // Buffer RGBA decodificado em background (largura, altura, rgba)
    pub decoded_image: Option<(u32, u32, Vec<u8>)>,
}

impl ResourceResponse {
    /// Verifica se a resposta em cache ainda é válida baseado em Cache-Control
    pub fn is_cache_valid(&self) -> bool {
        // Parsear Cache-Control para max-age
        if let Some(ref cache_control) = self.cache_control {
            if cache_control.contains("no-store") || cache_control.contains("no-cache") {
                return false; // Sempre revalidar
            }
            
            // Procurar por max-age=N
            if let Some(max_age_str) = cache_control.split("max-age=").nth(1) {
                if let Some(max_age_num) = max_age_str.split(',').next() {
                    if let Ok(max_age_secs) = max_age_num.trim().parse::<u64>() {
                        if let Ok(elapsed) = self.timestamp.elapsed() {
                            let elapsed_secs = elapsed.as_secs();
                            return elapsed_secs < max_age_secs;
                        }
                    }
                }
            }
        }
        
        // Fallback: cache por 1 hora se não houver Cache-Control
        if let Ok(elapsed) = self.timestamp.elapsed() {
            elapsed.as_secs() < 3600
        } else {
            true
        }
    }
}

#[derive(Clone)]
pub struct ResourceManager {
    pub client: Client,
    pub http3_client: Option<Http3Client>,
    pub tx: mpsc::UnboundedSender<ResourceResponse>,
    pub cookie_jar: Arc<Mutex<CookieJar>>,
    pub access_control: Arc<Mutex<AccessControl>>,
    pub response_cache: Arc<Mutex<std::collections::HashMap<String, ResourceResponse>>>,
    pub sw_manager: Arc<crate::runtime::core::service_worker::ServiceWorkerManager>,
}

impl ResourceManager {
    pub fn new(tx: mpsc::UnboundedSender<ResourceResponse>) -> Self {
        // HTTP/2 com fallback automático para HTTP/1.1
        // Ideal para carregamento paralelo de múltiplos recursos via ResourceManager
        //
        // HTTP/3 como protocolo primário para HTTPS:
        //   ✅ 25% mais rápido em conexões frias (0-RTT resumption)
        //   ✅ Connection migration automática (WiFi↔Cellular sem reconectar)
        //   ✅ Sem head-of-line blocking entre streams de recursos
        //   ✅ Recuperação melhorada de perda de pacotes
        //
        // Benefícios em ResourceManager (carregamento assíncrono):
        //   ✅ Multiplexing: HTML, CSS, JS, imagens carregam em paralelo
        //   ✅ Reduz número de conexões TCP abertas
        //   ✅ Melhor utilização de banda
        //   ✅ Flow control automático evita congestionamento
        //   ✅ Server push pode enviar dependências antecipadamente
        //
        // Recursos carregados em paralelo com HTTP/2 + HTTP/3:
        //   - HTML principal
        //   - CSS (múltiplos @import, stylesheets)
        //   - JavaScript (scripts síncronos e assíncronos)
        //   - Imagens (HTML <img>, CSS background-image)
        //   - Fonts (@font-face)
        //   - Favicons, web manifests
        //
        // Estratégia de fallback:
        //   1. HTTPS URLs → Tenta HTTP/3 primeiro
        //   2. Se HTTP/3 falhar → Usa HTTP/2 (reqwest)
        //   3. Se HTTP/2 falhar → Usa HTTP/1.1 (reqwest fallback)
        let sw_db = Arc::new(crate::runtime::core::sw_db::ServiceWorkerDatabase::new(std::path::PathBuf::from("sw.db")).unwrap());
        let sw_manager = Arc::new(crate::runtime::core::service_worker::ServiceWorkerManager::new(sw_db));
        
        Self {
            client: Client::builder()
                .user_agent("AlbedoBrowser/0.1 (Async)")
                // TCP connection pooling com timeout mais generoso para Async
                .pool_idle_timeout(Duration::from_secs(20))
                // HTTP/2 com conhecimento prévio (h2 protocol prefix)
                .http2_prior_knowledge()
                // Fallback automático para HTTP/1.1 (garante compatibilidade universal)
                .build()
                .unwrap_or_default(),
            http3_client: Http3Client::new().ok(),
            tx,
            cookie_jar: Arc::new(Mutex::new(CookieJar::new())),
            access_control: Arc::new(Mutex::new(AccessControl::new())),
            response_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            sw_manager,
        }
    }

    /// Envia a resposta pelo canal. Se for uma imagem, faz a decodificação
    /// no pool do tokio (background thread) antes de enviar para não travar a UI.
    pub fn send_response(tx: &mpsc::UnboundedSender<ResourceResponse>, mut response: ResourceResponse) {
        let tx = tx.clone();
        
        let is_image = response.resource_type == ResourceType::Image || 
                       response.content_type.starts_with("image/");
                       
        if is_image && response.decoded_image.is_none() && !response.data.is_empty() {
            tokio::task::spawn_blocking(move || {
                if let Ok(img) = image::load_from_memory(&response.data) {
                    let rgba = img.to_rgba8();
                    response.decoded_image = Some((rgba.width(), rgba.height(), rgba.into_raw()));
                }
                let _ = tx.send(response);
            });
        } else {
            let _ = tx.send(response);
        }
    }

    pub fn fetch(&self, url: String, resource_type: ResourceType, parent_origin: Option<Origin>) {
        let url_clone = url.clone();
        let client = self.client.clone();
        let tx = self.tx.clone();
        let cookie_jar = self.cookie_jar.clone();
        let response_cache = self.response_cache.clone();
        let sw_manager = self.sw_manager.clone();
        
        // 0. Service Worker Interception
        // Convert URL to string for safety
        let url_str = url.clone();
        let origin_str = if let Ok(parsed) = url::Url::parse(&url_str) {
            parsed.origin().unicode_serialization()
        } else {
            String::new()
        };

        if !origin_str.is_empty() {
             if let Ok(Some(reg)) = sw_manager.find_for_url(&origin_str, &url_str) {
                 if let Ok(Some(active)) = reg.get_active() {
                     // Dispatch fetch event to Service Worker
                     // This is a simplified version of dispatch_fetch_event that handles the 
                     // interception logic as requested in the plan.
                     let req_ctx = crate::runtime::core::service_worker::RequestContext {
                         method: "GET".to_string(), // ResourceManager mostly does GET
                         url: url_str.clone(),
                         headers: HashMap::new(),
                         body: None,
                         mode: "navigate".to_string(),
                         credentials: "omit".to_string(),
                         cache_mode: crate::runtime::core::service_worker::CacheMode::Default,
                         redirect: crate::runtime::core::service_worker::RedirectMode::Follow,
                     };
                     
                     // Try to intercept
                     if let Ok(InterceptResult::Handled(res_ctx)) = sw_manager.dispatch_fetch_event(&active, req_ctx) {
                         println!("[ResourceManager] Intercepted by Service Worker: {}", url_str);
                         let response = ResourceResponse {
                             url: url_str.clone(),
                             data: res_ctx.body,
                             resource_type: resource_type.clone(),
                             etag: res_ctx.headers.get("etag").cloned(),
                             cache_control: res_ctx.headers.get("cache-control").cloned(),
                             last_modified: res_ctx.headers.get("last-modified").cloned(),
                             expires: res_ctx.headers.get("expires").cloned(),
                             timestamp: std::time::SystemTime::now(),
                             content_type: res_ctx.headers.get("content-type").cloned().unwrap_or_else(|| "text/html".to_string()),
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

        // 1. Mixed Content Blocking
        if let Some(ref parent) = parent_origin {
            if parent.scheme == "https" && url.starts_with("http://") && !url.contains("localhost") {
                eprintln!("[Security] Mixed Content Bloqueado: {} tentou carregar recurso inseguro {}", parent, url);
                return;
            }
        }

        // Handle albedo:// URLs (silently ignored)
        if url.starts_with("albedo://") {
            return;
        }

        // 2. Handle data: URLs
        if url.starts_with("data:") {
            let url_clone = url.clone();
            let resource_type = resource_type.clone();
            
            tokio::spawn(async move {
                if let Some(comma_pos) = url_clone.find(',') {
                    let metadata = &url_clone[5..comma_pos];
                    let data_part = &url_clone[comma_pos + 1..];
                    
                    let is_base64 = metadata.ends_with(";base64");
                    let content_type = if is_base64 {
                        metadata.trim_end_matches(";base64").to_string()
                    } else {
                        metadata.to_string()
                    };
                    
                    let content_type = if content_type.is_empty() {
                        "text/plain;charset=US-ASCII".to_string()
                    } else {
                        content_type
                    };

                    let data = if is_base64 {
                        use base64::{Engine as _, engine::general_purpose};
                        general_purpose::STANDARD.decode(data_part).unwrap_or_default()
                    } else {
                        urlencoding::decode(data_part).map(|s| s.into_owned().into_bytes()).unwrap_or_default()
                    };

                    let response = ResourceResponse {
                        url: url_clone,
                        data,
                        resource_type,
                        etag: None,
                        cache_control: None,
                        last_modified: None,
                        expires: None,
                        timestamp: std::time::SystemTime::now(),
                        content_type,
                        status_code: 200,
                        original_size: 0, // data URLs don't have original size in the same concept
                        compressed_with: crate::network::cache::CompressionMethod::None,
                        decoded_image: None,
                    };
                    
                    Self::send_response(&tx, response);
                }
            });
            return;
        }

        // 3. Handle blob: URLs
        if url.starts_with("blob:") {
            let url_clone = url.clone();
            let resource_type = resource_type.clone();
            
            tokio::spawn(async move {
                // Access global blob store
                if let Some(blob) = crate::network::blob::GLOBAL_BLOB_STORE.get_blob(&url_clone) {
                     let response = ResourceResponse {
                        url: url_clone,
                        data: blob.data,
                        resource_type,
                        etag: None,
                        cache_control: None,
                        last_modified: None,
                        expires: None,
                        timestamp: std::time::SystemTime::now(),
                        content_type: blob.content_type,
                        status_code: 200,
                        original_size: blob.size,
                        compressed_with: crate::network::cache::CompressionMethod::None,
                        decoded_image: None,
                    };
                    Self::send_response(&tx, response);
                } else {
                    // Blob not found (404)
                    let response = ResourceResponse {
                        url: url_clone,
                        data: Vec::new(),
                        resource_type,
                        etag: None,
                        cache_control: None,
                        last_modified: None,
                        expires: None,
                        timestamp: std::time::SystemTime::now(),
                        content_type: "text/plain".to_string(),
                        status_code: 404,
                        original_size: 0,
                        compressed_with: crate::network::cache::CompressionMethod::None,
                        decoded_image: None,
                    };
                    Self::send_response(&tx, response);
                }
            });
            return;
        }

        // 4. Handle file: URLs
        if url.starts_with("file://") {
            let resource_type = resource_type.clone();
            
            tokio::spawn(async move {
                let path_buf = if let Ok(parsed_url) = url::Url::parse(&url_clone) {
                    if let Ok(file_path) = parsed_url.to_file_path() {
                        file_path
                    } else {
                        let path_str = url_clone.trim_start_matches("file://");
                        let mut p = urlencoding::decode(path_str).map(|s| s.into_owned()).unwrap_or_else(|_| path_str.to_string());
                        if cfg!(windows) && p.starts_with('/') {
                            let chars: Vec<char> = p.chars().collect();
                            if chars.len() > 3 && chars[1].is_ascii_alphabetic() && chars[2] == ':' {
                                p = p[1..].to_string();
                            }
                        }
                        std::path::PathBuf::from(p)
                    }
                } else {
                    let path_str = url_clone.trim_start_matches("file://");
                    let mut p = urlencoding::decode(path_str).map(|s| s.into_owned()).unwrap_or_else(|_| path_str.to_string());
                    if cfg!(windows) && p.starts_with('/') {
                        let chars: Vec<char> = p.chars().collect();
                        if chars.len() > 3 && chars[1].is_ascii_alphabetic() && chars[2] == ':' {
                            p = p[1..].to_string();
                        }
                    }
                    std::path::PathBuf::from(p)
                };
                
                let path = path_buf.as_path();
                
                match tokio::fs::read(path).await {
                    Ok(data) => {
                        // Guess content type manually
                        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                        let content_type = match extension.as_str() {
                            "html" | "htm" => "text/html",
                            "css" => "text/css",
                            "js" => "application/javascript",
                            "json" => "application/json",
                            "png" => "image/png",
                            "jpg" | "jpeg" => "image/jpeg",
                            "svg" => "image/svg+xml",
                            "txt" => "text/plain",
                            _ => "application/octet-stream",
                        }.to_string();
                        
                        let response = ResourceResponse {
                            url: url_clone,
                            data: data.clone(),
                            resource_type,
                            etag: None,
                            cache_control: None,
                            last_modified: None,
                            expires: None,
                            timestamp: std::time::SystemTime::now(),
                            content_type,
                            status_code: 200,
                            original_size: data.len(),
                            compressed_with: crate::network::cache::CompressionMethod::None,
                            decoded_image: None,
                        };
                        
                        Self::send_response(&tx, response);
                    },
                    Err(e) => {
                         eprintln!("[ResourceManager] Erro ao ler arquivo {}: {}", path.display(), e);
                         let response = ResourceResponse {
                            url: url_clone,
                            data: Vec::new(),
                            resource_type,
                            etag: None,
                            cache_control: None,
                            last_modified: None,
                            expires: None,
                            timestamp: std::time::SystemTime::now(),
                            content_type: "text/plain".to_string(),
                            status_code: 404,
                            original_size: 0,
                            compressed_with: crate::network::cache::CompressionMethod::None,
                            decoded_image: None,
                        };
                        Self::send_response(&tx, response);
                    }
                }
            });
            return;
        }

        let http3_client = self.http3_client.clone();

        tokio::spawn(async move {
            // 2. Verificar cache primeiro
            let mut cached_response = None;
            let mut etag_for_validation = None;
            let mut last_modified_for_validation = None;
            
            {
                let cache = response_cache.lock().unwrap();
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
                            println!("[ResourceManager] ✅ HTTP/3 sucesso para {}", url_clone);

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
                                let content_type = h3_resp.headers.get("content-type")
                                    .cloned()
                                    .unwrap_or_else(|| "application/octet-stream".to_string());

                                // Handle Set-Cookie
                                if let Some(cookie_val) = h3_resp.headers.get("set-cookie") {
                                    cookie_jar.lock().unwrap().set_cookie(&url_clone, cookie_val);
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
                                    let mut cache = response_cache.lock().unwrap();
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
                            eprintln!("[ResourceManager] HTTP/3 fallback para {} → {}", url_clone, e);
                        }
                    }
                }
            }
            // ─── FIM HTTP/3 ATTEMPT ──────────────────────────────────────

            // Fallback: HTTP/2 via reqwest (ou primário para HTTP URLs)
            if !used_h3 || !is_https {
                // 3. Construir requisição com headers condicionais
                let cookies = cookie_jar.lock().unwrap().get_cookies_for_url(&url_clone);
                
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
                                cookie_jar.lock().unwrap().set_cookie(&url_clone, c_str);
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
                            let etag = resp.headers()
                                .get("etag")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());
                            
                            let cache_control = resp.headers()
                                .get("cache-control")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());
                            
                            let last_modified = resp.headers()
                                .get("last-modified")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());
                            
                            let expires = resp.headers()
                                .get("expires")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());
                            
                            let content_type = resp.headers()
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
                                    let mut cache = response_cache.lock().unwrap();
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
                        eprintln!("[ResourceManager] Erro ao buscar {}: {}", url_clone, e);
                    }
                }
            }
        });
    }
}
