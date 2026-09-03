//! # Descompressão Transparente de Conteúdo HTTP (RFC 9110 §8.4.1 / RFC 7932)
//!
//! Fornece decodificação de payloads comprimidos nos formatos:
//! - `gzip` (`flate2::read::GzDecoder`)
//! - `deflate` (`flate2::read::ZlibDecoder` com fallback para raw deflate)
//! - `br` / Brotli (`brotli::Decompressor`)
//! - `identity` (sem alteração)

use crate::error::{NetError, NetResult};
use bytes::Bytes;
use http::header::CONTENT_ENCODING;
use http::HeaderMap;
use std::io::Read;

/// Algoritmos de codificação de conteúdo suportados pelo motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentEncoding {
    /// Algoritmo Gzip (RFC 1952)
    Gzip,
    /// Algoritmo Deflate / Zlib (RFC 1950 / RFC 1951)
    Deflate,
    /// Algoritmo Brotli (RFC 7932)
    Brotli,
    /// Conteúdo bruto sem compressão
    Identity,
}

impl ContentEncoding {
    /// Identifica a codificação a partir do cabeçalho `Content-Encoding`.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let val = headers.get(CONTENT_ENCODING)?.to_str().ok()?.trim();
        Self::parse(val)
    }

    /// Analisa uma string de codificação.
    pub fn parse(encoding_str: &str) -> Option<Self> {
        let lower = encoding_str.to_ascii_lowercase();
        // Se houver múltiplas codificações separadas por vírgula, extrai a última (mais externa)
        let last_encoding = lower.split(',').next_back()?.trim();

        match last_encoding {
            "gzip" | "x-gzip" => Some(Self::Gzip),
            "deflate" => Some(Self::Deflate),
            "br" => Some(Self::Brotli),
            "identity" => Some(Self::Identity),
            _ => None,
        }
    }
}

impl std::str::FromStr for ContentEncoding {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
}

/// Descomprime o payload binário bruto de acordo com a codificação informada.
pub fn decompress_payload(encoding: ContentEncoding, raw: &Bytes) -> NetResult<Bytes> {
    if raw.is_empty() {
        return Ok(Bytes::new());
    }

    match encoding {
        ContentEncoding::Identity => Ok(raw.clone()),
        ContentEncoding::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(&raw[..]);
            let mut decompressed = Vec::with_capacity(raw.len() * 2);
            decoder.read_to_end(&mut decompressed).map_err(|e| {
                NetError::HttpProtocolError(format!("Falha ao descomprimir payload gzip: {}", e))
            })?;
            Ok(Bytes::from(decompressed))
        }
        ContentEncoding::Deflate => {
            // Tenta decodificar primeiro como zlib padrão
            let mut zlib_decoder = flate2::read::ZlibDecoder::new(&raw[..]);
            let mut decompressed = Vec::with_capacity(raw.len() * 2);
            if zlib_decoder.read_to_end(&mut decompressed).is_ok() {
                return Ok(Bytes::from(decompressed));
            }

            // Fallback para raw deflate sem cabeçalhos zlib
            let mut raw_decoder = flate2::read::DeflateDecoder::new(&raw[..]);
            let mut raw_decompressed = Vec::with_capacity(raw.len() * 2);
            raw_decoder.read_to_end(&mut raw_decompressed).map_err(|e| {
                NetError::HttpProtocolError(format!("Falha ao descomprimir payload deflate: {}", e))
            })?;
            Ok(Bytes::from(raw_decompressed))
        }
        ContentEncoding::Brotli => {
            let mut decompressed = Vec::with_capacity(raw.len() * 2);
            let mut reader = brotli::Decompressor::new(&raw[..], 4096);
            reader.read_to_end(&mut decompressed).map_err(|e| {
                NetError::HttpProtocolError(format!("Falha ao descomprimir payload brotli: {}", e))
            })?;
            Ok(Bytes::from(decompressed))
        }
    }
}

/// Helper para compressão gzip em testes e servidores mock.
pub fn compress_gzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Helper para compressão brotli em testes e servidores mock.
pub fn compress_brotli(data: &[u8]) -> Vec<u8> {
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 6, 22);
    std::io::Write::write_all(&mut writer, data).unwrap();
    writer.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_decompression() {
        let original = b"<!DOCTYPE html><html><body><h1>Albedo Decompression Test</h1></body></html>";
        let compressed = compress_gzip(original);
        let raw = Bytes::from(compressed);

        let decompressed = decompress_payload(ContentEncoding::Gzip, &raw).unwrap();
        assert_eq!(decompressed.as_ref(), original);
    }

    #[test]
    fn test_brotli_decompression() {
        let original = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Albedo Browser.";
        let compressed = compress_brotli(original);
        let raw = Bytes::from(compressed);

        let decompressed = decompress_payload(ContentEncoding::Brotli, &raw).unwrap();
        assert_eq!(decompressed.as_ref(), original);
    }

    #[test]
    fn test_content_encoding_header_parsing() {
        assert_eq!(ContentEncoding::parse("gzip"), Some(ContentEncoding::Gzip));
        assert_eq!(ContentEncoding::parse("br"), Some(ContentEncoding::Brotli));
        assert_eq!(ContentEncoding::parse("deflate"), Some(ContentEncoding::Deflate));
        assert_eq!(ContentEncoding::parse("gzip, br"), Some(ContentEncoding::Brotli));
        assert_eq!(ContentEncoding::parse("unknown-codec"), None);
    }
}
