//! # Defesa de Intranet e Acesso a Redes Privadas (W3C Private Network Access - PNA)
//!
//! Implementa as restrições da especificação W3C Private Network Access para
//! impedir que websites públicos realizem requisições não autorizadas contra
//! dispositivos e serviços situados na rede local (ex: roteadores, impressoras,
//! servidores em localhost/127.0.0.1 ou faixas RFC 1918).

use crate::error::{NetError, NetResult};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Classificação do espaço de endereçamento IP segundo a especificação W3C PNA §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IpAddressSpace {
    /// Endereços de loopback ou link-local (`127.0.0.1`, `::1`, `169.254.0.0/16`, `fe80::/10`).
    Local,
    /// Endereços de rede privada interna (RFC 1918 `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`).
    Private,
    /// Endereços públicos roteáveis na Internet global.
    Public,
}

/// Classifica um endereço IP no respectivo espaço de endereçamento (Local, Private ou Public).
pub fn classify_ip(ip: IpAddr) -> IpAddressSpace {
    match ip {
        IpAddr::V4(v4) => classify_ipv4(v4),
        IpAddr::V6(v6) => classify_ipv6(v6),
    }
}

/// Classifica um endereço IPv4.
pub fn classify_ipv4(ip: Ipv4Addr) -> IpAddressSpace {
    let octets = ip.octets();

    // Loopback: 127.0.0.0/8
    if octets[0] == 127 {
        return IpAddressSpace::Local;
    }

    // Link-Local: 169.254.0.0/16
    if octets[0] == 169 && octets[1] == 254 {
        return IpAddressSpace::Local;
    }

    // This-Host: 0.0.0.0/8
    if octets[0] == 0 {
        return IpAddressSpace::Local;
    }

    // RFC 1918: 10.0.0.0/8
    if octets[0] == 10 {
        return IpAddressSpace::Private;
    }

    // RFC 1918: 172.16.0.0/12 (172.16.0.0 até 172.31.255.255)
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return IpAddressSpace::Private;
    }

    // RFC 1918: 192.168.0.0/16
    if octets[0] == 192 && octets[1] == 168 {
        return IpAddressSpace::Private;
    }

    // Carrier-Grade NAT: 100.64.0.0/10 (100.64.0.0 até 100.127.255.255)
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return IpAddressSpace::Private;
    }

    IpAddressSpace::Public
}

/// Classifica um endereço IPv6.
pub fn classify_ipv6(ip: Ipv6Addr) -> IpAddressSpace {
    // Loopback: ::1
    if ip == Ipv6Addr::LOCALHOST {
        return IpAddressSpace::Local;
    }

    // Unspecified: ::
    if ip == Ipv6Addr::UNSPECIFIED {
        return IpAddressSpace::Local;
    }

    let segments = ip.segments();

    // Link-Local Unicast: fe80::/10 (fe80: até febf:)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return IpAddressSpace::Local;
    }

    // Unique Local Unicast (ULA): fc00::/7 (fc00: até fdff:)
    if (segments[0] & 0xfe00) == 0xfc00 {
        return IpAddressSpace::Private;
    }

    // IPv4-Mapped IPv6: ::ffff:a.b.c.d
    if let Some(v4) = ip.to_ipv4_mapped() {
        return classify_ipv4(v4);
    }

    IpAddressSpace::Public
}

/// Verifica se um endereço IP é local ou de rede privada.
pub fn is_private_or_local(ip: IpAddr) -> bool {
    classify_ip(ip) != IpAddressSpace::Public
}

/// Verifica se uma string de host representa um endereço local ou privado (incluindo `localhost`).
pub fn is_host_private_or_local(host: &str) -> bool {
    let clean = host.trim().to_ascii_lowercase();
    if clean == "localhost" || clean.ends_with(".localhost") {
        return true;
    }

    if let Ok(ip) = clean.parse::<IpAddr>() {
        return is_private_or_local(ip);
    }

    false
}

/// Valida uma requisição segundo as diretrizes de Private Network Access.
/// Se a origem da requisição for pública e o destino for um IP local ou privado, aborta a requisição.
pub fn validate_private_network_access(initiator_is_public: bool, target_ip: IpAddr) -> NetResult<()> {
    if initiator_is_public && is_private_or_local(target_ip) {
        let space = classify_ip(target_ip);
        crate::net_log::log_net_event(
            crate::net_log::NetEventType::Warning,
            &target_ip.to_string(),
            "Bloqueado pelo Private Network Access (PNA): origem publica tentou acessar IP local/privado",
        );
        return Err(NetError::SecurityViolation(format!(
            "Private Network Access bloqueado: origem publica tentou acessar recurso no espaco {:?} ({})",
            space, target_ip
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_ipv4_addresses() {
        assert_eq!(classify_ipv4("127.0.0.1".parse().unwrap()), IpAddressSpace::Local);
        assert_eq!(classify_ipv4("127.0.1.1".parse().unwrap()), IpAddressSpace::Local);
        assert_eq!(classify_ipv4("169.254.10.20".parse().unwrap()), IpAddressSpace::Local);

        assert_eq!(classify_ipv4("10.0.0.1".parse().unwrap()), IpAddressSpace::Private);
        assert_eq!(classify_ipv4("172.16.5.10".parse().unwrap()), IpAddressSpace::Private);
        assert_eq!(classify_ipv4("172.31.255.254".parse().unwrap()), IpAddressSpace::Private);
        assert_eq!(classify_ipv4("192.168.1.1".parse().unwrap()), IpAddressSpace::Private);
        assert_eq!(classify_ipv4("100.64.0.1".parse().unwrap()), IpAddressSpace::Private);

        assert_eq!(classify_ipv4("1.1.1.1".parse().unwrap()), IpAddressSpace::Public);
        assert_eq!(classify_ipv4("8.8.8.8".parse().unwrap()), IpAddressSpace::Public);
        assert_eq!(classify_ipv4("142.250.190.46".parse().unwrap()), IpAddressSpace::Public);
    }

    #[test]
    fn test_classify_ipv6_addresses() {
        assert_eq!(classify_ipv6("::1".parse().unwrap()), IpAddressSpace::Local);
        assert_eq!(classify_ipv6("fe80::1".parse().unwrap()), IpAddressSpace::Local);
        assert_eq!(classify_ipv6("fd00::1".parse().unwrap()), IpAddressSpace::Private);
        assert_eq!(classify_ipv6("2606:4700:4700::1111".parse().unwrap()), IpAddressSpace::Public);
    }

    #[test]
    fn test_validate_pna_blocks_public_to_local_or_private() {
        let local_ip = "127.0.0.1".parse().unwrap();
        let private_ip = "192.168.0.1".parse().unwrap();
        let public_ip = "93.184.216.34".parse().unwrap();

        // Origem pública tentando acessar IP local: BLOQUEIO
        assert!(validate_private_network_access(true, local_ip).is_err());

        // Origem pública tentando acessar IP privado: BLOQUEIO
        assert!(validate_private_network_access(true, private_ip).is_err());

        // Origem pública acessando IP público: PERMITIDO
        assert!(validate_private_network_access(true, public_ip).is_ok());

        // Origem privada/local acessando qualquer destino: PERMITIDO
        assert!(validate_private_network_access(false, local_ip).is_ok());
        assert!(validate_private_network_access(false, private_ip).is_ok());
        assert!(validate_private_network_access(false, public_ip).is_ok());
    }

    #[test]
    fn test_is_host_private_or_local() {
        assert!(is_host_private_or_local("localhost"));
        assert!(is_host_private_or_local("sub.localhost"));
        assert!(is_host_private_or_local("127.0.0.1"));
        assert!(is_host_private_or_local("192.168.1.254"));
        assert!(!is_host_private_or_local("example.com"));
        assert!(!is_host_private_or_local("1.1.1.1"));
    }
}
