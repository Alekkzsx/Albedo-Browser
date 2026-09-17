use std::time::{Instant, Duration};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use ace_core::security::origin::Origin;
use crate::http::request::{Method, Request, CredentialsMode};
use http::header::ORIGIN;
use crate::security::cors::{is_safelisted_method, get_non_safelisted_headers};

#[derive(Debug, Clone)]
pub struct PreflightCacheEntry {
    pub expires_at: Instant,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub allow_credentials: bool,
}

pub struct CorsCache {
    entries: RwLock<FxHashMap<String, PreflightCacheEntry>>,
}

impl Default for CorsCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(FxHashMap::default()),
        }
    }
    
    fn make_key(req_origin: &str, target_url: &str) -> String {
        let target_origin = Origin::parse(target_url).unwrap_or_else(|_| Origin::new_opaque()).ascii_serialization();
        format!("{}::{}", req_origin, target_origin)
    }

    pub fn is_cached_and_valid(&self, req: &Request) -> bool {
        let req_origin_str = req.headers.get(ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
        let key = Self::make_key(req_origin_str, req.url.as_str());
        
        let entries = self.entries.read();
        if let Some(entry) = entries.get(&key) {
            if entry.expires_at > Instant::now() {
                if req.credentials == CredentialsMode::Include && !entry.allow_credentials {
                    return false;
                }
                if !entry.allow_methods.contains(&req.method.as_str().to_string()) && !is_safelisted_method(&req.method) {
                    return false;
                }
                let non_safelisted = get_non_safelisted_headers(req);
                for h in non_safelisted {
                    if !entry.allow_headers.iter().any(|ah| ah.eq_ignore_ascii_case(&h)) {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn insert(&self, req_origin: &str, target_url: &str, max_age_secs: u64, allow_methods: Vec<String>, allow_headers: Vec<String>, allow_credentials: bool) {
        let key = Self::make_key(req_origin, target_url);
        let expires_at = Instant::now() + Duration::from_secs(max_age_secs);
        let mut entries = self.entries.write();
        entries.insert(key, PreflightCacheEntry {
            expires_at,
            allow_methods,
            allow_headers,
            allow_credentials,
        });
    }
}
