//! # Testes de Integração SOTA: Fase 5 - Downloads Resumíveis e Proxies Enterprise / SOCKS5
//!
//! Valida o motor de downloads com suporte a retomada (`Range: bytes=N-`), integridade
//! em disco via arquivo `.albedodownload`, telemetria de progresso em tempo real
//! e conexões seguras por tunelamento HTTP CONNECT e SOCKS5 (RFC 1928 com Zero DNS Leak).

use ace_net::engine::download::{start_download, DownloadOptions, DownloadState, DOWNLOAD_TEMP_EXTENSION};
use ace_net::transport::proxy::{
    connect_http_connect_tunnel, connect_socks5_tunnel, ProxyAuth, ProxyBypassList, ProxyConfig,
};
use ace_net::create_default_fetcher;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

#[tokio::test]
async fn test_resumable_download_complete_and_atomic_rename() {
    let test_dir = std::env::temp_dir().join(format!("albedo_dl_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = tokio::fs::create_dir_all(&test_dir).await;
    let final_dest = test_dir.join("test_file.bin");

    // Servidor mock que serve arquivo completo de 128 bytes
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        let payload = vec![0x42u8; 128];
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: 128\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n"
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(&payload).await.unwrap();
        socket.flush().await.unwrap();
    });

    let fetcher = Arc::new(create_default_fetcher().unwrap());
    let url = Url::parse(&format!("http://{}/download", local_addr)).unwrap();
    let options = DownloadOptions::new(url, &final_dest);

    let (session, handle) = start_download(fetcher, options);
    let result = handle.await.unwrap();
    assert!(result.is_ok(), "Download worker falhou: {:?}", result);

    server_task.await.unwrap();

    // Validações pós-download:
    // 1. Arquivo final deve existir com 128 bytes
    assert!(final_dest.exists(), "Arquivo consolidado deve existir");
    let content = tokio::fs::read(&final_dest).await.unwrap();
    assert_eq!(content.len(), 128);
    assert_eq!(content[0], 0x42);

    // 2. Arquivo temporário .albedodownload deve ter sido renomeado (não existir mais)
    let temp_file = test_dir.join(format!("test_file.bin.{}", DOWNLOAD_TEMP_EXTENSION));
    assert!(!temp_file.exists(), "Arquivo temporário deve ter sido renomeado");

    // 3. Telemetria
    let progress = session.progress();
    assert_eq!(progress.bytes_downloaded, 128);
    assert_eq!(progress.state, DownloadState::Completed);
    assert_eq!(progress.progress_percentage, Some(100.0));

    // Cleanup
    let _ = tokio::fs::remove_dir_all(&test_dir).await;
}

#[tokio::test]
async fn test_resumable_download_partial_resume_206() {
    let test_dir = std::env::temp_dir().join(format!("albedo_dl_resume_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = tokio::fs::create_dir_all(&test_dir).await;
    let final_dest = test_dir.join("resume_target.dat");

    // Prepara arquivo temporário existente com 64 bytes prévios (de um download pausado)
    let temp_file = test_dir.join(format!("resume_target.dat.{}", DOWNLOAD_TEMP_EXTENSION));
    let initial_data = vec![0x11u8; 64];
    tokio::fs::write(&temp_file, &initial_data).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        // Verifica que o cliente enviou o cabeçalho Range solicitando a partir do offset 64
        assert!(req_str.contains("Range: bytes=64-") || req_str.contains("range: bytes=64-"));

        // Servidor responde com 206 Partial Content entregando os 64 bytes restantes (total 128)
        let remaining_data = vec![0x22u8; 64];
        let response = format!(
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 64-127/128\r\nContent-Length: 64\r\nConnection: close\r\n\r\n"
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(&remaining_data).await.unwrap();
        socket.flush().await.unwrap();
    });

    let fetcher = Arc::new(create_default_fetcher().unwrap());
    let url = Url::parse(&format!("http://{}/resumable", local_addr)).unwrap();
    let mut options = DownloadOptions::new(url, &final_dest);
    options.allow_resume = true;

    let (_session, handle) = start_download(fetcher, options);
    let result = handle.await.unwrap();
    assert!(result.is_ok());

    server_task.await.unwrap();

    // Valida integridade do arquivo resultante: 64 bytes de 0x11 seguidos por 64 bytes de 0x22
    assert!(final_dest.exists());
    let full_content = tokio::fs::read(&final_dest).await.unwrap();
    assert_eq!(full_content.len(), 128);
    assert_eq!(&full_content[..64], &initial_data[..]);
    assert_eq!(&full_content[64..], &vec![0x22u8; 64][..]);

    let _ = tokio::fs::remove_dir_all(&test_dir).await;
}

#[tokio::test]
async fn test_socks5_rfc1928_zero_dns_leak_tunnel() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // 1. Handshake de Métodos: Lê [VER: 0x05, NMETHODS, ...]
        let mut ver_buf = [0u8; 3];
        socket.read_exact(&mut ver_buf).await.unwrap();
        assert_eq!(ver_buf[0], 0x05); // SOCKS5
        assert_eq!(ver_buf[2], 0x00); // Suporte a NO AUTH

        // Servidor responde com [0x05, 0x00] (Versão 5, NO AUTH)
        socket.write_all(&[0x05, 0x00]).await.unwrap();
        socket.flush().await.unwrap();

        // 2. Requisição de Conexão: Lê VER(5), CMD(1), RSV(0), ATYP
        let mut req_header = [0u8; 4];
        socket.read_exact(&mut req_header).await.unwrap();
        assert_eq!(req_header[0], 0x05); // VER 5
        assert_eq!(req_header[1], 0x01); // CMD 1 (CONNECT)
        assert_eq!(req_header[3], 0x03); // ATYP 3 = DOMAINNAME (Zero DNS Leak comprovado!)

        // Lê tamanho do domínio e nome textual
        let mut domain_len = [0u8; 1];
        socket.read_exact(&mut domain_len).await.unwrap();
        let mut domain_buf = vec![0u8; domain_len[0] as usize];
        socket.read_exact(&mut domain_buf).await.unwrap();
        let target_host = String::from_utf8(domain_buf).unwrap();
        assert_eq!(target_host, "target.privacy.org");

        // Lê a porta (2 bytes big-endian)
        let mut port_buf = [0u8; 2];
        socket.read_exact(&mut port_buf).await.unwrap();
        let port = u16::from_be_bytes(port_buf);
        assert_eq!(port, 8443);

        // Servidor envia REP 0x00 (Sucesso)
        let resp = [0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0x20, 0xFB];
        socket.write_all(&resp).await.unwrap();
        socket.flush().await.unwrap();

        // 3. Testa tráfego no túnel estabelecido
        let mut echo_buf = [0u8; 5];
        socket.read_exact(&mut echo_buf).await.unwrap();
        assert_eq!(&echo_buf, b"HELLO");
        socket.write_all(b"WORLD").await.unwrap();
        socket.flush().await.unwrap();
    });

    // Cliente conectando através do túnel SOCKS5 com endereço de domínio
    let mut client_tunnel = connect_socks5_tunnel(proxy_addr, None, "target.privacy.org", 8443)
        .await
        .expect("Túnel SOCKS5 deve ser estabelecido com sucesso");

    client_tunnel.write_all(b"HELLO").await.unwrap();
    client_tunnel.flush().await.unwrap();

    let mut recv_buf = [0u8; 5];
    client_tunnel.read_exact(&mut recv_buf).await.unwrap();
    assert_eq!(&recv_buf, b"WORLD");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_http_connect_tunnel_with_basic_auth() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        // Valida que o comando é CONNECT para o destino corporativo com autorização Basic
        assert!(req_str.starts_with("CONNECT internal.bank.corp:443 HTTP/1.1"));
        assert!(req_str.contains("Proxy-Authorization: Basic YWRtaW46c2VjcmV0MTIz"));

        // Servidor proxy retorna 200 Connection Established
        let resp = "HTTP/1.1 200 Connection Established\r\n\r\n";
        socket.write_all(resp.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();

        // Eco bidirecional
        let mut msg = [0u8; 4];
        socket.read_exact(&mut msg).await.unwrap();
        assert_eq!(&msg, b"PING");
        socket.write_all(b"PONG").await.unwrap();
    });

    let auth = ProxyAuth::new("admin", "secret123");
    let mut tunnel = connect_http_connect_tunnel(proxy_addr, Some(&auth), "internal.bank.corp", 443)
        .await
        .expect("HTTP CONNECT deve estabelecer túnel com sucesso");

    tunnel.write_all(b"PING").await.unwrap();
    tunnel.flush().await.unwrap();

    let mut reply = [0u8; 4];
    tunnel.read_exact(&mut reply).await.unwrap();
    assert_eq!(&reply, b"PONG");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_proxy_bypass_list_evaluation() {
    let mut bypass = ProxyBypassList::new();
    bypass.add_rule("*.internal.lan");
    bypass.add_rule("192.168.0.1");

    assert!(bypass.should_bypass("localhost"));
    assert!(bypass.should_bypass("127.0.0.1"));
    assert!(bypass.should_bypass("printer.local"));
    assert!(bypass.should_bypass("intranet.internal.lan"));
    assert!(bypass.should_bypass("192.168.0.1"));

    assert!(!bypass.should_bypass("google.com"));
    assert!(!bypass.should_bypass("lan.com"));
}

#[tokio::test]
async fn test_resource_fetcher_proxy_configuration_access() {
    let fetcher = create_default_fetcher().unwrap();
    assert_eq!(fetcher.proxy(), ProxyConfig::Direct);

    let dummy_addr = "127.0.0.1:8080".parse().unwrap();
    fetcher.set_proxy(ProxyConfig::http(dummy_addr, None));
    match fetcher.proxy() {
        ProxyConfig::Http { proxy_addr, auth } => {
            assert_eq!(proxy_addr, dummy_addr);
            assert!(auth.is_none());
        }
        other => panic!("Esperado ProxyConfig::Http, obtido {:?}", other),
    }

    let mut bypass = ProxyBypassList::new();
    bypass.add_rule("test.corp");
    fetcher.set_proxy_bypass_list(bypass);
    assert!(fetcher.proxy_bypass_list().should_bypass("test.corp"));
}
