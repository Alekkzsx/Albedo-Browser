//! # Tratamento de Erros e Exceções de Rede (`NetError`)
//!
//! Define a taxonomia de falhas de transporte, DNS, segurança TLS,
//! protocolo HTTP e conformidade de cache para o subsistema `ace_net`.

use std::io;
use thiserror::Error;

/// Erros estruturados do subsistema de rede `ace_net`.
#[derive(Debug, Error)]
pub enum NetError {
    /// URL fornecida é malformada ou inválida segundo a especificação WHATWG.
    #[error("URL inválida: {0}")]
    InvalidUrl(String),

    /// Falha na resolução de nomes de domínio (DNS).
    #[error("Falha na resolução DNS para '{0}': {1}")]
    DnsResolutionFailed(String, String),

    /// Falha no estabelecimento de conexão TCP com o host remoto.
    #[error("Falha de conexão com '{0}': {1}")]
    ConnectionFailed(String, String),

    /// Falha durante o handshake criptográfico TLS (ex: certificado inválido/expirado).
    #[error("Falha de handshake TLS com '{0}': {1}")]
    TlsHandshakeFailed(String, String),

    /// Violação de protocolo HTTP ou erro de framing HTTP/1.1 ou HTTP/2.
    #[error("Erro no protocolo HTTP: {0}")]
    HttpProtocolError(String),

    /// A requisição excedeu o tempo limite (timeout) configurado.
    #[error("A operação de rede atingiu o tempo limite")]
    Timeout,

    /// A requisição foi cancelada ativamente pelo chamador ou por cancelamento de navegação.
    #[error("A requisição foi cancelada")]
    Cancelled,

    /// Excedido o limite máximo de saltos de redirecionamento (proteção contra loops infinitos).
    #[error("Excedido o limite de redirecionamentos ({0})")]
    TooManyRedirects(usize),

    /// Redirecionamento inseguro detectado (ex: downgrade de HTTPS para HTTP não autorizado).
    #[error("Redirecionamento inseguro bloqueado: {0}")]
    UnsafeRedirect(String),

    /// Erro no subsistema de cache HTTP ou violação de integridade.
    #[error("Erro no cache HTTP: {0}")]
    CacheError(String),

    /// O corpo de resposta excedeu a quota máxima de segurança permitida.
    #[error("O payload da resposta excedeu o limite máximo")]
    PayloadTooLarge,

    /// Esquema de URL não suportado pelo cliente de rede (ex: `ftp:`, `file:` direto na rede).
    #[error("Esquema não suportado: '{0}'")]
    UnsupportedScheme(String),

    /// Erro de I/O subjacente do sistema operacional.
    #[error("Erro de E/S de rede: {0}")]
    Io(#[from] io::Error),
}

/// Tipo `Result` especializado para operações em `ace_net`.
pub type NetResult<T> = Result<T, NetError>;

impl From<url::ParseError> for NetError {
    fn from(err: url::ParseError) -> Self {
        NetError::InvalidUrl(err.to_string())
    }
}

impl From<hyper::Error> for NetError {
    fn from(err: hyper::Error) -> Self {
        NetError::HttpProtocolError(err.to_string())
    }
}

impl From<http::Error> for NetError {
    fn from(err: http::Error) -> Self {
        NetError::HttpProtocolError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_formatting() {
        let err = NetError::InvalidUrl("ht!tp://invalid".into());
        assert_eq!(err.to_string(), "URL inválida: ht!tp://invalid");

        let redirect_err = NetError::TooManyRedirects(20);
        assert_eq!(redirect_err.to_string(), "Excedido o limite de redirecionamentos (20)");
    }

    #[test]
    fn test_url_parse_error_conversion() {
        let parse_result = url::Url::parse("not a url");
        assert!(parse_result.is_err());
        let net_err: NetError = parse_result.unwrap_err().into();
        match net_err {
            NetError::InvalidUrl(_) => {}
            other => panic!("Esperado InvalidUrl, obtido {:?}", other),
        }
    }
}
