//! # Cache de DNS Particionado por NIK (`PartitionedDnsCache`)
//!
//! Implementa cache de resolução de nomes particionado por `NetworkIsolationKey` (Double-Keyed DNS Cache),
//! prevenindo ataques de canal lateral de temporização (*DNS Cache Timing Attacks*), com suporte
//! normativo a RFC 8767 (*Serving Stale Data to Improve DNS Resiliency*) e TTL Clamping (5s..86400s).

use crate::cache::NetworkIsolationKey;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Limite mínimo de TTL para evitar saturação de consultas upstream (5 segundos).
pub const MIN_DNS_TTL_CLAMP: Duration = Duration::from_secs(5);
/// Limite máximo de TTL para respeitar mudanças de infraestrutura (24 horas).
pub const MAX_DNS_TTL_CLAMP: Duration = Duration::from_secs(86400);
/// Janela de tolerância para servir dados obsoletos em caso de falha de rede (RFC 8767: 30 segundos).
pub const STALE_SERVE_WINDOW: Duration = Duration::from_secs(30);

/// Entrada de resolução armazenada em cache.
#[derive(Debug, Clone)]
pub struct DnsCacheEntry {
    pub ips: Vec<SocketAddr>,
    pub ech_config: Option<Vec<u8>>,
    pub expires_at: Instant,
    pub stale_until: Instant,
}

impl DnsCacheEntry {
    /// Determina se a entrada ainda está dentro do período de validade (fresh).
    pub fn is_fresh(&self, now: Instant) -> bool {
        now < self.expires_at
    }

    /// Determina se a entrada expirou, mas é elegível para servir stale sob RFC 8767.
    pub fn is_stale_servable(&self, now: Instant) -> bool {
        now >= self.expires_at && now < self.stale_until
    }
}

/// Cache de DNS particionado por tupla dupla `(Option<NetworkIsolationKey>, Hostname)`.
#[derive(Debug, Clone)]
pub struct PartitionedDnsCache {
    entries: Arc<RwLock<FxHashMap<(Option<NetworkIsolationKey>, SmolStr), DnsCacheEntry>>>,
    max_capacity: usize,
}

impl Default for PartitionedDnsCache {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl PartitionedDnsCache {
    /// Cria uma nova instância de cache particionado com capacidade máxima especificada.
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(FxHashMap::default())),
            max_capacity,
        }
    }

    /// Obtém uma entrada de cache válida (fresh) para a partição e host fornecidos.
    pub fn get(
        &self,
        nik: Option<&NetworkIsolationKey>,
        host: &str,
        now: Instant,
    ) -> Option<DnsCacheEntry> {
        let key = (nik.cloned(), SmolStr::new(host.to_ascii_lowercase()));
        let guard = self.entries.read();
        let entry = guard.get(&key)?;
        if entry.is_fresh(now) {
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Obtém uma entrada mesmo se estiver obsoleta (stale), conforme RFC 8767 para resiliência a falhas de upstream.
    pub fn get_stale_fallback(
        &self,
        nik: Option<&NetworkIsolationKey>,
        host: &str,
        now: Instant,
    ) -> Option<DnsCacheEntry> {
        let key = (nik.cloned(), SmolStr::new(host.to_ascii_lowercase()));
        let guard = self.entries.read();
        let entry = guard.get(&key)?;
        if entry.is_stale_servable(now) {
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Insere uma nova resolução no cache com aplicação de TTL Clamping e janela stale da RFC 8767.
    pub fn insert(
        &self,
        nik: Option<&NetworkIsolationKey>,
        host: &str,
        ips: Vec<SocketAddr>,
        ech_config: Option<Vec<u8>>,
        raw_ttl: Duration,
        now: Instant,
    ) {
        let clamped_ttl = raw_ttl.clamp(MIN_DNS_TTL_CLAMP, MAX_DNS_TTL_CLAMP);
        let expires_at = now + clamped_ttl;
        let stale_until = expires_at + STALE_SERVE_WINDOW;

        let entry = DnsCacheEntry {
            ips,
            ech_config,
            expires_at,
            stale_until,
        };

        let key = (nik.cloned(), SmolStr::new(host.to_ascii_lowercase()));
        let mut guard = self.entries.write();

        // Evicção simples se exceder a capacidade
        if guard.len() >= self.max_capacity && !guard.contains_key(&key) {
            if let Some(first_key) = guard.keys().next().cloned() {
                guard.remove(&first_key);
            }
        }

        guard.insert(key, entry);
    }

    /// Limpa todas as entradas do cache de DNS.
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Retorna a quantidade atual de entradas em cache.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Indica se o cache está vazio.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_core::security::origin::Origin;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_partitioned_dns_cache_isolation_and_ttl() {
        let cache = PartitionedDnsCache::new(100);
        let now = Instant::now();

        let nik_a = NetworkIsolationKey::for_top_level(
            Origin::parse("https://bank.example.com").unwrap(),
        );

        let nik_b = NetworkIsolationKey::for_top_level(
            Origin::parse("https://attacker.com").unwrap(),
        );

        let addrs = vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443)];

        // Insere sob NIK_A
        cache.insert(
            Some(&nik_a),
            "api.example.com",
            addrs.clone(),
            None,
            Duration::from_secs(60),
            now,
        );

        // NIK_A consegue ler
        assert!(cache.get(Some(&nik_a), "api.example.com", now).is_some());

        // NIK_B (atacante) NÃO tem acesso ao cache do NIK_A (Anti-Timing Attack)
        assert!(cache.get(Some(&nik_b), "api.example.com", now).is_none());

        // Consulta sem NIK também é isolada
        assert!(cache.get(None, "api.example.com", now).is_none());

        // Após expirar TTL: get normal retorna None, mas stale_fallback funciona até a janela de 30s
        let after_ttl = now + Duration::from_secs(70);
        assert!(cache.get(Some(&nik_a), "api.example.com", after_ttl).is_none());
        assert!(cache.get_stale_fallback(Some(&nik_a), "api.example.com", after_ttl).is_some());
    }
}
