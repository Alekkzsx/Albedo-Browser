//! # Roteamento Corporativo e Privacidade via Proxies (HTTP CONNECT & SOCKS5 RFC 1928)
//!
//! Implementa suporte normativo a conexões intermediadas com proteção de privacidade:
//! - **SOCKS5 (RFC 1928):** Tunelamento com resolução de DNS remota no próprio proxy (*Zero DNS Leak*).
//! - **HTTP CONNECT:** Tunelamento seguro para tráfego HTTPS através de proxies corporativos com autenticação Basic.
//! - **ProxyBypassList:** Lista configurável de exceções para endereços locais e domínios intranet.

use crate::error::{NetError, NetResult};
use base64::Engine;
use smol_str::SmolStr;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Timeout padrão para handshake de proxy (10 segundos).
const PROXY_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Credenciais de autenticação para servidores proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyAuth {
    pub username: SmolStr,
    pub password: SmolStr,
}

impl ProxyAuth {
    pub fn new(username: impl Into<SmolStr>, password: impl Into<SmolStr>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Codifica as credenciais para o cabeçalho HTTP `Proxy-Authorization: Basic <base64>`.
    pub fn to_basic_auth_header(&self) -> String {
        let creds = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(creds);
        format!("Basic {}", encoded)
    }
}

/// Configuração do tipo de roteamento de rede via Proxy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyConfig {
    /// Conexão direta sem intermediários.
    #[default]
    Direct,
    /// Proxy HTTP que suporta o método `CONNECT` para tráfego seguro.
    Http {
        proxy_addr: SocketAddr,
        auth: Option<ProxyAuth>,
    },
    /// Proxy SOCKS5 (RFC 1928) com resolução DNS no destino remoto (Zero DNS Leak).
    Socks5 {
        proxy_addr: SocketAddr,
        auth: Option<ProxyAuth>,
    },
}

impl ProxyConfig {
    pub fn direct() -> Self {
        Self::Direct
    }

    pub fn http(proxy_addr: SocketAddr, auth: Option<ProxyAuth>) -> Self {
        Self::Http { proxy_addr, auth }
    }

    pub fn socks5(proxy_addr: SocketAddr, auth: Option<ProxyAuth>) -> Self {
        Self::Socks5 { proxy_addr, auth }
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct)
    }
}

/// Gerenciador de regras de bypass para conexão direta fora do proxy.
#[derive(Debug, Clone)]
pub struct ProxyBypassList {
    rules: Vec<SmolStr>,
}

impl Default for ProxyBypassList {
    fn default() -> Self {
        Self {
            rules: vec![
                SmolStr::new("localhost"),
                SmolStr::new("127.0.0.1"),
                SmolStr::new("::1"),
                SmolStr::new(".local"),
            ],
        }
    }
}

impl ProxyBypassList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adiciona uma nova regra de bypass (ex: `"*.corp.internal"` ou `"192.168.1.1"`).
    pub fn add_rule(&mut self, rule: impl Into<SmolStr>) {
        self.rules.push(rule.into());
    }

    /// Avalia se um host alvo deve ignorar o proxy e conectar-se diretamente.
    pub fn should_bypass(&self, host: &str) -> bool {
        let host_lower = host.to_ascii_lowercase();
        for rule in &self.rules {
            let r = rule.to_ascii_lowercase();
            if let Some(suffix) = r.strip_prefix('*') {
                if host_lower.ends_with(suffix) {
                    return true;
                }
            } else if let Some(suffix) = r.strip_prefix('.') {
                if host_lower.ends_with(&r) || host_lower == suffix {
                    return true;
                }
            } else if host_lower == r {
                return true;
            }
        }
        false
    }
}

/// Estabelece a conexão TCP de transporte, intermediada por Proxy quando aplicável.
pub async fn establish_connection(
    proxy: &ProxyConfig,
    bypass_list: Option<&ProxyBypassList>,
    target_host: &str,
    target_port: u16,
) -> NetResult<TcpStream> {
    if let Some(bypass) = bypass_list {
        if bypass.should_bypass(target_host) {
            return connect_direct(target_host, target_port).await;
        }
    }

    match proxy {
        ProxyConfig::Direct => connect_direct(target_host, target_port).await,
        ProxyConfig::Http { proxy_addr, auth } => {
            timeout(
                PROXY_HANDSHAKE_TIMEOUT,
                connect_http_connect_tunnel(*proxy_addr, auth.as_ref(), target_host, target_port),
            )
            .await
            .map_err(|_| NetError::Timeout)?
        }
        ProxyConfig::Socks5 { proxy_addr, auth } => {
            timeout(
                PROXY_HANDSHAKE_TIMEOUT,
                connect_socks5_tunnel(*proxy_addr, auth.as_ref(), target_host, target_port),
            )
            .await
            .map_err(|_| NetError::Timeout)?
        }
    }
}

async fn connect_direct(host: &str, port: u16) -> NetResult<TcpStream> {
    TcpStream::connect(format!("{}:{}", host, port))
        .await
        .map_err(|e| NetError::ConnectionFailed(host.to_string(), e.to_string()))
}

/// Realiza o handshake HTTP CONNECT sobre um proxy corporativo.
pub async fn connect_http_connect_tunnel(
    proxy_addr: SocketAddr,
    auth: Option<&ProxyAuth>,
    target_host: &str,
    target_port: u16,
) -> NetResult<TcpStream> {
    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| NetError::ConnectionFailed(proxy_addr.to_string(), e.to_string()))?;

    let authority = format!("{}:{}", target_host, target_port);
    let mut request = format!(
        "CONNECT {} HTTP/1.1\r\nHost: {}\r\nProxy-Connection: Keep-Alive\r\n",
        authority, authority
    );

    if let Some(creds) = auth {
        request.push_str(&format!("Proxy-Authorization: {}\r\n", creds.to_basic_auth_header()));
    }
    request.push_str("\r\n");

    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    // Lê a resposta do túnel HTTP
    let mut buf = [0u8; 1024];
    let mut total_read = 0;

    while total_read < buf.len() {
        let n = stream.read(&mut buf[total_read..]).await?;
        if n == 0 {
            return Err(NetError::ConnectionFailed(
                proxy_addr.to_string(),
                "Proxy encerrou conexão prematuramente durante CONNECT".into(),
            ));
        }
        total_read += n;

        // Procura pelo término do cabeçalho (\r\n\r\n)
        if let Some(pos) = buf[..total_read].windows(4).position(|w| w == b"\r\n\r\n") {
            let header_str = String::from_utf8_lossy(&buf[..pos]);
            let first_line = header_str.lines().next().unwrap_or("");

            if first_line.contains("200") {
                return Ok(stream);
            } else if first_line.contains("407") {
                return Err(NetError::SecurityViolation(
                    "Autenticação de proxy necessária (HTTP 407 Proxy Authentication Required)".into(),
                ));
            } else {
                return Err(NetError::ConnectionFailed(
                    proxy_addr.to_string(),
                    format!("Proxy rejeitou CONNECT: {}", first_line),
                ));
            }
        }
    }

    Err(NetError::HttpProtocolError("Resposta de CONNECT do proxy excedeu o tamanho máximo".into()))
}

/// Realiza o handshake SOCKS5 completo (RFC 1928 + RFC 1929) com Zero DNS Leak.
pub async fn connect_socks5_tunnel(
    proxy_addr: SocketAddr,
    auth: Option<&ProxyAuth>,
    target_host: &str,
    target_port: u16,
) -> NetResult<TcpStream> {
    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| NetError::ConnectionFailed(proxy_addr.to_string(), e.to_string()))?;

    // 1. Handshake de Métodos de Autenticação (RFC 1928 §3)
    if auth.is_some() {
        // Métodos suportados: 0x00 (NO AUTH) e 0x02 (USER/PASSWORD)
        stream.write_all(&[0x05, 0x02, 0x00, 0x02]).await?;
    } else {
        // Apenas 0x00 (NO AUTH)
        stream.write_all(&[0x05, 0x01, 0x00]).await?;
    }
    stream.flush().await?;

    let mut method_resp = [0u8; 2];
    stream.read_exact(&mut method_resp).await?;

    if method_resp[0] != 0x05 {
        return Err(NetError::HttpProtocolError(
            "Versão SOCKS inválida retornada pelo servidor".into(),
        ));
    }

    match method_resp[1] {
        0x00 => {
            // Sem autenticação exigida, prossegue
        }
        0x02 => {
            // Autenticação de Usuário e Senha (RFC 1929)
            let creds = auth.ok_or_else(|| {
                NetError::SecurityViolation("Proxy exige autenticação, mas nenhuma foi configurada".into())
            })?;

            let u_bytes = creds.username.as_bytes();
            let p_bytes = creds.password.as_bytes();

            if u_bytes.len() > 255 || p_bytes.len() > 255 {
                return Err(NetError::SecurityViolation(
                    "Credenciais de proxy SOCKS5 excedem o limite de 255 bytes".into(),
                ));
            }

            let mut auth_req = Vec::with_capacity(3 + u_bytes.len() + p_bytes.len());
            auth_req.push(0x01); // Versão do subnegociador RFC 1929
            auth_req.push(u_bytes.len() as u8);
            auth_req.extend_from_slice(u_bytes);
            auth_req.push(p_bytes.len() as u8);
            auth_req.extend_from_slice(p_bytes);

            stream.write_all(&auth_req).await?;
            stream.flush().await?;

            let mut auth_resp = [0u8; 2];
            stream.read_exact(&mut auth_resp).await?;

            if auth_resp[1] != 0x00 {
                return Err(NetError::SecurityViolation(
                    "Falha na autenticação do proxy SOCKS5 (usuário ou senha incorretos)".into(),
                ));
            }
        }
        0xFF => {
            return Err(NetError::SecurityViolation(
                "Nenhum método de autenticação aceito pelo servidor proxy SOCKS5".into(),
            ));
        }
        other => {
            return Err(NetError::HttpProtocolError(format!(
                "Método de autenticação SOCKS5 não suportado: 0x{:02X}",
                other
            )));
        }
    }

    // 2. Requisição de Conexão (RFC 1928 §4)
    // Para garantir Zero DNS Leak, domínios são enviados como ATYP 0x03 (Domain Name)
    let mut connect_req = Vec::with_capacity(64);
    connect_req.push(0x05); // VER
    connect_req.push(0x01); // CMD: CONNECT
    connect_req.push(0x00); // RSV

    if let Ok(ip) = target_host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                connect_req.push(0x01); // ATYP: IPv4
                connect_req.extend_from_slice(&v4.octets());
            }
            IpAddr::V6(v6) => {
                connect_req.push(0x04); // ATYP: IPv6
                connect_req.extend_from_slice(&v6.octets());
            }
        }
    } else {
        // Zero DNS Leak: ATYP 0x03 com o nome textual do domínio para resolução remota
        let host_bytes = target_host.as_bytes();
        if host_bytes.len() > 255 {
            return Err(NetError::InvalidUrl("Nome de host muito longo para SOCKS5".into()));
        }
        connect_req.push(0x03); // ATYP: DOMAINNAME
        connect_req.push(host_bytes.len() as u8);
        connect_req.extend_from_slice(host_bytes);
    }

    // Porta em Big-Endian (Network Byte Order)
    connect_req.extend_from_slice(&target_port.to_be_bytes());

    stream.write_all(&connect_req).await?;
    stream.flush().await?;

    // 3. Leitura da Resposta do Servidor SOCKS5
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;

    if header[0] != 0x05 {
        return Err(NetError::HttpProtocolError(
            "Resposta inválida de conexão SOCKS5".into(),
        ));
    }

    let rep = header[1];
    if rep != 0x00 {
        let err_desc = match rep {
            0x01 => "Falha geral do servidor SOCKS",
            0x02 => "Conexão não permitida pelo conjunto de regras",
            0x03 => "Rede inalcançável",
            0x04 => "Host de destino inalcançável",
            0x05 => "Conexão recusada pelo host remoto",
            0x06 => "TTL expirado",
            0x07 => "Comando não suportado pelo proxy",
            0x08 => "Tipo de endereço não suportado",
            _ => "Erro desconhecido do proxy SOCKS5",
        };
        return Err(NetError::ConnectionFailed(
            target_host.to_string(),
            format!("SOCKS5 REP 0x{:02X}: {}", rep, err_desc),
        ));
    }

    // Consome o endereço de ligação (BND.ADDR e BND.PORT)
    let atyp = header[3];
    match atyp {
        0x01 => {
            let mut bnd = [0u8; 6]; // 4 bytes IPv4 + 2 bytes port
            stream.read_exact(&mut bnd).await?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let mut domain_and_port = vec![0u8; len_buf[0] as usize + 2];
            stream.read_exact(&mut domain_and_port).await?;
        }
        0x04 => {
            let mut bnd = [0u8; 18]; // 16 bytes IPv6 + 2 bytes port
            stream.read_exact(&mut bnd).await?;
        }
        other => {
            return Err(NetError::HttpProtocolError(format!(
                "ATYP de resposta inválido retornado pelo SOCKS5: 0x{:02X}",
                other
            )));
        }
    }

    // Túnel TCP completamente estabelecido e isolado
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_auth_basic_header() {
        let auth = ProxyAuth::new("albedo_user", "sota_secret");
        let header = auth.to_basic_auth_header();
        assert!(header.starts_with("Basic "));
        // "albedo_user:sota_secret" em base64 é "YWxiZWRvX3VzZXI6c290YV9zZWNyZXQ="
        assert_eq!(header, "Basic YWxiZWRvX3VzZXI6c290YV9zZWNyZXQ=");
    }

    #[test]
    fn test_proxy_bypass_list() {
        let mut bypass = ProxyBypassList::new();
        assert!(bypass.should_bypass("localhost"));
        assert!(bypass.should_bypass("127.0.0.1"));
        assert!(bypass.should_bypass("server.local"));
        assert!(!bypass.should_bypass("example.com"));

        bypass.add_rule("*.intranet.corp");
        assert!(bypass.should_bypass("finance.intranet.corp"));
        assert!(!bypass.should_bypass("corp.com"));
    }

    #[test]
    fn test_proxy_config_defaults() {
        let cfg = ProxyConfig::direct();
        assert!(cfg.is_direct());
    }
}
