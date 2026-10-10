//! # Motor de MIME Sniffing WHATWG e Fiscalização de `nosniff`
//!
//! Implementa a especificação normativa WHATWG MIME Sniffing Standard (<https://mimesniff.spec.whatwg.org/>)
//! para identificação determinística do tipo MIME real de recursos web quando omitidos
//! ou incorretamente informados pelo servidor (ex: `text/plain` para imagens),
//! além da proteção estrita contra ataques de confusão de tipo via `X-Content-Type-Options: nosniff`.

use crate::http::request::RequestDestination;
use http::HeaderMap;
use smol_str::SmolStr;

/// Tamanho máximo de inspeção de bytes para identificação por assinatura (magic numbers).
pub const MIME_SNIFF_BUFFER_LIMIT: usize = 512;

/// Verifica se o cabeçalho `X-Content-Type-Options: nosniff` está ativo nos cabeçalhos da resposta.
pub fn is_nosniff_header_present(headers: &HeaderMap) -> bool {
    if let Some(val) = headers.get("x-content-type-options") {
        if let Ok(s) = val.to_str() {
            return s.trim().eq_ignore_ascii_case("nosniff");
        }
    }
    false
}

/// Extrai a essência do tipo MIME ignorando parâmetros adicionais (ex: `"text/html; charset=utf-8"` -> `"text/html"`).
pub fn extract_mime_essence(content_type: &str) -> SmolStr {
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    SmolStr::new(essence)
}

/// Determina se um tipo MIME fornecido é considerado genérico ou desconhecido segundo a especificação.
pub fn is_unknown_mime_type(mime: Option<&str>) -> bool {
    match mime {
        None => true,
        Some(raw) => {
            let essence = extract_mime_essence(raw);
            essence.is_empty()
                || essence == "unknown/unknown"
                || essence == "application/unknown"
                || essence == "*/*"
                || essence == "text/plain"
                || essence == "application/octet-stream"
        }
    }
}

/// Valida se o tipo MIME recebido é compatível com o destino de requisição (`RequestDestination`)
/// sob a diretiva de segurança estrita `X-Content-Type-Options: nosniff`.
///
/// Retorna `true` se permitido e `false` se houver violação de tipo proibida pelo navegador.
pub fn validate_nosniff_content_type(expected_dest: RequestDestination, actual_mime: &str) -> bool {
    let essence = extract_mime_essence(actual_mime);

    match expected_dest {
        RequestDestination::Script => {
            // WHATWG: Apenas MIME types canônicos de JavaScript são aceitos sob nosniff
            matches!(
                essence.as_str(),
                "application/javascript"
                    | "text/javascript"
                    | "application/x-javascript"
                    | "application/ecmascript"
                    | "text/ecmascript"
                    | "text/jscript"
                    | "text/livescript"
                    | "application/wasm"
            )
        }
        RequestDestination::Style => {
            // WHATWG: Estilos CSS devem ser estritamente text/css
            essence == "text/css"
        }
        RequestDestination::Image => {
            essence.starts_with("image/") || essence == "image/svg+xml"
        }
        RequestDestination::Font => {
            essence.starts_with("font/")
                || essence == "application/font-woff"
                || essence == "application/x-font-woff"
                || essence == "application/font-ttf"
                || essence == "application/x-font-ttf"
                || essence == "application/vnd.ms-fontobject"
        }
        RequestDestination::Media => {
            essence.starts_with("audio/") || essence.starts_with("video/")
        }
        RequestDestination::Document => {
            matches!(
                essence.as_str(),
                "text/html"
                    | "application/xhtml+xml"
                    | "application/xml"
                    | "text/xml"
                    | "image/svg+xml"
                    | "text/plain"
                    | "application/pdf"
            )
        }
        RequestDestination::Fetch | RequestDestination::Other => true,
    }
}

/// Identifica o tipo MIME exato analisando o cabeçalho fornecido e os primeiros bytes do corpo.
///
/// Se `nosniff` estiver ativado, o tipo informado pelo servidor é preservado (ou fallback seguro).
/// Caso contrário, executa a bateria de *magic numbers* da especificação WHATWG.
pub fn sniff_mime_type(supplied_type: Option<&str>, body_bytes: &[u8], nosniff: bool) -> SmolStr {
    if nosniff {
        return match supplied_type {
            Some(supplied) if !supplied.trim().is_empty() => extract_mime_essence(supplied),
            _ => SmolStr::new("application/octet-stream"),
        };
    }

    // Se o tipo fornecido for específico e não genérico/desconhecido, mantemos a essência
    if let Some(supplied) = supplied_type {
        let essence = extract_mime_essence(supplied);
        if !is_unknown_mime_type(Some(&essence)) {
            return essence;
        }
    }

    // Sniffing por Magic Numbers nos primeiros 512 bytes
    let limit = std::cmp::min(body_bytes.len(), MIME_SNIFF_BUFFER_LIMIT);
    let sample = &body_bytes[..limit];

    if sample.is_empty() {
        return SmolStr::new("text/plain");
    }

    // 1. Imagens e formatos binários reconhecidos
    if sample.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return SmolStr::new("image/png");
    }
    if sample.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return SmolStr::new("image/jpeg");
    }
    if sample.starts_with(b"GIF87a") || sample.starts_with(b"GIF89a") {
        return SmolStr::new("image/gif");
    }
    if sample.starts_with(b"BM") && sample.len() >= 14 {
        return SmolStr::new("image/bmp");
    }
    if sample.starts_with(&[0x00, 0x00, 0x01, 0x00]) || sample.starts_with(&[0x00, 0x00, 0x02, 0x00]) {
        return SmolStr::new("image/x-icon");
    }
    if sample.len() >= 12 && sample.starts_with(b"RIFF") && &sample[8..12] == b"WEBP" {
        return SmolStr::new("image/webp");
    }
    if sample.len() >= 12 && sample.starts_with(b"RIFF") && &sample[8..12] == b"WAVE" {
        return SmolStr::new("audio/wav");
    }
    if sample.starts_with(b"OggS") {
        return SmolStr::new("audio/ogg");
    }
    if sample.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return SmolStr::new("video/webm");
    }
    if sample.starts_with(b"%PDF-") {
        return SmolStr::new("application/pdf");
    }
    if sample.starts_with(b"%!PS") {
        return SmolStr::new("application/postscript");
    }
    if sample.starts_with(&[0x00, 0x61, 0x73, 0x6D]) {
        return SmolStr::new("application/wasm");
    }

    // ISO Base Media File Format (MP4 / AVIF)
    if sample.len() >= 12 && &sample[4..8] == b"ftyp" {
        let brand = &sample[8..12];
        if brand == b"avif" || brand == b"avis" {
            return SmolStr::new("image/avif");
        }
        if brand == b"mp41" || brand == b"mp42" || brand == b"isom" || brand == b"dash" {
            return SmolStr::new("video/mp4");
        }
    }

    // Áudio MP3
    if sample.starts_with(b"ID3")
        || sample.starts_with(&[0xFF, 0xFB])
        || sample.starts_with(&[0xFF, 0xF3])
        || sample.starts_with(&[0xFF, 0xF2])
    {
        return SmolStr::new("audio/mpeg");
    }

    // Fontes
    if sample.starts_with(b"wOFF") {
        return SmolStr::new("font/woff");
    }
    if sample.starts_with(b"wOF2") {
        return SmolStr::new("font/woff2");
    }
    if sample.starts_with(&[0x00, 0x01, 0x00, 0x00]) || sample.starts_with(b"true") {
        return SmolStr::new("font/ttf");
    }
    if sample.starts_with(b"OTTO") {
        return SmolStr::new("font/otf");
    }

    // 2. Formatos de Texto e Markup (ignorando whitespace inicial)
    let non_ws_idx = sample
        .iter()
        .position(|&b| !matches!(b, 0x09 | 0x0A | 0x0C | 0x0D | 0x20))
        .unwrap_or(sample.len());

    let text_slice = &sample[non_ws_idx..];

    if text_slice.starts_with(b"<?xml") {
        return SmolStr::new("application/xml");
    }

    // Sniffing de HTML conforme tabela WHATWG
    if is_html_signature(text_slice) {
        return SmolStr::new("text/html");
    }

    // JSON heurístico básico: inicia com `{` ou `[` e possui delimitador de string
    if (text_slice.starts_with(b"{") || text_slice.starts_with(b"["))
        && (text_slice.contains(&b':') || text_slice.contains(&b'"') || text_slice.ends_with(b"}") || text_slice.ends_with(b"]"))
    {
        return SmolStr::new("application/json");
    }

    // 3. Fallback: Verificação de bytes binários
    // Bytes não-imprimíveis e caracteres de controle indicam octet-stream
    let has_binary_bytes = sample.iter().any(|&b| {
        (b <= 0x08)
            || b == 0x0B
            || (0x0E..=0x1A).contains(&b)
            || (0x1C..=0x1F).contains(&b)
    });

    if has_binary_bytes {
        SmolStr::new("application/octet-stream")
    } else {
        SmolStr::new("text/plain")
    }
}

/// Auxiliar que testa se os bytes textuais correspondem a tags primárias de HTML.
fn is_html_signature(bytes: &[u8]) -> bool {
    let tags: &[&[u8]] = &[
        b"<!doctype html",
        b"<html",
        b"<head",
        b"<script",
        b"<iframe",
        b"<h1",
        b"<div",
        b"<font",
        b"<table",
        b"<a",
        b"<style",
        b"<title",
        b"<b",
        b"<body",
        b"<!--",
    ];

    for &tag in tags {
        if bytes.len() >= tag.len() {
            let prefix = &bytes[..tag.len()];
            if prefix.eq_ignore_ascii_case(tag) {
                // Deve ser seguido por espaço, '>' ou '/'
                if bytes.len() == tag.len() {
                    return true;
                }
                let next = bytes[tag.len()];
                if matches!(next, 0x09 | 0x0A | 0x0C | 0x0D | 0x20 | b'>' | b'/') {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mime_essence() {
        assert_eq!(extract_mime_essence("text/html; charset=utf-8"), "text/html");
        assert_eq!(extract_mime_essence("IMAGE/PNG "), "image/png");
        assert_eq!(extract_mime_essence("application/json;odata=verbose"), "application/json");
    }

    #[test]
    fn test_sniff_png_and_jpeg() {
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(sniff_mime_type(None, &png_bytes, false), "image/png");
        assert_eq!(
            sniff_mime_type(Some("text/plain"), &png_bytes, false),
            "image/png"
        );

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(sniff_mime_type(None, &jpeg_bytes, false), "image/jpeg");
    }

    #[test]
    fn test_sniff_html_and_xml() {
        let html_bytes = b"   \r\n  <!DOCTYPE html><html><body><h1>Teste</h1></body></html>";
        assert_eq!(sniff_mime_type(None, html_bytes, false), "text/html");

        let xml_bytes = b"  <?xml version=\"1.0\" encoding=\"UTF-8\"?><root/>";
        assert_eq!(sniff_mime_type(None, xml_bytes, false), "application/xml");
    }

    #[test]
    fn test_nosniff_enforcement() {
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        // Com nosniff, o tipo text/plain fornecido pelo servidor NÃO deve ser farejado como image/png
        assert_eq!(
            sniff_mime_type(Some("text/plain"), &png_bytes, true),
            "text/plain"
        );

        // Validação de destinos com nosniff
        assert!(!validate_nosniff_content_type(RequestDestination::Script, "image/png"));
        assert!(!validate_nosniff_content_type(RequestDestination::Script, "text/plain"));
        assert!(validate_nosniff_content_type(RequestDestination::Script, "text/javascript"));
        assert!(validate_nosniff_content_type(RequestDestination::Script, "application/javascript"));

        assert!(!validate_nosniff_content_type(RequestDestination::Style, "text/html"));
        assert!(validate_nosniff_content_type(RequestDestination::Style, "text/css"));
    }

    #[test]
    fn test_binary_vs_text_fallback() {
        let plain_text = b"Hello, this is a normal UTF-8 text file without binary characters.";
        assert_eq!(sniff_mime_type(None, plain_text, false), "text/plain");

        let binary_payload = [0x01, 0x02, 0x00, 0x7F, 0x80];
        assert_eq!(sniff_mime_type(None, &binary_payload, false), "application/octet-stream");
    }

    #[test]
    fn test_sniff_avif_and_wasm() {
        let wasm_bytes = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert_eq!(sniff_mime_type(None, &wasm_bytes, false), "application/wasm");

        let mut avif_bytes = vec![0x00, 0x00, 0x00, 0x20]; // box length
        avif_bytes.extend_from_slice(b"ftypavif");
        assert_eq!(sniff_mime_type(None, &avif_bytes, false), "image/avif");
    }
}
