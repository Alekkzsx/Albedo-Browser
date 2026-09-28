//! # Oblivious HTTP (OHTTP - RFC 9458)
//!
//! Encapsula requisições HTTP (telemetria, DNS, navegação privada) usando HPKE (Hybrid Public Key Encryption).
//! Isso garante que o relay (que conhece o IP do cliente) não conheça o destino/conteúdo,
//! e o gateway (que conhece o destino/conteúdo) não conheça o IP do cliente.

use crate::error::NetResult;
use bytes::Bytes;

/// Configuração do Relay e Gateway OHTTP
#[derive(Clone, Debug)]
pub struct OhttpConfig {
    pub relay_url: String,
    pub gateway_public_key: Vec<u8>,
    pub key_id: u8,
}

/// Representa o construtor de pacotes Oblivious HTTP
pub struct OhttpEncapsulator {
    config: OhttpConfig,
}

impl OhttpEncapsulator {
    pub fn new(config: OhttpConfig) -> Self {
        Self { config }
    }

    /// Encapsula uma requisição HTTP bruta em uma mensagem Binary HTTP (BHTTP) criptografada (HPKE).
    pub fn encapsulate_request(&self, _bhttp_payload: &[u8]) -> NetResult<Bytes> {
        // TODO: M5 - Implementar serialização BHTTP completa e HPKE single-shot encryption
        // Usaremos aws_lc_rs::hpke para blindar o payload.
        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::Warning, 
            &self.config.relay_url, 
            "OHTTP Encapsulation Stub triggered"
        );
        Ok(Bytes::from("encrypted_stub_payload"))
    }

    /// Desencapsula a resposta BHTTP criptografada retornada pelo relay.
    pub fn decapsulate_response(&self, _enc_response: &[u8]) -> NetResult<Bytes> {
        // TODO: M5 - HPKE open
        Ok(Bytes::from("decrypted_stub_response"))
    }
}
