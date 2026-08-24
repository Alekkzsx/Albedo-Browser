//! Suíte de Testes de Rede e Hardening de Parser (Milestone M2 - R3)
//!
//! Validação de preservação de bytes em percent-decoding e integridade
//! de decodificação UTF-8 no limite de 512 bytes em sniffing MIME.

use ace_core::net::{is_safe_url_scheme, parse_data_uri, percent_decode, sniff_mime_type};

#[test]
fn test_percent_decode_malformed_preservation() {
    // Caso clássico do bug: % literal não seguido por 2 hex válidos
    assert_eq!(percent_decode("100%_concluido"), "100%_concluido");
    assert_eq!(percent_decode("progresso_100%"), "progresso_100%");
    assert_eq!(percent_decode("100%2"), "100%2");
    assert_eq!(percent_decode("teste%GG%ZZ%1G"), "teste%GG%ZZ%1G");
    assert_eq!(percent_decode("%"), "%");
    assert_eq!(percent_decode("%%"), "%%");
    assert_eq!(percent_decode("%2"), "%2");
    assert_eq!(percent_decode("%%%20"), "%% ");

    // Sequências hexadecimais válidas
    assert_eq!(percent_decode("Hello%20World%21"), "Hello World!");
    assert_eq!(percent_decode("%41%42%43"), "ABC");
    assert_eq!(percent_decode("%61%62%63"), "abc");

    // Sequências UTF-8 válidas codificadas em percentual
    assert_eq!(percent_decode("%C3%A9%C3%A0%C3%BC"), "éàü");
    assert_eq!(
        percent_decode("Albedo%20Browser%20-%20100%25%20Seguro"),
        "Albedo Browser - 100% Seguro"
    );
}

#[test]
fn test_sniff_mime_type_utf8_boundary_at_512_bytes() {
    // Constrói um payload HTML de 600 bytes onde o byte 512 cai exatamente
    // no meio de um caractere UTF-8 de 3 bytes (ex: 'あ' = [0xE3, 0x81, 0x82])
    let header = b"<!DOCTYPE html><html><head><title>";
    let mut payload = Vec::new();
    payload.extend_from_slice(header);

    // Adiciona caracteres ASCII até o byte 510
    while payload.len() < 510 {
        payload.push(b'a');
    }

    // Agora payload.len() == 510.
    // Inserimos o caractere UTF-8 japonês de 3 bytes 'あ' [0xE3, 0x81, 0x82]:
    // byte 510: 0xE3
    // byte 511: 0x81
    // byte 512: 0x82 (este fica fora do slice inicial &[..512]!)
    payload.extend_from_slice(&[0xE3, 0x81, 0x82]);

    // Completa o HTML
    payload.extend_from_slice(b"</title></head><body>Corpo</body></html>");
    assert!(payload.len() > 512);

    // O sniffing DEVE recuperar a porção válida e reconhecer como "text/html"
    let mime = sniff_mime_type(&payload);
    assert_eq!(
        mime, "text/html",
        "Falha ao identificar HTML com split UTF-8 na fronteira de 512 bytes"
    );
}

#[test]
fn test_sniff_mime_type_svg_utf8_boundary() {
    let header = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><!-- ";
    let mut payload = Vec::new();
    payload.extend_from_slice(header);

    while payload.len() < 511 {
        payload.push(b'x');
    }

    // Inserimos emoji '🚀' (4 bytes: [0xF0, 0x9F, 0x99, 0x80]) no byte 511
    // Apenas 1 byte do emoji entra nos primeiros 512 bytes
    payload.extend_from_slice(&[0xF0, 0x9F, 0x99, 0x80]);
    payload.extend_from_slice(b" --><rect width=\"100\" height=\"100\"/></svg>");

    let mime = sniff_mime_type(&payload);
    assert_eq!(mime, "image/svg+xml");
}

#[test]
fn test_sniff_mime_type_binary_fallback() {
    // Dados binários aleatórios que não casam com nenhum magic number e não são UTF-8 válido
    let binary_data = [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x00, 0x12];
    assert_eq!(sniff_mime_type(&binary_data), "application/octet-stream");
}


#[test]
fn test_data_uri_and_safe_schemes() {
    let data_uri = "data:text/plain;charset=utf-8;base64,QWxiZWRv";
    let (mime, bytes) = parse_data_uri(data_uri).unwrap();
    assert_eq!(mime.essence(), "text/plain");
    assert_eq!(mime.charset(), Some("utf-8"));
    assert_eq!(String::from_utf8(bytes).unwrap(), "Albedo");

    let percent_data_uri = "data:text/html,<h1>100%_concluido</h1>";
    let (mime2, bytes2) = parse_data_uri(percent_data_uri).unwrap();
    assert_eq!(mime2.essence(), "text/html");
    assert_eq!(String::from_utf8(bytes2).unwrap(), "<h1>100%_concluido</h1>");

    assert!(is_safe_url_scheme("https"));
    assert!(is_safe_url_scheme("http"));
    assert!(is_safe_url_scheme("data"));
    assert!(is_safe_url_scheme("file"));
    assert!(!is_safe_url_scheme("javascript"));
    assert!(!is_safe_url_scheme("vbscript"));
}
