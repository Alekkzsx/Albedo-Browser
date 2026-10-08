//! # Testes de Conectividade Bidirecional (WebSocket) e Observabilidade DevTools (HAR & NetLog)
//!
//! Valida a conformidade da Fase 2 do ACE Net:
//! 1. RFC 6455 WebSockets: Handshake físico e roundtrip bidirecional de mensagens e frames de controle.
//! 2. Rejeição estrita de esquemas inválidos para WebSocket.
//! 3. Exportação e captura em tempo real de logs estruturados Chromium NetLog.
//! 4. Exportação HTTP Archive (HAR 1.2) em memória e persistente em disco.

use ace_net::protocol::websocket::{WebSocketMessage, WebSocketSession};
use ace_net::{NetError, Request, ResourceFetcher};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use url::Url;

#[tokio::test]
async fn test_websocket_local_echo_server_bidirectional_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Falha ao abrir TCP listener local");
    let addr = listener.local_addr().expect("Falha ao obter porta local");

    // Servidor mock WebSocket local rodando sobre tokio-tungstenite
    tokio::spawn(async move {
        if let Ok((tcp_stream, _)) = listener.accept().await {
            if let Ok(mut ws_stream) = tokio_tungstenite::accept_async(tcp_stream).await {
                while let Some(Ok(msg)) = ws_stream.next().await {
                    match msg {
                        tungstenite::Message::Text(t) => {
                            let reply = format!("echo: {}", t);
                            if ws_stream.send(tungstenite::Message::Text(reply)).await.is_err() {
                                break;
                            }
                        }
                        tungstenite::Message::Binary(b) => {
                            if ws_stream.send(tungstenite::Message::Binary(b)).await.is_err() {
                                break;
                            }
                        }
                        tungstenite::Message::Close(_) => {
                            let _ = ws_stream.send(tungstenite::Message::Close(None)).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    let ws_url = Url::parse(&format!("ws://127.0.0.1:{}", addr.port())).unwrap();
    let session = WebSocketSession::connect(ws_url)
        .await
        .expect("Falha ao conectar via WebSocketSession");

    // Teste de mensagem de texto UTF-8
    session
        .send(WebSocketMessage::Text("Albedo SOTA Net".into()))
        .await
        .expect("Falha ao enviar mensagem de texto");
    let received = session.recv().await.expect("Não recebeu mensagem de volta");
    assert_eq!(received, WebSocketMessage::Text("echo: Albedo SOTA Net".into()));

    // Teste de mensagem de buffer binário
    let payload = Bytes::from_static(b"\xde\xad\xbe\xef");
    session
        .send(WebSocketMessage::Binary(payload.clone()))
        .await
        .expect("Falha ao enviar payload binario");
    let received_bin = session.recv().await.expect("Não recebeu frame binário");
    assert_eq!(received_bin, WebSocketMessage::Binary(payload));
}

#[tokio::test]
async fn test_websocket_invalid_scheme_rejection() {
    let bad_url = Url::parse("https://example.com/socket").unwrap();
    let res = WebSocketSession::connect(bad_url).await;
    assert!(res.is_err());
    match res.err().unwrap() {
        NetError::UnsupportedScheme(s) => assert_eq!(s, "https"),
        other => panic!("Erro inesperado recebido: {:?}", other),
    }
}

#[tokio::test]
async fn test_resource_fetcher_in_memory_har_recording() {
    let fetcher = ResourceFetcher::new().expect("Falha ao criar ResourceFetcher");
    let exporter = fetcher.enable_in_memory_har_recording();
    assert_eq!(exporter.entries_count(), 0);

    let req = Request::get("data:text/html;charset=utf-8,<h1>HAR%20Test</h1>")
        .unwrap()
        .build();
    let resp = fetcher.fetch(req).await.expect("Falha ao processar data uri");
    assert_eq!(resp.status, http::StatusCode::OK);

    assert_eq!(exporter.entries_count(), 1);
    let json_har = exporter.export_json();
    assert!(json_har.contains("\"version\": \"1.2\""));
    assert!(json_har.contains("\"creator\""));
    assert!(json_har.contains("data:text/html;charset=utf-8,<h1>HAR%20Test</h1>"));
    assert!(json_har.contains("\"status\": 200"));

    assert!(fetcher.har_exporter().is_some());
    fetcher.disable_har_recording();
    assert!(fetcher.har_exporter().is_none());
}

#[tokio::test]
async fn test_resource_fetcher_netlog_structured_events() {
    let fetcher = ResourceFetcher::new().expect("Falha ao criar ResourceFetcher");
    let req = Request::get("data:text/plain;charset=utf-8,NetLogPayload")
        .unwrap()
        .build();
    let _ = fetcher.fetch(req).await.expect("Falha ao buscar recurso");

    let collector = fetcher.net_log_collector();
    assert!(!collector.is_empty());
    assert!(collector.len() >= 2); // REQUEST_START e RESPONSE_START

    let chrome_netlog = collector.export_chrome_netlog_json();
    assert!(chrome_netlog.contains("\"client\": \"Albedo Browser\""));
    assert!(chrome_netlog.contains("\"netLogVersion\": 1"));
    assert!(chrome_netlog.contains("REQUEST_START"));
    assert!(chrome_netlog.contains("RESPONSE_START"));
}

#[tokio::test]
async fn test_har_disk_export_file() {
    let temp_dir = tempfile::tempdir().expect("Falha ao criar diretório temporário");
    let har_path = temp_dir.path().join("trace.har");

    let exporter = ace_net::HarExporter::new(har_path.clone());
    let req = Request::get("data:text/plain,Hello").unwrap().build();
    let resp = ace_net::Response {
        url: req.url.clone(),
        status: http::StatusCode::OK,
        headers: http::HeaderMap::new(),
        body: ace_net::ResponseBody::Full(Bytes::from_static(b"Hello")),
        mime_type: "text/plain".into(),
        charset: None,
        content_encoding: None,
        retry_after: None,
        content_range: None,
        from_cache: false,
        timing: ace_net::ResponseTiming::default(),
    };

    let entry = ace_net::telemetry::har::HarEntry::from_response(&req, &resp);
    exporter.record_entry(entry);

    // Permite que a gravação assíncrona do arquivo seja finalizada
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert!(har_path.exists());
    let file_content = std::fs::read_to_string(&har_path).expect("Falha ao ler arquivo HAR");
    assert!(file_content.contains("\"version\": \"1.2\""));
    assert!(file_content.contains("data:text/plain,Hello"));
}
