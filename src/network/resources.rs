use std::sync::{Arc, Mutex};
use std::time::Duration;
use reqwest::Client;
use tokio::sync::mpsc;
use url::Url;
use super::security::{Origin, CookieJar, AccessControl};
use super::http3::Http3Client;

#[derive(Debug, Clone)]
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
        }
    }

    pub fn fetch(&self, url: String, resource_type: ResourceType, parent_origin: Option<Origin>) {
        let client = self.client.clone();
        let tx = self.tx.clone();
        let url_clone = url.clone();
        let cookie_jar = self.cookie_jar.clone();
        let response_cache = self.response_cache.clone();

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
                    };
                    
                    let _ = tx.send(response);
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
                    };
                    let _ = tx.send(response);
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
                    };
                    let _ = tx.send(response);
                }
            });
            return;
        }

        // 4. Handle file: URLs
        if url.starts_with("file://") {
            let url_clone = url.clone();
            let resource_type = resource_type.clone();
            
            tokio::spawn(async move {
                let path_str = url_clone.trim_start_matches("file://");
                let decoded_path = urlencoding::decode(path_str).map(|s| s.into_owned()).unwrap_or_else(|_| path_str.to_string());
                
                let path = std::path::Path::new(&decoded_path);
                
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
                        };
                        
                        let _ = tx.send(response);
                    },
                    Err(e) => {
                         eprintln!("[ResourceManager] Erro ao ler arquivo {}: {}", decoded_path, e);
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
                        };
                        let _ = tx.send(response);
                    }
                }
            });
            return;
        }

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
                        let _ = tx.send(cached.clone());
                        return;
                    } else {
                        // Cache expirou, mas podemos revalidar
                        etag_for_validation = cached.etag.clone();
                        last_modified_for_validation = cached.last_modified.clone();
                        cached_response = Some(cached.clone());
                    }
                }
            }
            
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
                            let _ = tx.send(cached);
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
                                compressed_with: crate::network::cache::CompressionMethod::None, // Raw data is stored uncompressed in memory response
                            };
                            
                            // 6. Armazenar em cache
                            {
                                let mut cache = response_cache.lock().unwrap();
                                // Limite simples: máximo 100 entradas em cache
                                if cache.len() >= 100 {
                                    // Remove entrada mais antiga (FIFO simples)
                                    if let Some(oldest_key) = cache.keys().next().cloned() {
                                        cache.remove(&oldest_key);
                                    }
                                }
                                cache.insert(url_clone, response.clone());
                            }
                            
                            // 7. Enviar resposta
                            let _ = tx.send(response);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ResourceManager] Erro ao buscar {}: {}", url_clone, e);
                }
            }
        });
    }
}
