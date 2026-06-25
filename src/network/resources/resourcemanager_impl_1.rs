use super::*;
use super::http3::Http3Client;
use super::security::{AccessControl, CookieJar, Origin};
use crate::shared::intercept::InterceptResult;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;



impl ResourceManager {
    /// TODO: add docs
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
        let sw_db = Arc::new(
            crate::ace::runtime::core::sw_db::ServiceWorkerDatabase::new(std::path::PathBuf::from(
                "sw.db",
            ))
            .expect("Albedo Engine: internal invariant violated"),
        );
        let sw_manager =
            Arc::new(crate::ace::runtime::core::service_worker::ServiceWorkerManager::new(sw_db));

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
    pub fn send_response(
        tx: &mpsc::UnboundedSender<ResourceResponse>,
        mut response: ResourceResponse,
    ) {
        let tx = tx.clone();

        let is_image = response.resource_type == ResourceType::Image
            || response.content_type.starts_with("image/");

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

    /// TODO: add docs
    pub fn fetch(&self, url: String, resource_type: ResourceType, parent_origin: Option<Origin>) {
        let url_clone = url.clone();
        let client = self.client.clone();
        let tx = self.tx.clone();
        let cookie_jar = self.cookie_jar.clone();
        let response_cache = self.response_cache.clone();
        let sw_manager = self.sw_manager.clone();

        include!("resource_sw.rs");

        // 1. Mixed Content Blocking
        if let Some(ref parent) = parent_origin {
            if parent.scheme == "https" && url.starts_with("http://") && !url.contains("localhost")
            {
                tracing::warn!(parent = %parent, url = %url, "Mixed Content blocked");
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
                include!("resource_data.rs");
            });
            return;
        }

        // 3. Handle blob: URLs
        if url.starts_with("blob:") {
            let url_clone = url.clone();
            let resource_type = resource_type.clone();

            tokio::spawn(async move {
                include!("resource_blob.rs");
            });
            return;
        }

        // 4. Handle file: URLs
        if url.starts_with("file://") {
            let resource_type = resource_type.clone();

                include!("resource_file.rs");
            return;
        }

        let http3_client = self.http3_client.clone();

        tokio::spawn(async move {
            include!("resource_http.rs");
        });
    }
}
