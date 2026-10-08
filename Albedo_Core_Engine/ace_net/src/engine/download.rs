//! # Gerenciador de Downloads Resumíveis e Telemetria (`DownloadSession`)
//!
//! Implementa o mecanismo de transferência contínua de arquivos volumosos em disco,
//! com suporte a pausas e retomadas atômicas via requisições de faixa (`Range: bytes=N-` e `If-Range`),
//! isolamento em arquivo temporário `.albedodownload` e telemetria de progresso em tempo real.

use crate::engine::fetcher::ResourceFetcher;
use crate::error::{NetError, NetResult};
use crate::http::request::{Request, RequestDestination};
use ace_core::id::RequestId;
use http::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_RANGE, RANGE};
use http::StatusCode;
use smol_str::SmolStr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use url::Url;

/// Extensão temporária utilizada durante o download ativo antes da consolidação atômica.
pub const DOWNLOAD_TEMP_EXTENSION: &str = "albedodownload";

/// Estado do ciclo de vida de um download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DownloadState {
    #[default]
    Idle,
    Downloading,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

/// Instantâneo de telemetria de progresso do download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Total acumulado de bytes salvos no disco até o momento.
    pub bytes_downloaded: u64,
    /// Tamanho total esperado do arquivo fornecido pelo servidor (se informado).
    pub total_bytes: Option<u64>,
    /// Taxa instantânea de transferência em bytes por segundo.
    pub bytes_per_second: f64,
    /// Porcentagem de conclusão (0.0 a 100.0).
    pub progress_percentage: Option<f32>,
    /// Indica se o servidor confirmou suporte a retomada (`Range` / `206 Partial Content`).
    pub is_resumable: bool,
    /// ETag de validação da versão do arquivo no servidor.
    pub etag: Option<SmolStr>,
    /// Estado atual da transferência.
    pub state: DownloadState,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            bytes_downloaded: 0,
            total_bytes: None,
            bytes_per_second: 0.0,
            progress_percentage: None,
            is_resumable: false,
            etag: None,
            state: DownloadState::Idle,
        }
    }
}

/// Parâmetros de configuração para início de um download.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// URL de origem do arquivo a ser transferido.
    pub url: Url,
    /// Caminho final de destino onde o arquivo consolidado será persistido.
    pub destination_path: PathBuf,
    /// Permite tentar continuar de onde parou caso o arquivo parcial já exista.
    pub allow_resume: bool,
    /// ETag previamente conhecido para validação de integridade `If-Range`.
    pub previous_etag: Option<SmolStr>,
}

impl DownloadOptions {
    pub fn new(url: Url, destination_path: impl Into<PathBuf>) -> Self {
        Self {
            url,
            destination_path: destination_path.into(),
            allow_resume: true,
            previous_etag: None,
        }
    }

    /// Retorna o caminho do arquivo temporário com sufixo `.albedodownload`.
    pub fn temp_path(&self) -> PathBuf {
        let mut path_str = self.destination_path.as_os_str().to_os_string();
        path_str.push(format!(".{}", DOWNLOAD_TEMP_EXTENSION));
        PathBuf::from(path_str)
    }
}

/// Sessão ativa de um download gerenciado pelo `ace_net`.
pub struct DownloadSession {
    pub id: RequestId,
    pub url: Url,
    pub destination_path: PathBuf,
    progress_receiver: watch::Receiver<DownloadProgress>,
    cancel_token: CancellationToken,
    is_paused: Arc<AtomicBool>,
}

impl DownloadSession {
    /// Retorna um instantâneo do progresso mais recente do download.
    pub fn progress(&self) -> DownloadProgress {
        self.progress_receiver.borrow().clone()
    }

    /// Obtém um receptor assíncrono para ser notificado de atualizações de progresso.
    pub fn subscribe(&self) -> watch::Receiver<DownloadProgress> {
        self.progress_receiver.clone()
    }

    /// Sinaliza a pausa do download. O arquivo parcial permanece no disco para retomada futura.
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        self.cancel_token.cancel();
    }

    /// Cancela o download definitivamente.
    pub fn cancel(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        self.cancel_token.cancel();
    }
}

/// Inicia o processo de download assíncrono conectado ao `ResourceFetcher`.
///
/// Retorna a `DownloadSession` e a `JoinHandle` da tarefa em background.
pub fn start_download(
    fetcher: Arc<ResourceFetcher>,
    options: DownloadOptions,
) -> (DownloadSession, tokio::task::JoinHandle<NetResult<()>>) {
    let id = RequestId::new();
    let cancel_token = CancellationToken::new();
    let is_paused = Arc::new(AtomicBool::new(false));

    let (progress_tx, progress_rx) = watch::channel(DownloadProgress {
        state: DownloadState::Idle,
        ..Default::default()
    });

    let session = DownloadSession {
        id,
        url: options.url.clone(),
        destination_path: options.destination_path.clone(),
        progress_receiver: progress_rx,
        cancel_token: cancel_token.clone(),
        is_paused: is_paused.clone(),
    };

    let worker_handle = tokio::spawn(async move {
        execute_download_worker(fetcher, options, cancel_token, is_paused, progress_tx).await
    });

    (session, worker_handle)
}

/// Trabalhador em background responsável pela negociação HTTP, escrita em disco e telemetria.
async fn execute_download_worker(
    fetcher: Arc<ResourceFetcher>,
    options: DownloadOptions,
    cancel_token: CancellationToken,
    is_paused: Arc<AtomicBool>,
    progress_tx: watch::Sender<DownloadProgress>,
) -> NetResult<()> {
    let temp_path = options.temp_path();

    // Cria os diretórios pais se não existirem
    if let Some(parent) = options.destination_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    // Verifica se já temos bytes gravados para tentativa de retomada
    let mut current_offset: u64 = 0;
    if options.allow_resume && temp_path.exists() {
        if let Ok(metadata) = tokio::fs::metadata(&temp_path).await {
            current_offset = metadata.len();
        }
    }

    let mut req_builder = Request::get(options.url.clone())?
        .destination(RequestDestination::Other)
        .streaming(true)
        .cancellation_token(cancel_token.clone());

    if current_offset > 0 {
        if let Ok(val) = http::HeaderValue::from_str(&format!("bytes={}-", current_offset)) {
            req_builder = req_builder.header(RANGE, val);
        }
        if let Some(etag) = &options.previous_etag {
            if let Ok(val) = http::HeaderValue::from_str(etag.as_str()) {
                req_builder = req_builder.header(IF_RANGE, val);
            }
        }
    }

    let request = req_builder.build();

    // Atualiza estado para Downloading
    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded: current_offset,
        state: DownloadState::Downloading,
        ..Default::default()
    });

    let response = fetcher.fetch(request).await?;
    let status = response.status;

    let etag: Option<SmolStr> = response
        .headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(SmolStr::new);

    let (mut file, mut bytes_downloaded, total_expected, is_resumable) = if status == StatusCode::PARTIAL_CONTENT && current_offset > 0 {
        // Servidor aceitou a retomada (206 Partial Content)
        let total = parse_total_from_content_range(response.headers.get(CONTENT_RANGE));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&temp_path)
            .await?;
        (file, current_offset, total, true)
    } else if status.is_success() {
        // Download do zero (200 OK) ou servidor rejeitou range
        let total = response
            .headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let accepts_ranges = response
            .headers
            .get(ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.eq_ignore_ascii_case("bytes"))
            .unwrap_or(false);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .await?;
        (file, 0u64, total, accepts_ranges)
    } else {
        let _ = progress_tx.send(DownloadProgress {
            bytes_downloaded: current_offset,
            state: DownloadState::Failed,
            ..Default::default()
        });
        return Err(NetError::HttpProtocolError(format!(
            "Download rejeitado com status HTTP {}",
            status
        )));
    };

    let mut stream = response
        .body
        .take_stream()
        .await
        .ok_or_else(|| NetError::HttpProtocolError("Corpo de resposta indisponível para streaming".into()))?;

    let mut last_sample_time = Instant::now();
    let mut bytes_since_sample: u64 = 0;
    let mut current_speed: f64 = 0.0;

    loop {
        if cancel_token.is_cancelled() {
            let final_state = if is_paused.load(Ordering::SeqCst) {
                DownloadState::Paused
            } else {
                DownloadState::Cancelled
            };
            let _ = progress_tx.send(DownloadProgress {
                bytes_downloaded,
                total_bytes: total_expected,
                bytes_per_second: current_speed,
                progress_percentage: calculate_percentage(bytes_downloaded, total_expected),
                is_resumable,
                etag,
                state: final_state,
            });
            return Ok(());
        }

        let chunk_opt = tokio::select! {
            _ = cancel_token.cancelled() => {
                let final_state = if is_paused.load(Ordering::SeqCst) {
                    DownloadState::Paused
                } else {
                    DownloadState::Cancelled
                };
                let _ = progress_tx.send(DownloadProgress {
                    bytes_downloaded,
                    total_bytes: total_expected,
                    bytes_per_second: current_speed,
                    progress_percentage: calculate_percentage(bytes_downloaded, total_expected),
                    is_resumable,
                    etag,
                    state: final_state,
                });
                return Ok(());
            }
            res = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)) => res,
        };

        match chunk_opt {
            Some(Ok(chunk)) => {
                let chunk_len = chunk.len() as u64;
                file.write_all(&chunk).await?;
                bytes_downloaded += chunk_len;
                bytes_since_sample += chunk_len;

                let elapsed = last_sample_time.elapsed();
                if elapsed.as_millis() >= 250 {
                    current_speed = (bytes_since_sample as f64) / elapsed.as_secs_f64();
                    bytes_since_sample = 0;
                    last_sample_time = Instant::now();

                    let _ = progress_tx.send(DownloadProgress {
                        bytes_downloaded,
                        total_bytes: total_expected,
                        bytes_per_second: current_speed,
                        progress_percentage: calculate_percentage(bytes_downloaded, total_expected),
                        is_resumable,
                        etag: etag.clone(),
                        state: DownloadState::Downloading,
                    });
                }
            }
            Some(Err(e)) => {
                let _ = progress_tx.send(DownloadProgress {
                    bytes_downloaded,
                    total_bytes: total_expected,
                    bytes_per_second: 0.0,
                    progress_percentage: calculate_percentage(bytes_downloaded, total_expected),
                    is_resumable,
                    etag,
                    state: DownloadState::Failed,
                });
                return Err(e);
            }
            None => {
                // Fim do stream atingido (Download concluído)
                break;
            }
        }
    }

    file.flush().await?;
    file.sync_all().await?;
    drop(file);

    // Renomeia o arquivo temporário de forma atômica para o destino final
    tokio::fs::rename(&temp_path, &options.destination_path).await?;

    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded,
        total_bytes: total_expected.or(Some(bytes_downloaded)),
        bytes_per_second: 0.0,
        progress_percentage: Some(100.0),
        is_resumable,
        etag,
        state: DownloadState::Completed,
    });

    Ok(())
}

fn calculate_percentage(current: u64, total: Option<u64>) -> Option<f32> {
    total.map(|tot| {
        if tot == 0 {
            100.0
        } else {
            ((current as f64 / tot as f64) * 100.0) as f32
        }
    })
}

fn parse_total_from_content_range(val: Option<&http::HeaderValue>) -> Option<u64> {
    val.and_then(|v| v.to_str().ok()).and_then(|s| {
        // Exemplo de cabeçalho: "bytes 500-999/1234" -> 1234
        s.split('/').nth(1).and_then(|tot| tot.parse::<u64>().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_options_temp_path() {
        let dest = PathBuf::from("downloads/archive.tar.gz");
        let opts = DownloadOptions::new(Url::parse("https://example.com/archive.tar.gz").unwrap(), dest);
        assert_eq!(
            opts.temp_path(),
            PathBuf::from("downloads/archive.tar.gz.albedodownload")
        );
    }

    #[test]
    fn test_calculate_percentage() {
        assert_eq!(calculate_percentage(50, Some(100)), Some(50.0));
        assert_eq!(calculate_percentage(100, Some(100)), Some(100.0));
        assert_eq!(calculate_percentage(50, None), None);
    }

    #[test]
    fn test_parse_total_from_content_range() {
        let val = http::HeaderValue::from_static("bytes 0-499/12345");
        assert_eq!(parse_total_from_content_range(Some(&val)), Some(12345));

        let val_wildcard = http::HeaderValue::from_static("bytes 0-499/*");
        assert_eq!(parse_total_from_content_range(Some(&val_wildcard)), None);
    }
}
