//! # Bateria de Testes SOTA Fase 1 — Preconnect, Resource Hints & Persistência de Sessão
//!
//! Valida:
//! 1. Persistência atômica e restauração de políticas HSTS dinâmicas em disco.
//! 2. Persistência atômica e restauração de rotas Alt-Svc (`persist=1`) em disco.
//! 3. Persistência e restauração unificada de sessão (Cookies, HSTS, Alt-Svc) via `ResourceFetcher`.
//! 4. Criação, extração de host e despacho de `ResourceHint` (`DnsPrefetch`, `Preconnect`, `Preload`).
//! 5. Conversão normativa de `EarlyHint` (RFC 8288 / 103 Early Hints) para `ResourceHint`.
//! 6. Bloqueio de segurança PNA (Private Network Access) em preconnect especulativo.

use ace_net::cache::partition::NetworkIsolationKey;
use ace_net::cookie::Cookie;
use ace_net::engine::fetcher::ResourceFetcher;
use ace_net::http::hints::EarlyHint;
use ace_net::protocol::alt_svc::{parse_alt_svc, AltSvcRegistry};
use ace_net::security::hsts::HstsStore;
use ace_net::telemetry::hints::ResourceHint;
use ace_core::security::origin::Origin;
use http::HeaderValue;
use smol_str::SmolStr;
use std::time::{Duration, SystemTime};
use url::Url;

#[test]
fn test_hsts_disk_persistence() {
    let store = HstsStore::new();
    let now = SystemTime::now();

    let val = HeaderValue::from_static("max-age=86400; includeSubDomains");
    store.update_from_header("secure.albedo.app", &val, now);

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("hsts.json");

    store.save_to_file(&path).expect("Falha ao salvar HSTS em disco");
    assert!(path.exists());

    let restored_store = HstsStore::new();
    let loaded_count = restored_store.load_from_file(&path).expect("Falha ao carregar HSTS do disco");
    assert_eq!(loaded_count, 1);
    assert!(restored_store.should_upgrade("secure.albedo.app", now));
    assert!(restored_store.should_upgrade("sub.secure.albedo.app", now));
}

#[test]
fn test_alt_svc_disk_persistence() {
    let registry = AltSvcRegistry::new();
    let now = SystemTime::now();

    // h3=":443"; ma=86400; persist=1 deve persistir.
    // h2=":8443"; ma=3600 sem persist=1 deve ser descartado ao persistir.
    let records = parse_alt_svc("h3=\":443\"; ma=86400; persist=1, h2=\":8443\"; ma=3600", now);
    registry.insert(None, "fast-edge.net", records);

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("altsvc.json");

    registry.save_to_file(&path).expect("Falha ao salvar Alt-Svc em disco");
    assert!(path.exists());

    let restored_registry = AltSvcRegistry::new();
    let count = restored_registry.load_from_file(&path).expect("Falha ao carregar Alt-Svc do disco");
    assert_eq!(count, 1);

    let alts = restored_registry.get_alternatives(None, "fast-edge.net", now);
    assert_eq!(alts.len(), 1);
    assert_eq!(alts[0].protocol_id.as_str(), "h3");
    assert_eq!(alts[0].port, 443);
    assert!(alts[0].persist);
}

#[tokio::test]
async fn test_unified_session_persistence_via_resource_fetcher() {
    let temp_dir = tempfile::tempdir().unwrap();
    let session_path = temp_dir.path().to_path_buf();
    let now = SystemTime::now();

    // 1. Instancia o Fetcher original e popula Cookies, HSTS e Alt-Svc
    {
        let fetcher = ResourceFetcher::with_cache_and_disk(1024 * 1024, Some(session_path.clone()))
            .expect("Falha ao instanciar ResourceFetcher");

        // Insere Cookie persistente com validade futura
        let cookie = Cookie {
            name: "auth_token".into(),
            value: "xyz987654".into(),
            domain: "portal.example.com".into(),
            path: "/".into(),
            expires_at: Some(now + Duration::from_secs(86400)),
            secure: true,
            http_only: true,
            same_site: ace_net::cookie::SameSite::Lax,
            partition_key: None,
            created_at: now,
            last_accessed: now,
        };
        fetcher.cookie_jar().store_cookie(cookie);

        // Insere política HSTS
        let hsts_hdr = HeaderValue::from_static("max-age=60000; includeSubDomains");
        fetcher.hsts_store().update_from_header("portal.example.com", &hsts_hdr, now);

        // Insere rota Alt-Svc persistente
        let alt_records = parse_alt_svc("h3=\":443\"; ma=86400; persist=1", now);
        fetcher.alt_svc_registry().insert(None, "portal.example.com", alt_records);

        // Salva a sessão no disco
        fetcher.save_session_to_disk().expect("Falha ao salvar sessão em disco");

        assert!(session_path.join("cookies.txt").exists());
        assert!(session_path.join("hsts.json").exists());
        assert!(session_path.join("altsvc.json").exists());
    }

    // 2. Instancia um novo Fetcher apontando para o mesmo diretório e valida restauração
    {
        let new_fetcher = ResourceFetcher::with_cache_and_disk(1024 * 1024, Some(session_path))
            .expect("Falha ao instanciar segundo ResourceFetcher");

        // Valida cookies restaurados
        let test_url = Url::parse("https://portal.example.com/dashboard").unwrap();
        let cookie_hdr = new_fetcher.cookie_jar().build_cookie_header(
            &test_url,
            None,
            ace_net::http::request::CredentialsMode::SameOrigin,
            true,
            true,
            now,
        );
        assert!(cookie_hdr.is_some());
        assert!(cookie_hdr.unwrap().to_str().unwrap().contains("auth_token=xyz987654"));

        // Valida HSTS restaurado
        assert!(new_fetcher.hsts_store().should_upgrade("portal.example.com", now));
        assert!(new_fetcher.hsts_store().should_upgrade("api.portal.example.com", now));

        // Valida Alt-Svc restaurado
        let alts = new_fetcher.alt_svc_registry().get_alternatives(None, "portal.example.com", now);
        assert_eq!(alts.len(), 1);
        assert_eq!(alts[0].protocol_id.as_str(), "h3");
    }
}

#[tokio::test]
async fn test_resource_hint_execution_and_early_hints_conversion() {
    let fetcher = ResourceFetcher::new().unwrap();

    // 1. Testa conversão de EarlyHint para ResourceHint
    let early_dns = EarlyHint {
        url: Url::parse("https://cdn.photos.com/pic.jpg").unwrap(),
        rel: SmolStr::from("dns-prefetch"),
        as_type: None,
        crossorigin: false,
    };
    let hint_dns = early_dns.to_resource_hint().expect("Deve converter para DnsPrefetch");
    assert!(matches!(hint_dns, ResourceHint::DnsPrefetch(_)));
    assert_eq!(hint_dns.target_host().as_deref(), Some("cdn.photos.com"));

    let early_preconnect = EarlyHint {
        url: Url::parse("https://fonts.gstatic.com").unwrap(),
        rel: SmolStr::from("preconnect"),
        as_type: None,
        crossorigin: true,
    };
    let hint_preconnect = early_preconnect.to_resource_hint().expect("Deve converter para Preconnect");
    assert!(matches!(hint_preconnect, ResourceHint::Preconnect { .. }));
    assert_eq!(hint_preconnect.target_host().as_deref(), Some("fonts.gstatic.com"));

    // 2. Testa despacho de hint através do fetcher
    let res = fetcher.handle_resource_hint(hint_dns, None).await;
    // O DoH tentará resolver cdn.photos.com e terá sucesso (ou falhará graciosamente se sem internet)
    let _ = res;

    // 3. Testa preconnect com esquema inválido
    let ftp_origin = Origin::parse("ftp://ftp.example.com").unwrap();
    let ftp_hint = ResourceHint::preconnect(ftp_origin, false);
    let ftp_res = fetcher.handle_resource_hint(ftp_hint, None).await;
    assert!(ftp_res.is_err(), "Preconnect em esquema não-HTTP/HTTPS deve falhar");
}

#[tokio::test]
async fn test_preconnect_pna_security_blocking() {
    let fetcher = ResourceFetcher::new().unwrap();

    // Origem pública tenta preconnect para um IP privado local
    let public_origin = Origin::parse("https://ecommerce.com").unwrap();
    let private_origin = Origin::parse("http://192.168.1.1").unwrap();

    let nik = NetworkIsolationKey::new(public_origin.clone(), public_origin, false);

    let res = fetcher.execute_preconnect(&private_origin, Some(&nik)).await;
    assert!(res.is_err(), "PNA deve bloquear preconnect de origem pública para IP privado");
}
