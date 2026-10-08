use ace_core::id::RequestId;
use ace_core::security::origin::Origin;
use ace_net::cache::NetworkIsolationKey;
use ace_net::engine::download::{start_parallel_download, DownloadChunk, DownloadOptions};
use ace_net::engine::scheduler::{ResourceScheduler, SchedulerConfig, TabId};
use ace_net::PriorityLevel;
use ace_net::security::captive_portal::{CaptivePortalDetector, CaptivePortalStatus};
use ace_net::transport::dns_cache::{
    PartitionedDnsCache, MAX_DNS_TTL_CLAMP, MIN_DNS_TTL_CLAMP,
};
use ace_net::transport::network_change::{
    NetworkChangeEvent, NetworkConnectionType,
};
use ace_net::transport::client::TransportClient;
use ace_net::ResourceFetcher;
use bytes::Bytes;
use http::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, LOCATION};
use http::{Method, StatusCode};
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::watch;
use url::Url;

// =========================================================================
// 1. Testes de Sensoriamento de Mudança de Rede e Flush de Sockets
// =========================================================================

#[tokio::test]
async fn test_network_change_notifier_and_fetcher_orchestration() {
    let fetcher = Arc::new(ResourceFetcher::new().expect("Fetcher deve inicializar com sucesso"));
    let notifier = fetcher.network_change_notifier();

    assert!(notifier.is_online(), "Fetcher deve iniciar com status online padrão");
    assert_eq!(notifier.connection_type(), NetworkConnectionType::Unknown);

    let mut event_rx = notifier.subscribe();

    // Simula transição para Wi-Fi com mudança de IP e interface
    let wifi_event = NetworkChangeEvent {
        connection_type: NetworkConnectionType::Wifi,
        ip_changed: true,
        dns_changed: true,
        interface_name: Some("wlan0".into()),
    };

    fetcher.handle_network_change(wifi_event.clone()).await;

    // Verifica que o evento foi transmitido no canal broadcast
    let received = event_rx.recv().await.expect("Evento deve ser recebido");
    assert_eq!(received, wifi_event);
    assert_eq!(notifier.connection_type(), NetworkConnectionType::Wifi);
    assert!(notifier.is_online());

    // Simula desconexão total (Offline)
    let offline_event = NetworkChangeEvent {
        connection_type: NetworkConnectionType::None,
        ip_changed: true,
        dns_changed: false,
        interface_name: None,
    };

    fetcher.handle_network_change(offline_event).await;

    let received_offline = event_rx.recv().await.expect("Evento offline deve ser recebido");
    assert_eq!(received_offline.connection_type, NetworkConnectionType::None);
    assert!(!notifier.is_online(), "Deve refletir offline quando tipo de conexão for None");
}

// =========================================================================
// 2. Testes de Detecção de Portal Cativo (Canonical 204 Probe & Interception)
// =========================================================================

#[tokio::test]
async fn test_captive_portal_detector_states() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = hyper_util::rt::TokioIo::new(stream);
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(|req: hyper::Request<hyper::body::Incoming>| async move {
                                let path = req.uri().path();
                                match path {
                                    "/generate_204" => {
                                        let mut resp = hyper::Response::new(Full::new(Bytes::new()));
                                        *resp.status_mut() = StatusCode::NO_CONTENT;
                                        Ok::<_, Infallible>(resp)
                                    }
                                    "/hotel_login" => {
                                        let mut resp = hyper::Response::new(Full::new(Bytes::new()));
                                        *resp.status_mut() = StatusCode::FOUND;
                                        resp.headers_mut().insert(
                                            LOCATION,
                                            http::HeaderValue::from_static("/portal/auth.html"),
                                        );
                                        Ok::<_, Infallible>(resp)
                                    }
                                    "/intercepted_200" => {
                                        let mut resp = hyper::Response::new(Full::new(Bytes::from("<html>Login Required</html>")));
                                        *resp.status_mut() = StatusCode::OK;
                                        resp.headers_mut().insert(
                                            CONTENT_TYPE,
                                            http::HeaderValue::from_static("text/html"),
                                        );
                                        Ok::<_, Infallible>(resp)
                                    }
                                    _ => {
                                        let mut resp = hyper::Response::new(Full::new(Bytes::new()));
                                        *resp.status_mut() = StatusCode::NOT_FOUND;
                                        Ok::<_, Infallible>(resp)
                                    }
                                }
                            }),
                        )
                        .await;
                });
            }
        }
    });

    let transport = TransportClient::new().unwrap();

    // 1. Cenário Online: Sonda 204 responde com No Content
    let probe_url_204 = Url::parse(&format!("http://127.0.0.1:{}/generate_204", port)).unwrap();
    let detector_online = CaptivePortalDetector::with_probe_url(probe_url_204);
    let status_online = detector_online.check_portal(&transport).await.unwrap();
    assert_eq!(status_online, CaptivePortalStatus::Online);
    assert_eq!(detector_online.status(), CaptivePortalStatus::Online);

    // 2. Cenário Interceptado por Redirecionamento 302
    let probe_url_hotel = Url::parse(&format!("http://127.0.0.1:{}/hotel_login", port)).unwrap();
    let detector_hotel = CaptivePortalDetector::with_probe_url(probe_url_hotel);
    let status_hotel = detector_hotel.check_portal(&transport).await.unwrap();
    let expected_auth_url = Url::parse(&format!("http://127.0.0.1:{}/portal/auth.html", port)).unwrap();
    assert_eq!(status_hotel, CaptivePortalStatus::BehindCaptivePortal(expected_auth_url));

    // 3. Cenário Interceptado servindo HTTP 200 OK com tela HTML em vez de 204
    let probe_url_200 = Url::parse(&format!("http://127.0.0.1:{}/intercepted_200", port)).unwrap();
    let detector_200 = CaptivePortalDetector::with_probe_url(probe_url_200.clone());
    let status_200 = detector_200.check_portal(&transport).await.unwrap();
    assert_eq!(status_200, CaptivePortalStatus::BehindCaptivePortal(probe_url_200));
}

// =========================================================================
// 3. Testes de Cache de DNS Particionado por NIK e RFC 8767 Stale Fallback
// =========================================================================

#[test]
fn test_partitioned_dns_cache_isolation_and_rfc8767() {
    let cache = PartitionedDnsCache::new(100);
    let now = Instant::now();

    let nik_bank = NetworkIsolationKey::for_top_level(
        Origin::parse("https://bank.example.com").unwrap(),
    );
    let nik_tracker = NetworkIsolationKey::for_top_level(
        Origin::parse("https://tracker.adtech.com").unwrap(),
    );

    let host = "cdn.shared-assets.com";
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10)), 443);

    // 1. Inserção sob NIK de Bank com TTL raw de 10 segundos
    cache.insert(
        Some(&nik_bank),
        host,
        vec![addr],
        None,
        Duration::from_secs(10),
        now,
    );

    // Consulta com NIK do banco: deve encontrar (Hit isolado)
    let bank_entry = cache.get(Some(&nik_bank), host, now);
    assert!(bank_entry.is_some());
    assert_eq!(bank_entry.unwrap().ips[0], addr);

    // Consulta com NIK de Tracker: DEVE falhar (Miss para isolamento cross-site / Timing Attack Defense)
    let tracker_entry = cache.get(Some(&nik_tracker), host, now);
    assert!(tracker_entry.is_none(), "Cache particionado não deve permitir vazamento cross-origin");

    // Consulta sem NIK: também deve ser isolada
    let unpartitioned_entry = cache.get(None, host, now);
    assert!(unpartitioned_entry.is_none());

    // 2. TTL Clamping: testar limites mínimo e máximo
    // Sub-segundo clampado para MIN_DNS_TTL_CLAMP (5s)
    cache.insert(
        None,
        "clamped.min.com",
        vec![addr],
        None,
        Duration::from_millis(100),
        now,
    );
    let min_entry = cache.get(None, "clamped.min.com", now).unwrap();
    assert_eq!(min_entry.expires_at - now, MIN_DNS_TTL_CLAMP);

    // TTL de 10 dias clampado para MAX_DNS_TTL_CLAMP (86400s)
    cache.insert(
        None,
        "clamped.max.com",
        vec![addr],
        None,
        Duration::from_secs(999_999),
        now,
    );
    let max_entry = cache.get(None, "clamped.max.com", now).unwrap();
    assert_eq!(max_entry.expires_at - now, MAX_DNS_TTL_CLAMP);

    // 3. RFC 8767: Stale Serving
    // Entrada expirada aos 10s:
    let after_expiry = now + Duration::from_secs(12);
    assert!(
        cache.get(Some(&nik_bank), host, after_expiry).is_none(),
        "get comum não deve retornar entradas expiradas"
    );

    // get_stale_fallback deve recuperar enquanto estiver dentro da janela de 30s
    let stale_entry = cache.get_stale_fallback(Some(&nik_bank), host, after_expiry);
    assert!(stale_entry.is_some(), "RFC 8767 deve fornecer stale fallback para resiliência");
    assert_eq!(stale_entry.unwrap().ips[0], addr);

    // Após a janela stale (10s + 30s = 40s):
    let way_past = now + Duration::from_secs(45);
    assert!(cache.get_stale_fallback(Some(&nik_bank), host, way_past).is_none());
}

// =========================================================================
// 4. Testes de Acelerador de Download Concorrente Multi-Fatias (Range Requests)
// =========================================================================

#[tokio::test]
async fn test_parallel_multi_chunk_range_download() {
    let payload_size: usize = 1_500_000; // 1.5 MB (> 1MB threshold para parallel chunking)
    let mut payload = Vec::with_capacity(payload_size);
    for i in 0..payload_size {
        payload.push(((i * 7 + 13) % 251) as u8);
    }
    let payload_arc = Arc::new(payload);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let payload_clone = payload_arc.clone();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let payload_inner = payload_clone.clone();
                let io = hyper_util::rt::TokioIo::new(stream);
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                                let payload_ref = payload_inner.clone();
                                async move {
                                    if req.method() == Method::HEAD {
                                        let mut resp = hyper::Response::new(Full::new(Bytes::new()));
                                        *resp.status_mut() = StatusCode::OK;
                                        resp.headers_mut().insert(
                                            CONTENT_LENGTH,
                                            http::HeaderValue::from_str(&payload_ref.len().to_string()).unwrap(),
                                        );
                                        resp.headers_mut().insert(
                                            ACCEPT_RANGES,
                                            http::HeaderValue::from_static("bytes"),
                                        );
                                        resp.headers_mut().insert(
                                            ETAG,
                                            http::HeaderValue::from_static("\"parallel-test-etag\""),
                                        );
                                        return Ok::<_, Infallible>(resp);
                                    }

                                    // GET com Range header
                                    if let Some(range_header) = req.headers().get(http::header::RANGE) {
                                        let range_str = range_header.to_str().unwrap();
                                        if let Some(spec) = range_str.strip_prefix("bytes=") {
                                            let parts: Vec<&str> = spec.split('-').collect();
                                            let start: usize = parts[0].parse().unwrap();
                                            let end: usize = parts[1].parse().unwrap();

                                            let slice = Bytes::copy_from_slice(&payload_ref[start..=end]);
                                            let mut resp = hyper::Response::new(Full::new(slice));
                                            *resp.status_mut() = StatusCode::PARTIAL_CONTENT;
                                            resp.headers_mut().insert(
                                                CONTENT_RANGE,
                                                http::HeaderValue::from_str(&format!(
                                                    "bytes {}-{}/{}",
                                                    start,
                                                    end,
                                                    payload_ref.len()
                                                ))
                                                .unwrap(),
                                            );
                                            resp.headers_mut().insert(
                                                CONTENT_LENGTH,
                                                http::HeaderValue::from_str(&(end - start + 1).to_string()).unwrap(),
                                            );
                                            return Ok::<_, Infallible>(resp);
                                        }
                                    }

                                    // Fallback GET total
                                    let mut resp = hyper::Response::new(Full::new(Bytes::copy_from_slice(&payload_ref)));
                                    *resp.status_mut() = StatusCode::OK;
                                    Ok::<_, Infallible>(resp)
                                }
                            }),
                        )
                        .await;
                });
            }
        }
    });

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("assembled_download.bin");

    let url = Url::parse(&format!("http://127.0.0.1:{}/bigfile.dat", port)).unwrap();
    let options = DownloadOptions::new(url, dest_file.clone());

    let fetcher = Arc::new(ResourceFetcher::new().unwrap());

    // Valida planejamento de fatias
    let planned_chunks = DownloadChunk::plan_chunks(payload_size as u64, 4);
    assert_eq!(planned_chunks.len(), 4);
    assert_eq!(planned_chunks[0].start_byte, 0);
    assert_eq!(planned_chunks[3].end_byte, (payload_size - 1) as u64);

    // Inicia download paralelo em 4 fatias
    let (_session, worker_handle) = start_parallel_download(fetcher, options, 4);
    let result = worker_handle.await.expect("Worker deve concluir");
    assert!(result.is_ok(), "Download paralelo deve completar com sucesso: {:?}", result.err());

    // Validação da montagem do arquivo final no disco
    assert!(dest_file.exists(), "Arquivo destino deve existir após término");
    let downloaded_bytes = tokio::fs::read(&dest_file).await.unwrap();
    assert_eq!(downloaded_bytes.len(), payload_size, "Tamanho consolidado deve ser idêntico");
    assert_eq!(downloaded_bytes, *payload_arc, "Bytes recebidos em paralelo com seek devem ser idênticos ao original");

    // Validação de remoção do arquivo temporário (.albedodownload)
    let temp_download_file = dest_file.with_extension("albedodownload");
    assert!(!temp_download_file.exists(), "Arquivo temporário deve ter sido renomeado atomicamente");
}

// =========================================================================
// 5. Testes de Repriorização Dinâmica e Background Tab Throttling
// =========================================================================

#[test]
fn test_scheduler_tab_throttling_and_dynamic_reprioritization() {
    let scheduler = ResourceScheduler::new(SchedulerConfig::default());

    let tab_main = TabId(1);
    let tab_background = TabId(2);

    let req_main = RequestId::new();
    let req_bg_1 = RequestId::new();
    let req_bg_2 = RequestId::new();

    let (tx_main, rx_main) = watch::channel(PriorityLevel::High);
    let (tx_bg_1, rx_bg_1) = watch::channel(PriorityLevel::High);
    let (tx_bg_2, rx_bg_2) = watch::channel(PriorityLevel::Medium);

    // Registra streams ativos no agendador
    scheduler.register_active_stream(req_main, tx_main);
    scheduler.register_active_stream(req_bg_1, tx_bg_1);
    scheduler.register_active_stream(req_bg_2, tx_bg_2);

    // Associa requisições com abas
    scheduler.associate_tab(req_main, tab_main, PriorityLevel::High);
    scheduler.associate_tab(req_bg_1, tab_background, PriorityLevel::High);
    scheduler.associate_tab(req_bg_2, tab_background, PriorityLevel::Medium);

    // 1. Aba 2 vai para segundo plano: deve ativar Background Tab Throttling
    scheduler.set_tab_visibility(tab_background, false);

    // Streams da aba 2 devem ser rebaixados imediatamente para Lowest (u=7)
    assert_eq!(*rx_bg_1.borrow(), PriorityLevel::Lowest, "Requisição 1 da aba em background deve ter prioridade Lowest");
    assert_eq!(*rx_bg_2.borrow(), PriorityLevel::Lowest, "Requisição 2 da aba em background deve ter prioridade Lowest");
    assert_eq!(*rx_main.borrow(), PriorityLevel::High, "Aba ativa não deve sofrer alteração");

    // 2. Usuário foca de volta na Aba 2: restauração das prioridades originais
    scheduler.set_tab_visibility(tab_background, true);

    assert_eq!(*rx_bg_1.borrow(), PriorityLevel::High, "Requisição 1 deve ser restaurada para High original");
    assert_eq!(*rx_bg_2.borrow(), PriorityLevel::Medium, "Requisição 2 deve ser restaurada para Medium original");

    // 3. Desassocia requisições ao finalizar
    scheduler.disassociate_tab(req_bg_1, tab_background);
    scheduler.disassociate_tab(req_bg_2, tab_background);
    scheduler.unregister_active_stream(req_bg_1);
    scheduler.unregister_active_stream(req_bg_2);
}
