//! # DNS-over-HTTPS (DoH) & Happy Eyeballs v2 (RFC 8305)
//!
//! Implementa o adaptador `hyper_util::client::legacy::connect::dns::Resolve`
//! sobre o resolver assíncrono `hickory_resolver` com interleaving de endereços
//! IPv6 e IPv4 conforme o algoritmo Happy Eyeballs v2.

use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use hyper_util::client::legacy::connect::dns::{Name, Resolve};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;

/// Resolucão de DNS criptografado via HTTPS (DoH) com ordenação Happy Eyeballs v2 (RFC 8305).
#[derive(Clone)]
pub struct DohHappyEyeballsResolver {
    resolver: Arc<TokioAsyncResolver>,
}

impl DohHappyEyeballsResolver {
    /// Cria uma nova instância de `DohHappyEyeballsResolver` apontando para o DoH Cloudflare 1.1.1.1.
    pub fn new() -> Self {
        Self {
            resolver: Arc::new(TokioAsyncResolver::tokio(
                ResolverConfig::cloudflare_https(),
                ResolverOpts::default(),
            )),
        }
    }

    /// Cria uma instância a partir de um resolver Hickory já configurado.
    pub fn with_resolver(resolver: Arc<TokioAsyncResolver>) -> Self {
        Self { resolver }
    }

    /// Implementa o algoritmo de ordenação e interleaving de Happy Eyeballs v2 (RFC 8305 §4):
    /// Intercala endereços IPv6 e IPv4 (ex: [IPv6_1, IPv4_1, IPv6_2, IPv4_2])
    /// com preferência inicial estrita para IPv6.
    pub fn interleave_happy_eyeballs(addrs: Vec<IpAddr>) -> Vec<SocketAddr> {
        let mut v6 = Vec::new();
        let mut v4 = Vec::new();

        for ip in addrs {
            match ip {
                IpAddr::V6(a) => v6.push(SocketAddr::new(IpAddr::V6(a), 0)),
                IpAddr::V4(a) => v4.push(SocketAddr::new(IpAddr::V4(a), 0)),
            }
        }

        let mut interleaved = Vec::with_capacity(v6.len() + v4.len());
        let mut i6 = v6.into_iter();
        let mut i4 = v4.into_iter();

        loop {
            let next_v6 = i6.next();
            let next_v4 = i4.next();

            if next_v6.is_none() && next_v4.is_none() {
                break;
            }
            if let Some(a6) = next_v6 {
                interleaved.push(a6);
            }
            if let Some(a4) = next_v4 {
                interleaved.push(a4);
            }
        }

        interleaved
    }
}

impl Default for DohHappyEyeballsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolve for DohHappyEyeballsResolver {
    type Addr = SocketAddr;
    type Iterator = std::vec::IntoIter<SocketAddr>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Iterator, std::io::Error>> + Send>>;

    fn resolve(&self, name: Name) -> Self::Future {
        let resolver = self.resolver.clone();
        let host_str = name.as_str().to_string();

        Box::pin(async move {
            match resolver.lookup_ip(&host_str).await {
                Ok(lookup) => {
                    let ips: Vec<IpAddr> = lookup.iter().collect();
                    let interleaved = Self::interleave_happy_eyeballs(ips);
                    if interleaved.is_empty() {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Nenhum endereço IP retornado via DoH para '{}'", host_str),
                        ))
                    } else {
                        Ok(interleaved.into_iter())
                    }
                }
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Falha na resolução DoH para '{}': {}", host_str, e),
                )),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_happy_eyeballs_interleaving() {
        let addrs = vec![
            IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
            IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)),
            IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1001)),
        ];

        let interleaved = DohHappyEyeballsResolver::interleave_happy_eyeballs(addrs);
        assert_eq!(interleaved.len(), 4);

        // Primeiro elemento DEVE ser IPv6 conforme RFC 8305
        assert!(interleaved[0].is_ipv6());
        // Segundo elemento DEVE ser IPv4
        assert!(interleaved[1].is_ipv4());
        // Terceiro elemento DEVE ser IPv6
        assert!(interleaved[2].is_ipv6());
        // Quarto elemento DEVE ser IPv4
        assert!(interleaved[3].is_ipv4());
    }
}
