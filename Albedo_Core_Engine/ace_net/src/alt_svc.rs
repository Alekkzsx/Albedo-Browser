//! # Registro de Serviços Alternativos HTTP — `Alt-Svc` (RFC 7838)
//!
//! Permite que servidores anunciem serviços alternativos mais rápidos (ex: HTTP/3 via QUIC na porta 443)
//! para conexões futuras, eliminando round-trips de descoberta.

use crate::cache::partition::NetworkIsolationKey;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::time::{Duration, SystemTime};

/// Registro individual de um serviço alternativo anunciado pelo servidor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AltSvcRecord {
    /// Identificador do protocolo ALPN alternativo (ex: "h3", "h2").
    pub protocol_id: SmolStr,
    /// Host alternativo opcional (se ausente, o mesmo host da origem deve ser utilizado).
    pub host: Option<SmolStr>,
    /// Porta TCP/UDP onde o serviço alternativo está escutando.
    pub port: u16,
    /// Horário absoluto em que o registro expira.
    pub expires_at: SystemTime,
    /// Se o registro deve persistir entre mudanças de rede.
    pub persist: bool,
}

impl AltSvcRecord {
    /// Verifica se o registro de serviço alternativo ainda é válido.
    pub fn is_valid(&self, now: SystemTime) -> bool {
        self.expires_at > now
    }
}

/// Analisa o valor bruto de um cabeçalho `Alt-Svc`.
/// Exemplos:
/// - `h3=":443"; ma=86400; persist=1`
/// - `h2="alt.example.com:443"; ma=3600`
/// - `clear`
pub fn parse_alt_svc(header_str: &str, now: SystemTime) -> Vec<AltSvcRecord> {
    let trimmed = header_str.trim();
    if trimmed.eq_ignore_ascii_case("clear") {
        return Vec::new();
    }

    let mut records = Vec::new();

    for entry in trimmed.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        // Divide parâmetros separados por ponto e vírgula
        let mut parts = entry.split(';');
        let service_part = match parts.next() {
            Some(p) => p.trim(),
            None => continue,
        };

        // Formato: <protocol_id>="<host>:<port>" ou <protocol_id>=":<port>"
        let eq_idx = match service_part.find('=') {
            Some(idx) => idx,
            None => continue,
        };

        let protocol_id = service_part[..eq_idx].trim();
        let target_quoted = service_part[eq_idx + 1..].trim().trim_matches('"');

        // Divide host e porta
        let (host, port_str) = match target_quoted.rfind(':') {
            Some(idx) => {
                let h = &target_quoted[..idx];
                let p = &target_quoted[idx + 1..];
                (if h.is_empty() { None } else { Some(SmolStr::from(h)) }, p)
            }
            None => continue,
        };

        let port = match port_str.parse::<u16>() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let mut max_age = Duration::from_secs(86400); // Padrão: 24 horas (RFC 7838 §3.1)
        let mut persist = false;

        for param in parts {
            let param = param.trim();
            if let Some(stripped) = param.strip_prefix("ma=") {
                if let Ok(secs) = stripped.trim().parse::<u64>() {
                    max_age = Duration::from_secs(secs);
                }
            } else if param == "persist=1" {
                persist = true;
            }
        }

        records.push(AltSvcRecord {
            protocol_id: SmolStr::from(protocol_id),
            host,
            port,
            expires_at: now + max_age,
            persist,
        });
    }

    records
}

/// Chave de particionamento de serviço alternativo: (NIK serializado opcional, origin_host)
pub type AltSvcKey = (Option<String>, String);

/// Mapa de registros de serviços alternativos indexados por chave particionada.
pub type AltSvcMap = FxHashMap<AltSvcKey, Vec<AltSvcRecord>>;

/// Capacidade máxima de hosts rastreados no registro Alt-Svc para proteção de memória.
pub const MAX_ALT_SVC_ENTRIES: usize = 1000;

/// Registro compartilhado em memória de serviços alternativos.
#[derive(Debug, Default)]
pub struct AltSvcRegistry {
    entries: RwLock<AltSvcMap>,
}

impl AltSvcRegistry {
    /// Cria uma nova instância de `AltSvcRegistry`.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(FxHashMap::default()),
        }
    }

    /// Registra novos serviços alternativos para um host, respeitando a quota máxima de segurança.
    pub fn insert(
        &self,
        nik: Option<&NetworkIsolationKey>,
        origin_host: &str,
        records: Vec<AltSvcRecord>,
    ) {
        let key = (nik.map(|k| k.serialize()), origin_host.to_ascii_lowercase());
        let mut map = self.entries.write();
        if map.len() >= MAX_ALT_SVC_ENTRIES && !map.contains_key(&key) {
            if let Some(evict_key) = map.keys().next().cloned() {
                map.remove(&evict_key);
            }
        }
        map.insert(key, records);
    }

    /// Limpa registros associados a um host (ex: quando recebe `Alt-Svc: clear`).
    pub fn clear(&self, nik: Option<&NetworkIsolationKey>, origin_host: &str) {
        let key = (nik.map(|k| k.serialize()), origin_host.to_ascii_lowercase());
        self.entries.write().remove(&key);
    }

    /// Remove todos os registros alternativos cuja data de validade já expirou.
    pub fn cleanup_expired(&self, now: SystemTime) {
        let mut map = self.entries.write();
        map.retain(|_, records| {
            records.retain(|r| r.is_valid(now));
            !records.is_empty()
        });
    }

    /// Retorna a quantidade de hosts registrados no AltSvcRegistry.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Verifica se o registro está vazio.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retorna os serviços alternativos válidos para o host informado.
    pub fn get_alternatives(
        &self,
        nik: Option<&NetworkIsolationKey>,
        origin_host: &str,
        now: SystemTime,
    ) -> Vec<AltSvcRecord> {
        let key = (nik.map(|k| k.serialize()), origin_host.to_ascii_lowercase());
        let map = self.entries.read();
        match map.get(&key) {
            Some(records) => records.iter().filter(|r| r.is_valid(now)).cloned().collect(),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_alt_svc_multiple() {
        let now = SystemTime::now();
        let header = "h3=\":443\"; ma=3600; persist=1, h2=\"alt.example.com:8443\"; ma=7200";
        let records = parse_alt_svc(header, now);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].protocol_id.as_str(), "h3");
        assert_eq!(records[0].host, None);
        assert_eq!(records[0].port, 443);
        assert!(records[0].persist);

        assert_eq!(records[1].protocol_id.as_str(), "h2");
        assert_eq!(records[1].host.as_deref(), Some("alt.example.com"));
        assert_eq!(records[1].port, 8443);
        assert!(!records[1].persist);
    }

    #[test]
    fn test_parse_alt_svc_clear() {
        let now = SystemTime::now();
        let records = parse_alt_svc("clear", now);
        assert!(records.is_empty());
    }

    #[test]
    fn test_alt_svc_registry_isolation() {
        let registry = AltSvcRegistry::new();
        let now = SystemTime::now();
        let records = parse_alt_svc("h3=\":443\"; ma=3600", now);

        registry.insert(None, "example.com", records);
        let alternatives = registry.get_alternatives(None, "example.com", now);
        assert_eq!(alternatives.len(), 1);
        assert_eq!(alternatives[0].protocol_id.as_str(), "h3");

        registry.clear(None, "example.com");
        assert!(registry.get_alternatives(None, "example.com", now).is_empty());
    }

    #[test]
    fn test_alt_svc_registry_cleanup_expired() {
        let registry = AltSvcRegistry::new();
        let now = SystemTime::now();

        // Registro expirando em 10 segundos
        let records = parse_alt_svc("h3=\":443\"; ma=10", now);
        registry.insert(None, "shortlived.com", records);
        assert_eq!(registry.len(), 1);

        // Limpeza no momento `now` não deve remover
        registry.cleanup_expired(now);
        assert_eq!(registry.len(), 1);

        // Limpeza 20 segundos depois (já expirado)
        registry.cleanup_expired(now + Duration::from_secs(20));
        assert_eq!(registry.len(), 0);
    }
}
