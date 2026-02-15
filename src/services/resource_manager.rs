use std::sync::{Arc, Mutex};
use reqwest::Client;
use tokio::sync::mpsc;
use url::Url;

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
    client: Client,
    pub tx: mpsc::UnboundedSender<ResourceResponse>,
}

impl ResourceManager {
    pub fn new(tx: mpsc::UnboundedSender<ResourceResponse>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("AlbedoBrowser/0.1 (Async)")
                .build()
                .unwrap_or_default(),
            tx,
        }
    }

    pub fn fetch(&self, url: String, resource_type: ResourceType) {
        let client = self.client.clone();
        let tx = self.tx.clone();
        let url_clone = url.clone();

        tokio::spawn(async move {
            match client.get(&url_clone).send().await {
                Ok(resp) => {
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
