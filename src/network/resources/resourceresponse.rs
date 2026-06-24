use super::*;
use super::http3::Http3Client;
use super::security::{AccessControl, CookieJar, Origin};
use crate::shared::intercept::InterceptResult;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;



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
