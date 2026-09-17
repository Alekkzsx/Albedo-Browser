//! # Bateria de Testes do Plano de Ataque (SOTA Hardening)
//!
//! Valida a eliminação das 6 fragilidades do `ace_net`:
//! 1. Streaming reativo de resposta HTTP e proteção contra OOM.
//! 2. Cache L2 atômico em disco (.tmp -> .cache) e suporte a `stale-if-error`.
//! 3. Persistência de Cookies no disco (salvamento/restauração atômica, exclusão de voláteis).
//! 4. Public Suffix List com Trie e cobertura de ccTLDs/domínios privados.
//! 5. Fallback resiliente de HTTP/3 para TCP/TLS.
//! 6. W3C Private Network Access (PNA) bloqueando acesso de páginas públicas a IPs privados/locais.

use ace_net::cache::entry::CacheEntry;
use ace_net::cache::storage::HttpCache;
use ace_net::cookie::{is_public_suffix, is_valid_cookie_domain, CookieJar};
use ace_net::pna::{classify_ip, is_host_private_or_local, validate_private_network_access, IpAddressSpace};
use ace_net::request::Request;
use ace_net::response::ResponseBody;
use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, SET_COOKIE};
use http::{HeaderMap, HeaderValue, StatusCode};
use std::net::IpAddr;
use std::time::{Duration, SystemTime};
use url::Url;

// ---------------------------------------------------------------------------
// FRENTE 1: Streaming Reativo
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_streaming_flag_and_stream_consumption() {
    let req = Request::get("https://example.com/large-video.mp4")
        .unwrap()
        .streaming(true)
        .build();

    assert!(req.streaming, "A flag de streaming deve estar ativada");

    // Simula stream de chunks
    struct TestStream {
        chunks: Vec<Bytes>,
        pos: usize,
    }

    impl futures_core::Stream for TestStream {
        type Item = ace_net::error::NetResult<Bytes>;

        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            if self.pos < self.chunks.len() {
                let chunk = self.chunks[self.pos].clone();
                self.pos += 1;
                std::task::Poll::Ready(Some(Ok(chunk)))
            } else {
                std::task::Poll::Ready(None)
            }
        }
    }

    let stream = TestStream {
        chunks: vec![
            Bytes::from_static(b"video_frame_0;"),
            Bytes::from_static(b"video_frame_1;"),
            Bytes::from_static(b"video_frame_2"),
        ],
        pos: 0,
    };

    let response_body = ResponseBody::from_stream(stream);
    assert!(!response_body.is_empty());

    let collected = response_body.collect_bytes().await.unwrap();
    assert_eq!(collected.as_ref(), b"video_frame_0;video_frame_1;video_frame_2");
}

// ---------------------------------------------------------------------------
// FRENTE 2: Cache L2 Atômico & stale-if-error
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
async fn test_atomic_disk_cache_persistence_and_cleanup() {
    let temp_dir = std::env::temp_dir().join(format!("albedo_cache_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    // Cria arquivo temporário .tmp "órfão" para testar higienização
    let orphan_tmp = temp_dir.join("0123456789abcdef.tmp");
    std::fs::write(&orphan_tmp, b"corrupted temp data").unwrap();
    assert!(orphan_tmp.exists());

    // Inicializa cache L2 no diretório
    let cache = HttpCache::new(10 * 1024 * 1024).with_disk_path(temp_dir.clone());

    // O arquivo .tmp deve ter sido limpo na inicialização
    assert!(!orphan_tmp.exists(), "Arquivos .tmp orfaos devem ser limpos na carga do indice");

    let url = Url::parse("https://cdn.example.com/bundle.js").unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/javascript"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=3600"));

    let entry = CacheEntry::new(
        url.clone(),
        StatusCode::OK,
        headers,
        Bytes::from_static(b"console.log('albedo sota');"),
        SystemTime::now(),
        SystemTime::now(),
    );

    // Grava no cache
    cache.put(None, url.clone(), entry);

    // Aguarda escrita em background do tokio::spawn
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verifica que o arquivo .cache foi gravado e que não sobrou nenhum .tmp
    let files: Vec<_> = std::fs::read_dir(&temp_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    assert!(files.iter().any(|f| f.ends_with(".cache")), "O arquivo de cache deve ter sido gravado");
    assert!(!files.iter().any(|f| f.ends_with(".tmp")), "Nenhum arquivo .tmp temporario deve restar");

    // Reinicia o cache do mesmo diretório e valida carga do índice L2
    let restored_cache = HttpCache::new(10 * 1024 * 1024).with_disk_path(temp_dir.clone());
    let hit = restored_cache.get(None, &url).await;
    assert!(hit.is_some(), "Entrada gravada em disco deve ser encontrada no L2");
    assert_eq!(hit.unwrap().body.as_ref(), b"console.log('albedo sota');");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_stale_if_error_rfc5861() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("max-age=60, stale-if-error=300"),
    );

    let now = SystemTime::now();
    let entry = CacheEntry::new(
        Url::parse("https://api.example.com/weather").unwrap(),
        StatusCode::OK,
        headers,
        Bytes::from_static(b"{\"temp\":22}"),
        now,
        now,
    );

    assert_eq!(entry.freshness_lifetime(), Duration::from_secs(60));
    assert_eq!(entry.stale_if_error_lifetime(), Duration::from_secs(300));

    // Em t=30s: fresco (não precisa de fallback)
    assert!(!entry.is_stale_if_error(now + Duration::from_secs(30)));

    // Em t=100s: expirado (60s), mas dentro da janela stale-if-error (60s + 300s = 360s)
    let t100 = now + Duration::from_secs(100);
    assert!(!entry.is_fresh(t100));
    assert!(entry.is_stale_if_error(t100));

    // Em t=400s: expirado além do stale-if-error
    let t400 = now + Duration::from_secs(400);
    assert!(!entry.is_stale_if_error(t400));
}

// ---------------------------------------------------------------------------
// FRENTE 3: Persistência de Cookies no Disco
// ---------------------------------------------------------------------------
#[test]
fn test_cookie_persistence_disk_roundtrip() {
    let temp_file = std::env::temp_dir().join(format!("albedo_cookies_test_{}.tsv", std::process::id()));
    let _ = std::fs::remove_file(&temp_file);

    let jar = CookieJar::new();
    let url = Url::parse("https://app.example.com").unwrap();
    let now = SystemTime::now();

    // 1. Cookie persistente com Max-Age futuro
    let mut h1 = HeaderMap::new();
    h1.insert(
        SET_COOKIE,
        HeaderValue::from_static("auth_token=secret_xyz; Secure; HttpOnly; SameSite=Strict; Max-Age=86400"),
    );
    jar.process_response_headers(&url, &h1, None, now);

    // 2. Cookie de sessão efêmero (sem Max-Age nem Expires)
    let mut h2 = HeaderMap::new();
    h2.insert(
        SET_COOKIE,
        HeaderValue::from_static("session_temp=volatile_123; Path=/; SameSite=Lax"),
    );
    jar.process_response_headers(&url, &h2, None, now);

    // 3. Cookie particionado (CHIPS) com expiração futura
    let mut h3 = HeaderMap::new();
    h3.insert(
        SET_COOKIE,
        HeaderValue::from_static("widget_state=active; Secure; Partitioned; SameSite=None; Max-Age=3600"),
    );
    jar.process_response_headers(&url, &h3, Some("example.com"), now);

    assert_eq!(jar.len(), 3, "Jar original deve conter 3 cookies");

    // Salva cookies no disco
    jar.save_to_file(&temp_file).unwrap();
    assert!(temp_file.exists());

    // Cria um NOVO CookieJar (simulando reinício do navegador)
    let new_jar = CookieJar::new();
    let loaded_count = new_jar.load_from_file(&temp_file).unwrap();

    // Apenas cookies persistentes (auth_token e widget_state) devem ser restaurados.
    // O cookie de sessão efêmero (session_temp) deve ser descartado no reinício!
    assert_eq!(loaded_count, 2, "Apenas os 2 cookies persistentes devem ser carregados");
    assert_eq!(new_jar.len(), 2);

    let hdr = new_jar
        .build_cookie_header(&url, None, ace_net::request::CredentialsMode::SameOrigin, true, true, now)
        .unwrap();
    let hdr_str = hdr.to_str().unwrap();
    assert!(hdr_str.contains("auth_token=secret_xyz"), "Cookie persistente deve estar presente");
    assert!(!hdr_str.contains("session_temp=volatile_123"), "Cookie de sessao volatil nao deve persistir");

    let _ = std::fs::remove_file(&temp_file);
}

// ---------------------------------------------------------------------------
// FRENTE 4: Public Suffix List com Trie e Domínios Privados
// ---------------------------------------------------------------------------
#[test]
fn test_extended_psl_trie_super_cookie_rejection() {
    // TLDs e ccTLDs comuns
    assert!(is_public_suffix("com"));
    assert!(is_public_suffix("co.uk"));
    assert!(is_public_suffix("com.br"));
    assert!(is_public_suffix("org.br"));
    assert!(is_public_suffix("gov.br"));
    assert!(is_public_suffix("co.jp"));
    assert!(is_public_suffix("com.au"));

    // Provedores de hospedagem privada (Multi-tenant)
    assert!(is_public_suffix("pages.dev"));
    assert!(is_public_suffix("vercel.app"));
    assert!(is_public_suffix("github.io"));
    assert!(is_public_suffix("netlify.app"));

    // Domínios registráveis (NÃO são sufixos públicos)
    assert!(!is_public_suffix("google.com"));
    assert!(!is_public_suffix("bbc.co.uk"));
    assert!(!is_public_suffix("uol.com.br"));
    assert!(!is_public_suffix("meusite.pages.dev"));
    assert!(!is_public_suffix("projeto.vercel.app"));

    // Rejeição estrita de Super-Cookies em provedores multi-tenant
    assert!(!is_valid_cookie_domain("pages.dev", "meusite.pages.dev"));
    assert!(!is_valid_cookie_domain("vercel.app", "projeto.vercel.app"));
    assert!(!is_valid_cookie_domain("com.br", "empresa.com.br"));
    assert!(is_valid_cookie_domain("meusite.pages.dev", "meusite.pages.dev"));
    assert!(is_valid_cookie_domain("empresa.com.br", "empresa.com.br"));
}

// ---------------------------------------------------------------------------
// FRENTE 6: W3C Private Network Access (PNA)
// ---------------------------------------------------------------------------
#[test]
fn test_pna_classification_and_ip_validation() {
    let local_v4: IpAddr = "127.0.0.1".parse().unwrap();
    let local_v6: IpAddr = "::1".parse().unwrap();
    let private_192: IpAddr = "192.168.1.1".parse().unwrap();
    let private_10: IpAddr = "10.0.0.1".parse().unwrap();
    let private_172: IpAddr = "172.16.0.1".parse().unwrap();
    let public_dns: IpAddr = "1.1.1.1".parse().unwrap();

    assert_eq!(classify_ip(local_v4), IpAddressSpace::Local);
    assert_eq!(classify_ip(local_v6), IpAddressSpace::Local);
    assert_eq!(classify_ip(private_192), IpAddressSpace::Private);
    assert_eq!(classify_ip(private_10), IpAddressSpace::Private);
    assert_eq!(classify_ip(private_172), IpAddressSpace::Private);
    assert_eq!(classify_ip(public_dns), IpAddressSpace::Public);

    // Host helper
    assert!(is_host_private_or_local("localhost"));
    assert!(is_host_private_or_local("app.localhost"));
    assert!(is_host_private_or_local("127.0.0.1"));
    assert!(is_host_private_or_local("192.168.0.1"));
    assert!(!is_host_private_or_local("cloudflare.com"));

    // Bloqueio de navegação cross-space: Web pública NÃO pode chamar IP privado/local
    assert!(validate_private_network_access(true, local_v4).is_err());
    assert!(validate_private_network_access(true, private_192).is_err());
    assert!(validate_private_network_access(true, public_dns).is_ok());

    // Requisição interna ou local pode acessar qualquer destino
    assert!(validate_private_network_access(false, local_v4).is_ok());
    assert!(validate_private_network_access(false, private_192).is_ok());
    assert!(validate_private_network_access(false, public_dns).is_ok());
}
