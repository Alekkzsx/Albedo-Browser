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
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};
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

/// Representação de uma fatia de download para aceleração paralela multi-stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadChunk {
    pub chunk_index: usize,
    pub start_byte: u64,
    pub end_byte: u64,
}

impl DownloadChunk {
    /// Divide o tamanho total do arquivo em `num_chunks` fatias contíguas.
    pub fn plan_chunks(total_bytes: u64, num_chunks: usize) -> Vec<DownloadChunk> {
        let num_chunks = num_chunks.max(1);
        if total_bytes == 0 {
            return vec![DownloadChunk {
                chunk_index: 0,
                start_byte: 0,
                end_byte: 0,
            }];
        }
        let chunk_size = total_bytes / (num_chunks as u64);
        let mut chunks = Vec::with_capacity(num_chunks);
        let mut start = 0;

        for i in 0..num_chunks {
            let end = if i == num_chunks - 1 {
                total_bytes - 1
            } else {
                start + chunk_size - 1
            };
            chunks.push(DownloadChunk {
                chunk_index: i,
                start_byte: start,
                end_byte: end,
            });
            start = end + 1;
        }

        chunks
    }
}

/// Inicia o processo de download acelerado por fatias concorrentes (Parallel Range Slicing).
///
/// Divide a transferência do arquivo em `num_chunks` conexões paralelas, gravando os dados
/// concorrentemente com seek no arquivo temporário `.albedodownload`.
pub fn start_parallel_download(
    fetcher: Arc<ResourceFetcher>,
    options: DownloadOptions,
    num_chunks: usize,
) -> (DownloadSession, tokio::task::JoinHandle<NetResult<()>>) {
    if num_chunks <= 1 {
        return start_download(fetcher, options);
    }

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
        execute_parallel_download_worker(fetcher, options, num_chunks, cancel_token, is_paused, progress_tx).await
    });

    (session, worker_handle)
}

/// Trabalhador em background responsável pelo fatiamento concorrente em N streams e escrita com seek.
async fn execute_parallel_download_worker(
    fetcher: Arc<ResourceFetcher>,
    options: DownloadOptions,
    num_chunks: usize,
    cancel_token: CancellationToken,
    is_paused: Arc<AtomicBool>,
    progress_tx: watch::Sender<DownloadProgress>,
) -> NetResult<()> {
    let temp_path = options.temp_path();

    if let Some(parent) = options.destination_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    // 1. Sondagem HEAD para obter tamanho e verificar suporte a Range
    let head_req = Request::builder(options.url.clone(), http::Method::HEAD)?
        .destination(RequestDestination::Other)
        .cancellation_token(cancel_token.clone())
        .build();

    let head_resp = match fetcher.fetch(head_req).await {
        Ok(r) => r,
        Err(_) => {
            return execute_download_worker(fetcher, options, cancel_token, is_paused, progress_tx).await;
        }
    };

    let total_bytes = head_resp
        .headers
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    let accepts_ranges = head_resp
        .headers
        .get(ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("bytes"));

    let etag: Option<SmolStr> = head_resp
        .headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(SmolStr::new);

    let total_size = match total_bytes {
        Some(size) if accepts_ranges && size > 1024 * 1024 => size,
        _ => {
            return execute_download_worker(fetcher, options, cancel_token, is_paused, progress_tx).await;
        }
    };

    // 2. Pré-aloca o arquivo temporário
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)
        .await?;
    file.set_len(total_size).await?;
    drop(file);

    let chunks = DownloadChunk::plan_chunks(total_size, num_chunks);
    let downloaded_per_chunk: Arc<parking_lot::Mutex<Vec<u64>>> = Arc::new(parking_lot::Mutex::new(vec![0; chunks.len()]));
    let mut join_set = tokio::task::JoinSet::new();

    for chunk in chunks {
        let fetcher_c = fetcher.clone();
        let options_c = options.clone();
        let temp_path_c = temp_path.clone();
        let cancel_c = cancel_token.clone();
        let downloaded_map = downloaded_per_chunk.clone();
        let progress_sender = progress_tx.clone();
        let etag_c = etag.clone();

        join_set.spawn(async move {
            let mut req_builder = Request::get(options_c.url.clone())?
                .destination(RequestDestination::Other)
                .streaming(true)
                .cancellation_token(cancel_c.clone());

            let range_val = http::HeaderValue::from_str(&format!("bytes={}-{}", chunk.start_byte, chunk.end_byte))
                .map_err(|e| NetError::HttpProtocolError(e.to_string()))?;
            req_builder = req_builder.header(RANGE, range_val);

            if let Some(ref e) = etag_c {
                if let Ok(hdr) = http::HeaderValue::from_str(e.as_str()) {
                    req_builder = req_builder.header(IF_RANGE, hdr);
                }
            }

            let resp = fetcher_c.fetch(req_builder.build()).await?;
            if resp.status != StatusCode::PARTIAL_CONTENT && resp.status != StatusCode::OK {
                return Err(NetError::HttpProtocolError(format!(
                    "Status inesperado na fatia {}: {}",
                    chunk.chunk_index, resp.status
                )));
            }

            let mut chunk_file = OpenOptions::new()
                .write(true)
                .open(&temp_path_c)
                .await?;

            chunk_file.seek(SeekFrom::Start(chunk.start_byte)).await?;

            let mut stream = resp.body.take_stream().await.ok_or_else(|| {
                NetError::HttpProtocolError("Corpo da resposta sem stream na fatia".into())
            })?;

            while let Some(res) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
                if cancel_c.is_cancelled() {
                    return Err(NetError::Cancelled);
                }
                let bytes = res?;
                chunk_file.write_all(&bytes).await?;

                let mut guard = downloaded_map.lock();
                guard[chunk.chunk_index] += bytes.len() as u64;
                let total_done: u64 = guard.iter().sum();
                drop(guard);

                let _ = progress_sender.send(DownloadProgress {
                    bytes_downloaded: total_done,
                    total_bytes: Some(total_size),
                    bytes_per_second: 0.0,
                    progress_percentage: calculate_percentage(total_done, Some(total_size)),
                    is_resumable: true,
                    etag: etag_c.clone(),
                    state: DownloadState::Downloading,
                });
            }

            chunk_file.flush().await?;
            Ok::<(), NetError>(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let _ = progress_tx.send(DownloadProgress {
                    bytes_downloaded: 0,
                    total_bytes: Some(total_size),
                    bytes_per_second: 0.0,
                    progress_percentage: None,
                    is_resumable: true,
                    etag,
                    state: DownloadState::Failed,
                });
                return Err(e);
            }
            Err(e) => {
                return Err(NetError::HttpProtocolError(e.to_string()));
            }
        }
    }

    tokio::fs::rename(&temp_path, &options.destination_path).await?;

    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded: total_size,
        total_bytes: Some(total_size),
        bytes_per_second: 0.0,
        progress_percentage: Some(100.0),
        is_resumable: true,
        etag,
        state: DownloadState::Completed,
    });

    Ok(())
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
