//! # Requisições de Faixa de Bytes e Conteúdo Parcial (RFC 9110 §14)
//!
//! Permite requisições de trechos específicos de arquivos grandes (`Range: bytes=...`)
//! e processamento de respostas `206 Partial Content` com cabeçalho `Content-Range`.

use http::HeaderValue;

/// Especificação da faixa de bytes solicitada na requisição HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRangeSpec {
    /// Faixa a partir do offset inicial até o fim do recurso (ex: `bytes=1024-`).
    From(u64),
    /// Faixa contígua fechada (inclusiva) entre dois offsets (ex: `bytes=0-499`).
    Range(u64, u64),
    /// Últimos N bytes do recurso (ex: `bytes=-500`).
    Suffix(u64),
}

impl ByteRangeSpec {
    /// Formata a especificação no cabeçalho padronizado `Range`.
    pub fn to_header_value(self) -> HeaderValue {
        let s = match self {
            Self::From(start) => format!("bytes={}-", start),
            Self::Range(start, end) => format!("bytes={}-{}", start, end),
            Self::Suffix(len) => format!("bytes=-{}", len),
        };
        HeaderValue::try_from(s).unwrap_or_else(|_| HeaderValue::from_static("bytes=0-"))
    }
}

/// Informações estruturadas de uma resposta parcial extraídas do cabeçalho `Content-Range`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    /// Primeiro byte da faixa entregue (inclusivo).
    pub start: u64,
    /// Último byte da faixa entregue (inclusivo).
    pub end: u64,
    /// Tamanho total do recurso completo em bytes, se conhecido pelo servidor.
    pub total: Option<u64>,
}

impl ContentRange {
    /// Analisa o valor de um cabeçalho `Content-Range`.
    /// Exemplo: `bytes 200-1000/67589` ou `bytes 200-1000/*`
    pub fn parse(val: &HeaderValue) -> Option<Self> {
        let s = val.to_str().ok()?.trim();
        let stripped = s.strip_prefix("bytes ")?.trim();

        let slash_idx = stripped.find('/')?;
        let range_part = &stripped[..slash_idx].trim();
        let total_part = &stripped[slash_idx + 1..].trim();

        let dash_idx = range_part.find('-')?;
        let start = range_part[..dash_idx].trim().parse::<u64>().ok()?;
        let end = range_part[dash_idx + 1..].trim().parse::<u64>().ok()?;

        let total = if *total_part == "*" {
            None
        } else {
            Some(total_part.parse::<u64>().ok()?)
        };

        Some(Self { start, end, total })
    }

    /// Retorna o tamanho desta fatia de bytes.
    pub fn range_len(&self) -> u64 {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_range_formatting() {
        assert_eq!(ByteRangeSpec::From(1024).to_header_value().to_str().unwrap(), "bytes=1024-");
        assert_eq!(ByteRangeSpec::Range(0, 499).to_header_value().to_str().unwrap(), "bytes=0-499");
        assert_eq!(ByteRangeSpec::Suffix(500).to_header_value().to_str().unwrap(), "bytes=-500");
    }

    #[test]
    fn test_content_range_parsing() {
        let val = HeaderValue::from_static("bytes 200-1000/67589");
        let cr = ContentRange::parse(&val).unwrap();

        assert_eq!(cr.start, 200);
        assert_eq!(cr.end, 1000);
        assert_eq!(cr.total, Some(67589));
        assert_eq!(cr.range_len(), 801);

        let val_wildcard = HeaderValue::from_static("bytes 0-499/*");
        let cr_wildcard = ContentRange::parse(&val_wildcard).unwrap();
        assert_eq!(cr_wildcard.start, 0);
        assert_eq!(cr_wildcard.end, 499);
        assert_eq!(cr_wildcard.total, None);
    }
}
