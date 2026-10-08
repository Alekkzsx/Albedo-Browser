use serde::Serialize;
use tokio::sync::mpsc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarEntry {
    pub started_date_time: String,
    pub time: f64,
    pub request: HarRequest,
    pub response: HarResponse,
    pub timings: HarTimings,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub http_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarResponse {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    pub body_size: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarTimings {
    pub blocked: f64,
    pub dns: f64,
    pub connect: f64,
    pub send: f64,
    pub wait: f64,
    pub receive: f64,
    pub ssl: f64,
}

impl HarEntry {
    /// Constrói uma entrada HAR completa a partir dos metadados de uma requisição e resposta do motor.
    pub fn from_response(
        req: &crate::http::request::Request,
        resp: &crate::http::response::Response,
    ) -> Self {
        let now = std::time::SystemTime::now();
        let started_date_time = httpdate::fmt_http_date(now);
        let time = resp.timing.total_duration.as_secs_f64() * 1000.0;

        let http_version = if req.force_h3 {
            "HTTP/3".to_string()
        } else {
            "HTTP/2".to_string()
        };

        let request = HarRequest {
            method: req.method.as_str().to_string(),
            url: req.url.to_string(),
            http_version: http_version.clone(),
        };

        let response = HarResponse {
            status: resp.status.as_u16(),
            status_text: resp.status.canonical_reason().unwrap_or("").to_string(),
            http_version,
            body_size: resp.len() as i64,
        };

        let timings = HarTimings {
            blocked: -1.0,
            dns: resp.timing.dns_duration.map_or(-1.0, |d| d.as_secs_f64() * 1000.0),
            connect: resp.timing.tcp_duration.map_or(-1.0, |d| d.as_secs_f64() * 1000.0),
            ssl: resp.timing.tls_duration.map_or(-1.0, |d| d.as_secs_f64() * 1000.0),
            send: 0.0,
            wait: resp.timing.ttfb.as_secs_f64() * 1000.0,
            receive: resp.timing.total_duration.saturating_sub(resp.timing.ttfb).as_secs_f64() * 1000.0,
        };

        Self {
            started_date_time,
            time,
            request,
            response,
            timings,
        }
    }
}

/// Coletor de eventos de rede para o formato padrão HAR (HTTP Archive 1.2).
pub struct HarExporter {
    entries: std::sync::Arc<parking_lot::RwLock<Vec<HarEntry>>>,
    output_path: Option<std::path::PathBuf>,
}

impl HarExporter {
    /// Cria um novo exportador associado opcionalmente a um caminho em disco.
    pub fn new(output_path: std::path::PathBuf) -> Self {
        Self {
            entries: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            output_path: Some(output_path),
        }
    }

    /// Cria um novo exportador puramente em memória (ideal para DevTools e testes).
    pub fn in_memory() -> Self {
        Self {
            entries: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            output_path: None,
        }
    }

    /// Registra uma nova entrada de requisição no log HAR.
    pub fn record_entry(&self, entry: HarEntry) {
        let mut list = self.entries.write();
        list.push(entry);

        if let Some(ref path) = self.output_path {
            let path_clone = path.clone();
            let json = self.build_har_json_internal(&list);
            tokio::spawn(async move {
                if let Ok(mut file) = File::create(&path_clone).await {
                    let _ = file.write_all(json.as_bytes()).await;
                }
            });
        }
    }

    /// Retorna a contagem atual de entradas gravadas.
    pub fn entries_count(&self) -> usize {
        self.entries.read().len()
    }

    /// Exporta o log HAR acumulado como string JSON formatada.
    pub fn export_json(&self) -> String {
        let list = self.entries.read();
        self.build_har_json_internal(&list)
    }

    fn build_har_json_internal(&self, entries: &[HarEntry]) -> String {
        let log = HarLog {
            version: "1.2".into(),
            creator: HarCreator {
                name: "Albedo Browser".into(),
                version: "0.1.0".into(),
            },
            entries: entries.to_vec(),
        };

        let har_data = serde_json::json!({ "log": log });
        serde_json::to_string_pretty(&har_data).unwrap_or_default()
    }
}
