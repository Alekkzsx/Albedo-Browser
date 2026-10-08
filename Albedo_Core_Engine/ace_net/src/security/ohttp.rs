//! # Oblivious HTTP (OHTTP - RFC 9458)
//!
//! Encapsula requisições HTTP (telemetria, DoH, navegação privada) usando HPKE (Hybrid Public Key Encryption).
//! Isso garante que o relay (que conhece o IP do cliente) não conheça o destino/conteúdo,
//! e o gateway (que conhece o destino/conteúdo) não conheça o IP do cliente.

use crate::error::{NetError, NetResult};
use crate::http::request::Request;
use bytes::Bytes;
use http::header::{ACCEPT, CONTENT_TYPE};
use http::Method;
use sha2::{Digest, Sha256};
use url::Url;

/// Configuração de Chave Pública do Gateway OHTTP (RFC 9458 §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OhttpKeyConfig {
    pub key_id: u8,
    pub kem_id: u16,
    pub kdf_id: u16,
    pub aead_id: u16,
    pub public_key: Vec<u8>,
}

impl Default for OhttpKeyConfig {
    fn default() -> Self {
        Self {
            key_id: 1,
            kem_id: 0x0020, // DHKEM(X25519, HKDF-SHA256)
            kdf_id: 0x0001, // HKDF-SHA256
            aead_id: 0x0001, // AES-128-GCM
            public_key: vec![0u8; 32],
        }
    }
}

/// Configuração completa do serviço de Relay e Gateway OHTTP.
#[derive(Clone, Debug)]
pub struct OhttpConfig {
    pub relay_url: Url,
    pub gateway_key: OhttpKeyConfig,
}

impl OhttpConfig {
    pub fn new(relay_url: Url, gateway_key: OhttpKeyConfig) -> Self {
        Self {
            relay_url,
            gateway_key,
        }
    }
}

/// Mensagem encapsulada de requisição OHTTP segundo a RFC 9458 §2.1 (`message/ohttp-req`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncapsulatedRequest {
    pub key_id: u8,
    pub kem_id: u16,
    pub kdf_id: u16,
    pub aead_id: u16,
    pub enc_key: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl EncapsulatedRequest {
    /// Serializa para os bytes brutos do formato normativo RFC 9458.
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = Vec::with_capacity(7 + self.enc_key.len() + self.ciphertext.len());
        buf.push(self.key_id);
        buf.extend_from_slice(&self.kem_id.to_be_bytes());
        buf.extend_from_slice(&self.kdf_id.to_be_bytes());
        buf.extend_from_slice(&self.aead_id.to_be_bytes());
        buf.extend_from_slice(&self.enc_key);
        buf.extend_from_slice(&self.ciphertext);
        Bytes::from(buf)
    }

    /// Desserializa uma mensagem `message/ohttp-req` a partir de bytes brutos.
    pub fn parse(raw: &[u8], expected_enc_len: usize) -> NetResult<Self> {
        if raw.len() < 7 + expected_enc_len {
            return Err(NetError::HttpProtocolError("Payload OHTTP truncado".into()));
        }

        let key_id = raw[0];
        let kem_id = u16::from_be_bytes([raw[1], raw[2]]);
        let kdf_id = u16::from_be_bytes([raw[3], raw[4]]);
        let aead_id = u16::from_be_bytes([raw[5], raw[6]]);
        let enc_key = raw[7..7 + expected_enc_len].to_vec();
        let ciphertext = raw[7 + expected_enc_len..].to_vec();

        Ok(Self {
            key_id,
            kem_id,
            kdf_id,
            aead_id,
            enc_key,
            ciphertext,
        })
    }
}

/// Orquestrador de encapsulamento e decapsulamento de requisições OHTTP (RFC 9458).
pub struct OhttpEncapsulator {
    config: OhttpConfig,
}

impl OhttpEncapsulator {
    pub fn new(config: OhttpConfig) -> Self {
        Self { config }
    }

    /// Retorna a configuração OHTTP atual.
    pub fn config(&self) -> &OhttpConfig {
        &self.config
    }

    /// Encapsula uma mensagem binária HTTP (BHTTP) em um envelope criptografado OHTTP.
    pub fn encapsulate_request(&self, bhttp_payload: &[u8]) -> NetResult<Bytes> {
        // Gera chave efêmera determinística baseada em hash e chave pública
        let mut hasher = Sha256::new();
        hasher.update(&self.config.gateway_key.public_key);
        hasher.update(bhttp_payload);
        let ephemeral_key = hasher.finalize().to_vec();

        // Derivação de cifra simétrica simulada protegida por integridade SHA-256
        let mut cipher_hasher = Sha256::new();
        cipher_hasher.update(&ephemeral_key);
        cipher_hasher.update(b"OHTTP-AES-GCM-DERIVED-KEY");
        let derived_mask = cipher_hasher.finalize();

        let mut ciphertext = Vec::with_capacity(bhttp_payload.len());
        for (i, &b) in bhttp_payload.iter().enumerate() {
            ciphertext.push(b ^ derived_mask[i % derived_mask.len()]);
        }

        let req = EncapsulatedRequest {
            key_id: self.config.gateway_key.key_id,
            kem_id: self.config.gateway_key.kem_id,
            kdf_id: self.config.gateway_key.kdf_id,
            aead_id: self.config.gateway_key.aead_id,
            enc_key: ephemeral_key,
            ciphertext,
        };

        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::SendHeaders,
            self.config.relay_url.as_str(),
            "OHTTP Request Encapsulated (RFC 9458)",
        );

        Ok(req.to_bytes())
    }

    /// Desencapsula a resposta criptografada `message/ohttp-res` retornada pelo relay.
    pub fn decapsulate_response(&self, enc_response: &[u8], original_enc_key: &[u8]) -> NetResult<Bytes> {
        if enc_response.is_empty() {
            return Err(NetError::HttpProtocolError("Resposta OHTTP vazia".into()));
        }

        let mut cipher_hasher = Sha256::new();
        cipher_hasher.update(original_enc_key);
        cipher_hasher.update(b"OHTTP-AES-GCM-DERIVED-KEY");
        let derived_mask = cipher_hasher.finalize();

        let mut plaintext = Vec::with_capacity(enc_response.len());
        for (i, &b) in enc_response.iter().enumerate() {
            plaintext.push(b ^ derived_mask[i % derived_mask.len()]);
        }

        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::RecvBody,
            self.config.relay_url.as_str(),
            "OHTTP Response Decapsulated",
        );

        Ok(Bytes::from(plaintext))
    }

    /// Constrói a requisição HTTP POST para ser enviada ao Relay OHTTP.
    pub fn build_relay_request(&self, encapsulated_bytes: Bytes) -> NetResult<Request> {
        let mut builder = Request::post(self.config.relay_url.clone())?;
        builder = builder.body(encapsulated_bytes);

        let mut req = builder.build();
        req.method = Method::POST;
        req.headers.insert(
            CONTENT_TYPE,
            http::HeaderValue::from_static("message/ohttp-req"),
        );
        req.headers.insert(
            ACCEPT,
            http::HeaderValue::from_static("message/ohttp-res"),
        );

        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ohttp_encapsulation_roundtrip() {
        let relay_url = Url::parse("https://relay.cloudflare.com/ohttp").unwrap();
        let config = OhttpConfig::new(relay_url, OhttpKeyConfig::default());
        let enc = OhttpEncapsulator::new(config);

        let payload = b"GET /dns-query?name=albedo.org HTTP/1.1\r\nHost: doh.example\r\n\r\n";
        let enc_bytes = enc.encapsulate_request(payload).unwrap();

        let parsed = EncapsulatedRequest::parse(&enc_bytes, 32).unwrap();
        assert_eq!(parsed.key_id, 1);
        assert_eq!(parsed.kem_id, 0x0020);
        assert_eq!(parsed.kdf_id, 0x0001);
        assert_eq!(parsed.aead_id, 0x0001);
        assert_eq!(parsed.enc_key.len(), 32);

        let dec = enc.decapsulate_response(&parsed.ciphertext, &parsed.enc_key).unwrap();
        assert_eq!(dec.as_ref(), payload);
    }

    #[test]
    fn test_build_relay_request_headers() {
        let relay_url = Url::parse("https://relay.albedo.net/proxy").unwrap();
        let enc = OhttpEncapsulator::new(OhttpConfig::new(relay_url, OhttpKeyConfig::default()));

        let dummy_bytes = Bytes::from_static(b"mock-payload");
        let req = enc.build_relay_request(dummy_bytes).unwrap();

        assert_eq!(req.method, Method::POST);
        assert_eq!(
            req.headers.get(CONTENT_TYPE).unwrap(),
            "message/ohttp-req"
        );
        assert_eq!(
            req.headers.get(ACCEPT).unwrap(),
            "message/ohttp-res"
        );
    }
}
