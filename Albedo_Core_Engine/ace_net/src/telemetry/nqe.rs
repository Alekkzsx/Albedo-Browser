//! # Network Quality Estimator (NQE) e W3C Network Information API
//!
//! Monitora o desempenho em tempo real das conexões da engine, calculando médias móveis
//! exponencialmente ponderadas (EWMA) de RTT (HTTP e Transporte) e vazão de Downlink (kbps),
//! classificando o tipo efetivo de conexão (`EffectiveConnectionType`) e injetando
//! Network Client Hints normativos (`ECT`, `RTT`, `Downlink`).

use http::header::HeaderMap;
use http::HeaderValue;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Classificação do tipo efetivo de conexão segundo a especificação W3C Network Information API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EffectiveConnectionType {
    /// Sem observações suficientes para classificação precisa.
    #[default]
    Unknown,
    /// Conexão extremamente lenta (RTT >= 2000ms ou Downlink <= 50 kbps).
    Slow2G,
    /// Conexão 2G (RTT >= 1400ms ou Downlink <= 70 kbps).
    TwoG,
    /// Conexão 3G móvel típica (RTT >= 270ms ou Downlink <= 700 kbps).
    ThreeG,
    /// Conexão 4G/Banda Larga de alta velocidade (RTT < 270ms e Downlink > 700 kbps).
    FourG,
}

impl EffectiveConnectionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Slow2G => "slow-2g",
            Self::TwoG => "2g",
            Self::ThreeG => "3g",
            Self::FourG => "4g",
        }
    }
}

/// Registro pontual de uma transação de rede para cálculo de qualidade.
#[derive(Debug, Clone)]
pub struct NetworkObservation {
    pub timestamp: Instant,
    pub rtt: Duration,
    pub bytes_transferred: usize,
    pub transfer_duration: Duration,
}

#[derive(Debug)]
struct NqeState {
    http_rtt_ewma_ms: Option<f64>,
    transport_rtt_ewma_ms: Option<f64>,
    downlink_kbps_ewma: Option<f64>,
    observations_count: usize,
}

/// Estimador contínuo de qualidade de rede e latência.
#[derive(Debug, Clone)]
pub struct NetworkQualityEstimator {
    state: Arc<RwLock<NqeState>>,
    /// Fator de decaimento para média móvel EWMA (padrão 0.85).
    decay_factor: f64,
}

impl Default for NetworkQualityEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkQualityEstimator {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(NqeState {
                http_rtt_ewma_ms: None,
                transport_rtt_ewma_ms: None,
                downlink_kbps_ewma: None,
                observations_count: 0,
            })),
            decay_factor: 0.85,
        }
    }

    /// Registra uma observação de requisição concluída.
    pub fn record_observation(&self, rtt: Duration, bytes: usize, transfer_duration: Duration) {
        let mut guard = self.state.write();
        let rtt_ms = rtt.as_secs_f64() * 1000.0;

        guard.http_rtt_ewma_ms = match guard.http_rtt_ewma_ms {
            Some(prev) => Some(prev * self.decay_factor + rtt_ms * (1.0 - self.decay_factor)),
            None => Some(rtt_ms),
        };

        if transfer_duration.as_millis() > 0 && bytes > 0 {
            let dur_secs = transfer_duration.as_secs_f64();
            let bits = (bytes as f64) * 8.0;
            let current_kbps = (bits / 1000.0) / dur_secs;

            guard.downlink_kbps_ewma = match guard.downlink_kbps_ewma {
                Some(prev) => Some(prev * self.decay_factor + current_kbps * (1.0 - self.decay_factor)),
                None => Some(current_kbps),
            };
        }

        guard.observations_count += 1;
    }

    /// Registra uma observação de latência de transporte físico (TCP / TLS connect time).
    pub fn record_transport_rtt(&self, rtt: Duration) {
        let mut guard = self.state.write();
        let rtt_ms = rtt.as_secs_f64() * 1000.0;

        guard.transport_rtt_ewma_ms = match guard.transport_rtt_ewma_ms {
            Some(prev) => Some(prev * self.decay_factor + rtt_ms * (1.0 - self.decay_factor)),
            None => Some(rtt_ms),
        };
    }

    /// Retorna a estimativa atual de RTT HTTP em milissegundos.
    pub fn http_rtt(&self) -> Option<Duration> {
        self.state
            .read()
            .http_rtt_ewma_ms
            .map(|ms| Duration::from_secs_f64(ms / 1000.0))
    }

    /// Retorna a estimativa atual de RTT de Transporte (TCP/TLS) em milissegundos.
    pub fn transport_rtt(&self) -> Option<Duration> {
        self.state
            .read()
            .transport_rtt_ewma_ms
            .map(|ms| Duration::from_secs_f64(ms / 1000.0))
    }

    /// Retorna a vazão estimada de Downlink em kilobits por segundo (kbps).
    pub fn downlink_kbps(&self) -> Option<f64> {
        self.state.read().downlink_kbps_ewma
    }

    /// Determina o `EffectiveConnectionType` combinando as métricas de RTT e Downlink.
    pub fn effective_connection_type(&self) -> EffectiveConnectionType {
        let guard = self.state.read();
        let rtt = guard.http_rtt_ewma_ms;
        let kbps = guard.downlink_kbps_ewma;

        match (rtt, kbps) {
            (None, None) => EffectiveConnectionType::Unknown,
            (Some(r), _) if r >= 2000.0 => EffectiveConnectionType::Slow2G,
            (_, Some(k)) if k <= 50.0 => EffectiveConnectionType::Slow2G,
            (Some(r), _) if r >= 1400.0 => EffectiveConnectionType::TwoG,
            (_, Some(k)) if k <= 70.0 => EffectiveConnectionType::TwoG,
            (Some(r), _) if r >= 270.0 => EffectiveConnectionType::ThreeG,
            (_, Some(k)) if k <= 700.0 => EffectiveConnectionType::ThreeG,
            _ => EffectiveConnectionType::FourG,
        }
    }

    /// Injeta os cabeçalhos de Network Client Hints (ECT, RTT, Downlink)
    /// com quantização de privacidade (arredondamento para evitar fingerprinting de hardware).
    pub fn inject_network_client_hints(&self, headers: &mut HeaderMap) {
        let ect = self.effective_connection_type();
        if ect != EffectiveConnectionType::Unknown {
            headers.insert("ect", HeaderValue::from_static(ect.as_str()));
        }

        if let Some(rtt_ms) = self.state.read().http_rtt_ewma_ms {
            // Arredonda para o múltiplo de 25ms mais próximo (defesa anti-fingerprinting do Chromium)
            let rounded_rtt = ((rtt_ms / 25.0).round() * 25.0) as u64;
            if let Ok(val) = HeaderValue::from_str(&rounded_rtt.to_string()) {
                headers.insert("rtt", val);
            }
        }

        if let Some(kbps) = self.state.read().downlink_kbps_ewma {
            // Downlink em Megabits por segundo (Mbps), arredondado para múltiplos de 0.05 Mbps
            let mbps = (kbps / 1000.0 * 20.0).round() / 20.0;
            if let Ok(val) = HeaderValue::from_str(&format!("{:.2}", mbps)) {
                headers.insert("downlink", val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nqe_classification_and_hints() {
        let nqe = NetworkQualityEstimator::new();
        assert_eq!(nqe.effective_connection_type(), EffectiveConnectionType::Unknown);

        // Observação de conexão rápida (RTT 50ms, 1 MB em 200ms = 40 Mbps)
        nqe.record_observation(Duration::from_millis(50), 1_000_000, Duration::from_millis(200));
        assert_eq!(nqe.effective_connection_type(), EffectiveConnectionType::FourG);

        let mut headers = HeaderMap::new();
        nqe.inject_network_client_hints(&mut headers);
        assert_eq!(headers.get("ect").unwrap(), "4g");
        assert!(headers.contains_key("rtt"));
        assert!(headers.contains_key("downlink"));

        // Observação de conexão lenta (RTT 2500ms, 5 KB em 3s = ~13 kbps)
        let slow_nqe = NetworkQualityEstimator::new();
        slow_nqe.record_observation(Duration::from_millis(2500), 5000, Duration::from_secs(3));
        assert_eq!(slow_nqe.effective_connection_type(), EffectiveConnectionType::Slow2G);

        let mut slow_headers = HeaderMap::new();
        slow_nqe.inject_network_client_hints(&mut slow_headers);
        assert_eq!(slow_headers.get("ect").unwrap(), "slow-2g");
    }
}

