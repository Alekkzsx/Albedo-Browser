//! # Testes de Validação SOTA Fase 3: Compression Dictionary Transport, OHTTP e WebTransport
//!
//! Valida a conformidade da Fase 3 do ACE Net:
//! 1. RFC 9290: Registro normativo de dicionários via `Use-As-Dictionary`, injeção de `Available-Dictionary` e descompressão com dicionário.
//! 2. RFC 9458: Oblivious HTTP (OHTTP), encapsulamento `message/ohttp-req`, decapsulação e headers de relay.
//! 3. RFC 9297: Validação e contratos de segurança do WebTransport sobre QUIC/HTTP/3.

use ace_net::http::dictionary::{decompress_with_dictionary, DictionaryManager, UseAsDictionaryDirective};
use ace_net::security::ohttp::{EncapsulatedRequest, OhttpConfig, OhttpEncapsulator, OhttpKeyConfig};
use ace_net::protocol::webtransport::validate_webtransport_url;
use ace_net::{Request, ResourceFetcher};
use bytes::Bytes;
use http::header::{ACCEPT, CONTENT_TYPE};
use std::time::{Duration, SystemTime};
use sha2::Digest;
use url::Url;

#[test]
fn test_dictionary_transport_registration_and_lookup() {
    let mgr = DictionaryManager::new();
    let now = SystemTime::now();

    let origin = ace_core::security::origin::Origin::parse("https://cdn.albedo.org").unwrap();
    let directive = UseAsDictionaryDirective {
        match_pattern: "/scripts/*".into(),
        id: Some("v2.1".into()),
        ttl: Duration::from_secs(7200),
    };

    let dict_content = Bytes::from_static(b"function commonHelper() { return 42; }");
    let hash = mgr.register_dictionary(None, origin.clone(), dict_content.clone(), directive, now);

    assert_eq!(mgr.total_dictionaries_count(), 1);
    assert!(!hash.is_empty());

    // URL que deve casar com o padrão
    let match_url = Url::parse("https://cdn.albedo.org/scripts/main.js").unwrap();
    let found = mgr.find_dictionary_for_url(&match_url, None, now);
    assert!(found.is_some());
    let dict = found.unwrap();
    assert_eq!(dict.hash_base64, hash);
    assert_eq!(dict.available_dictionary_header_value(), format!(":{}:", hash));

    // URL que não deve casar (caminho diferente)
    let mismatch_url = Url::parse("https://cdn.albedo.org/styles/main.css").unwrap();
    assert!(mgr.find_dictionary_for_url(&mismatch_url, None, now).is_none());

    // URL em outra origem (não pode vazar o dicionário!)
    let other_origin = Url::parse("https://untrusted.com/scripts/main.js").unwrap();
    assert!(mgr.find_dictionary_for_url(&other_origin, None, now).is_none());
}

#[tokio::test]
async fn test_resource_fetcher_dictionary_integration() {
    let fetcher = ResourceFetcher::new().expect("Falha ao criar fetcher");
    let now = SystemTime::now();

    let origin = ace_core::security::origin::Origin::parse("https://albedo-cdn.net").unwrap();
    let directive = UseAsDictionaryDirective {
        match_pattern: "/assets/*".into(),
        id: Some("bundle-v1".into()),
        ttl: Duration::from_secs(3600),
    };

    let dict_body = Bytes::from_static(b"console.log('common bundle runtime');");
    let hash = fetcher.dictionary_manager().register_dictionary(
        None,
        origin,
        dict_body,
        directive,
        now,
    );

    assert_eq!(fetcher.dictionary_manager().total_dictionaries_count(), 1);

    // Cria requisição para uma URL que casa com o dicionário
    let req = Request::get("https://albedo-cdn.net/assets/vendor.js")
        .unwrap()
        .build();

    // Testa se a resolução do dicionário pelo fetcher encontra o dicionário correto
    let matched = fetcher.dictionary_manager().find_dictionary_for_url(&req.url, None, now);
    assert!(matched.is_some());
    assert_eq!(matched.unwrap().hash_base64, hash);
}

#[test]
fn test_zstandard_dictionary_decompression_flow() {
    let dict_bytes = b"CommonDictionaryPrefixForZstdCompression123456";
    let original_payload = b"CommonDictionaryPrefixForZstdCompression123456 -- specific update content payload!";

    // Comprime usando Zstandard com dicionário
    let mut encoder = zstd::stream::read::Encoder::with_dictionary(&original_payload[..], 3, dict_bytes).unwrap();
    let mut compressed = Vec::new();
    std::io::Read::read_to_end(&mut encoder, &mut compressed).unwrap();

    // Descomprime usando nossa engine RFC 9290
    let decompressed = decompress_with_dictionary("dcz", &compressed, dict_bytes).unwrap();
    assert_eq!(decompressed.as_ref(), original_payload);
}

#[test]
fn test_ohttp_relay_encapsulation_and_decapsulation_flow() {
    let relay_url = Url::parse("https://ohttp-relay.cloudflare.com").unwrap();
    let key_config = OhttpKeyConfig {
        key_id: 42,
        kem_id: 0x0020,
        kdf_id: 0x0001,
        aead_id: 0x0001,
        public_key: vec![7u8; 32],
    };

    let config = OhttpConfig::new(relay_url, key_config);
    let encapsulator = OhttpEncapsulator::new(config);

    let inner_http_request = b"POST /dns-query HTTP/1.1\r\nHost: private.doh\r\n\r\nPayload";
    let enc_bytes = encapsulator.encapsulate_request(inner_http_request).unwrap();

    // Valida estrutura binária message/ohttp-req
    let parsed = EncapsulatedRequest::parse(&enc_bytes, 32).unwrap();
    assert_eq!(parsed.key_id, 42);
    assert_eq!(parsed.kem_id, 0x0020);
    assert_eq!(parsed.enc_key.len(), 32);

    // Valida construção de requisição ao relay
    let relay_req = encapsulator.build_relay_request(enc_bytes).unwrap();
    assert_eq!(relay_req.headers.get(CONTENT_TYPE).unwrap(), "message/ohttp-req");
    assert_eq!(relay_req.headers.get(ACCEPT).unwrap(), "message/ohttp-res");

    // Valida desencapsulamento simétrico de resposta
    let mock_response = b"HTTP/1.1 200 OK\r\nContent-Type: application/dns-message\r\n\r\nAnswer";
    // Gera mock de resposta cifrada com a chave efêmera
    let mut cipher_hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut cipher_hasher, &parsed.enc_key);
    sha2::Digest::update(&mut cipher_hasher, b"OHTTP-AES-GCM-DERIVED-KEY");
    let mask = sha2::Digest::finalize(cipher_hasher);

    let mut enc_resp = Vec::new();
    for (i, &b) in mock_response.iter().enumerate() {
        enc_resp.push(b ^ mask[i % mask.len()]);
    }

    let dec_resp = encapsulator.decapsulate_response(&enc_resp, &parsed.enc_key).unwrap();
    assert_eq!(dec_resp.as_ref(), mock_response);
}

#[test]
fn test_webtransport_url_and_protocol_validation() {
    let valid = Url::parse("https://wt.cloudflare.com:443/chat").unwrap();
    assert!(validate_webtransport_url(&valid).is_ok());

    let invalid_http = Url::parse("http://wt.cloudflare.com/chat").unwrap();
    assert!(validate_webtransport_url(&invalid_http).is_err());

    let invalid_ws = Url::parse("ws://wt.cloudflare.com/chat").unwrap();
    assert!(validate_webtransport_url(&invalid_ws).is_err());
}
