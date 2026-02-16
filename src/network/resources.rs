use std::sync::{Arc, Mutex};
use reqwest::Client;
use tokio::sync::mpsc;
use url::Url;
use super::security::{Origin, CookieJar, AccessControl};

#[derive(Debug, Clone)]
pub enum ResourceType {
    Html,
    Css,
    Image,
}

#[derive(Debug)]
pub struct ResourceResponse {
    pub url: String,
    pub data: Vec<u8>,
    pub resource_type: ResourceType,
}

#[derive(Clone)]
pub struct ResourceManager {
    pub client: Client,
    pub tx: mpsc::UnboundedSender<ResourceResponse>,
    pub cookie_jar: Arc<Mutex<CookieJar>>,
    pub access_control: Arc<Mutex<AccessControl>>,
}

impl ResourceManager {
    pub fn new(tx: mpsc::UnboundedSender<ResourceResponse>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("AlbedoBrowser/0.1 (Async)")
                .build()
                .unwrap_or_default(),
            tx,
            cookie_jar: Arc::new(Mutex::new(CookieJar::new())),
            access_control: Arc::new(Mutex::new(AccessControl::new())),
        }
    }

    pub fn fetch(&self, url: String, resource_type: ResourceType, parent_origin: Option<Origin>) {
        let client = self.client.clone();
        let tx = self.tx.clone();
        let url_clone = url.clone();
        let cookie_jar = self.cookie_jar.clone();

        // 1. Mixed Content Blocking
        if let Some(ref parent) = parent_origin {
            if parent.scheme == "https" && url.starts_with("http://") && !url.contains("localhost") {
                eprintln!("[Security] Mixed Content Bloqueado: {} tentou carregar recurso inseguro {}", parent, url);
                return;
            }
        }

        tokio::spawn(async move {
            let cookies = cookie_jar.lock().unwrap().get_cookies_for_url(&url_clone);
            
            let mut req_builder = client.get(&url_clone);
            if !cookies.is_empty() {
                req_builder = req_builder.header("Cookie", cookies);
            }

            match req_builder.send().await {
                Ok(resp) => {
                    // Update CookieJar
                    if let Some(cookie_header) = resp.headers().get("set-cookie") {
                        if let Ok(c_str) = cookie_header.to_str() {
                            cookie_jar.lock().unwrap().set_cookie(&url_clone, c_str);
                        }
                    }

                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = tx.send(ResourceResponse {
                                url: url_clone,
                                data: bytes.to_vec(),
                                resource_type,
                            });
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
