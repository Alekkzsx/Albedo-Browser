//! # Testes de Conformidade RFC 9111 para o HTTP Cache
//!
//! Cobre cálculo de frescor, controle de idade, revalidação condicional (ETag/304),
//! invalidação de métodos não seguros e particionamento por Network Isolation Key (NIK).

use ace_core::security::origin::Origin;
use ace_net::cache::{CacheEntry, HttpCache, NetworkIsolationKey};
use bytes::Bytes;
use http::header::{CACHE_CONTROL, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use std::time::{Duration, SystemTime};
use url::Url;

#[test]
fn test_cache_entry_max_age_freshness() {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=7200"));
    headers.insert(ETAG, HeaderValue::from_static("\"v1.0.0\""));

    let now = SystemTime::now();
    let url = Url::parse("https://cdn.albedo.org/engine.wasm").unwrap();
    let entry = CacheEntry::new(
        url.clone(),
        StatusCode::OK,
        headers,
        Bytes::from_static(b"\x00asm\x01\x00\x00\x00"),
        now,
        now,
    );

    assert_eq!(entry.freshness_lifetime(), Duration::from_secs(7200));
    assert!(entry.is_fresh(now));
    assert!(entry.is_fresh(now + Duration::from_secs(3600)));
    assert!(!entry.is_fresh(now + Duration::from_secs(7201)));
}

#[test]
fn test_cache_entry_no_store_is_not_cacheable() {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

    assert!(!CacheEntry::is_cacheable(&Method::GET, StatusCode::OK, &headers));
}

#[test]
fn test_cache_entry_private_is_not_cacheable() {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, max-age=3600"));

    assert!(!CacheEntry::is_cacheable(&Method::GET, StatusCode::OK, &headers));
}

#[test]
fn test_cache_conditional_revalidation_generation() {
    let mut headers = HeaderMap::new();
    headers.insert(ETAG, HeaderValue::from_static("\"etag-abc-123\""));
    headers.insert(LAST_MODIFIED, HeaderValue::from_static("Wed, 01 Jan 2026 00:00:00 GMT"));

    let now = SystemTime::now();
    let url = Url::parse("https://example.com/api/data").unwrap();
    let entry = CacheEntry::new(url, StatusCode::OK, headers, Bytes::from_static(b"{\"ok\":true}"), now, now);

    let cond_headers = entry.conditional_headers();
    assert_eq!(cond_headers.get(IF_NONE_MATCH).unwrap(), "\"etag-abc-123\"");
    assert_eq!(cond_headers.get(IF_MODIFIED_SINCE).unwrap(), "Wed, 01 Jan 2026 00:00:00 GMT");
}

#[test]
fn test_cache_update_from_304() {
    let mut headers = HeaderMap::new();
    headers.insert(ETAG, HeaderValue::from_static("\"v1\""));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=60"));

    let now = SystemTime::now();
    let url = Url::parse("https://example.com/asset.js").unwrap();
    let mut entry = CacheEntry::new(url, StatusCode::OK, headers, Bytes::from_static(b"console.log('hi');"), now, now);

    let mut response_304_headers = HeaderMap::new();
    response_304_headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=3600"));
    response_304_headers.insert(ETAG, HeaderValue::from_static("\"v2\""));

    let later = now + Duration::from_secs(65);
    entry.update_from_304(&response_304_headers, later);

    assert_eq!(entry.headers.get(ETAG).unwrap(), "\"v2\"");
    assert_eq!(entry.headers.get(CACHE_CONTROL).unwrap(), "max-age=3600");
    assert_eq!(entry.body.as_ref(), b"console.log('hi');");
}

#[test]
fn test_cache_partitioning_via_nik() {
    let cache = HttpCache::new(1024 * 1024);
    let url = Url::parse("https://shared-cdn.com/lib.js").unwrap();
    let now = SystemTime::now();

    let top1 = Origin::parse("https://site-a.com").unwrap();
    let top2 = Origin::parse("https://site-b.com").unwrap();
    let frame = Origin::parse("https://shared-cdn.com").unwrap();

    let nik1 = NetworkIsolationKey::new(top1, frame.clone());
    let nik2 = NetworkIsolationKey::new(top2, frame);

    let entry1 = CacheEntry::new(url.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from_static(b"code_for_a"), now, now);

    cache.put(Some(&nik1), url.clone(), entry1);

    // Site A deve ter cache hit
    let hit_a = cache.get(Some(&nik1), &url);
    assert!(hit_a.is_some());
    assert_eq!(hit_a.unwrap().body.as_ref(), b"code_for_a");

    // Site B NÃO deve ter cache hit (isolamento completo de estado de rede)
    let hit_b = cache.get(Some(&nik2), &url);
    assert!(hit_b.is_none());
}

#[test]
fn test_cache_lru_capacity_and_stats() {
    let cache = HttpCache::new(1000); // 1000 bytes no máximo

    let now = SystemTime::now();
    let url1 = Url::parse("https://example.com/1").unwrap();
    let url2 = Url::parse("https://example.com/2").unwrap();

    // 100 bytes payload: 100 + 256 = 356 bytes cada. 2 x 356 = 712 bytes <= 1000 bytes
    let payload = vec![0xAA; 100];
    let e1 = CacheEntry::new(url1.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from(payload.clone()), now, now);
    let e2 = CacheEntry::new(url2.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from(payload), now, now);

    cache.put(None, url1.clone(), e1);
    cache.put(None, url2.clone(), e2);

    assert_eq!(cache.stats.evictions.load(std::sync::atomic::Ordering::Relaxed), 0);

    // Inserir um terceiro item força o despejo de url1 (712 + 356 = 1068 > 1000)
    let url3 = Url::parse("https://example.com/3").unwrap();
    let e3 = CacheEntry::new(url3.clone(), StatusCode::OK, HeaderMap::new(), Bytes::from(vec![0xBB; 100]), now, now);
    cache.put(None, url3.clone(), e3);

    assert_eq!(cache.stats.evictions.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert!(cache.get(None, &url1).is_none());
    assert!(cache.get(None, &url3).is_some());
}
