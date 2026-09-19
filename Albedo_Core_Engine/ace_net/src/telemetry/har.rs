use serde::Serialize;
use tokio::sync::mpsc;
use std::sync::Arc;
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

/// Coletor assíncrono de eventos de rede para o formato HAR (HTTP Archive)
pub struct HarExporter {
    sender: mpsc::UnboundedSender<HarEntry>,
}

impl HarExporter {
    pub fn new(output_path: std::path::PathBuf) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<HarEntry>();
        
        tokio::spawn(async move {
            let mut entries = Vec::new();
            while let Some(entry) = rx.recv().await {
                entries.push(entry);
                // Flush every 100 entries or similar, or just keep in memory until drop
            }
            
            let log = HarLog {
                version: "1.2".into(),
                creator: HarCreator {
                    name: "Albedo Browser".into(),
                    version: "1.0".into(),
                },
                entries,
            };
            
            let har_data = serde_json::json!({ "log": log });
            if let Ok(json_string) = serde_json::to_string_pretty(&har_data) {
                if let Ok(mut file) = File::create(&output_path).await {
                    let _ = file.write_all(json_string.as_bytes()).await;
                }
            }
        });
        
        Self { sender: tx }
    }
    
    pub fn record_entry(&self, entry: HarEntry) {
        let _ = self.sender.send(entry);
    }
}
