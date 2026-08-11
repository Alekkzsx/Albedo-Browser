// ============================================================================
// Albedo Core Engine (ACE)
// File: data_uri.rs
// Description: Parseador de URIs `data:` nativo com decodificação Base64 manual.
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::AceError;

/// Representa o resultado do parseamento de um `data: URI`.
#[derive(Debug, PartialEq)]
pub struct DataUri<'a> {
    pub mime_type: &'a str,
    pub data: Vec<u8>,
}

/// Mapeia o caractere base64 de volta para o seu valor inteiro (0-63).
#[inline(always)]
fn decode_b64_char(c: u8) -> Result<u8, AceError> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        b'=' => Ok(0), // Padding
        _ => Err(AceError::Parse {
            message: format!("Caractere Base64 inválido: {}", c as char),
        }),
    }
}

/// Decodifica uma string Base64 em um vetor de bytes brutos sem usar dependências.
pub fn decode_base64(input: &str) -> ace_core::AceResult<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut clean_bytes = Vec::with_capacity(bytes.len());
    
    // Ignora espaços e quebras de linha que são comuns no HTML Base64 (Line wrapping).
    for &b in bytes {
        if b != b' ' && b != b'\n' && b != b'\r' && b != b'\t' {
            clean_bytes.push(b);
        }
    }

    if clean_bytes.is_empty() {
        return Ok(Vec::new());
    }

    if clean_bytes.len() % 4 != 0 {
        return Err(AceError::Parse {
            message: "Comprimento do Base64 não é múltiplo de 4".to_string(),
        });
    }

    let mut padding = 0;
    if clean_bytes.ends_with(b"==") {
        padding = 2;
    } else if clean_bytes.ends_with(b"=") {
        padding = 1;
    }

    let mut output = Vec::with_capacity((clean_bytes.len() / 4) * 3);
    
    for chunk in clean_bytes.chunks_exact(4) {
        let n = (decode_b64_char(chunk[0])? as u32) << 18
              | (decode_b64_char(chunk[1])? as u32) << 12
              | (decode_b64_char(chunk[2])? as u32) << 6
              | (decode_b64_char(chunk[3])? as u32);

        output.push((n >> 16) as u8);
        output.push((n >> 8) as u8);
        output.push(n as u8);
    }

    // Remove os bytes zeros adicionados pelo padding final.
    for _ in 0..padding {
        output.pop();
    }

    Ok(output)
}

/// Extrai os metadados e os bytes crus de uma string `data:` (Ex: `data:text/plain;base64,SGVsbG8=`).
pub fn parse_data_uri(uri: &str) -> ace_core::AceResult<DataUri> {
    if !uri.starts_with("data:") {
        return Err(AceError::Parse {
            message: "URI não começa com 'data:'".to_string(),
        });
    }

    let rest = &uri[5..];
    
    // Procura pela vírgula que separa os metadados dos dados
    let comma_idx = rest.find(',').ok_or_else(|| AceError::Parse {
        message: "URI data: não contém vírgula separadora".to_string(),
    })?;

    let meta = &rest[..comma_idx];
    let data_str = &rest[comma_idx + 1..];

    // O padrão da web, se omitido, é text/plain;charset=US-ASCII
    let mut mime_type = "text/plain;charset=US-ASCII";
    let mut is_base64 = false;

    if !meta.is_empty() {
        if meta.ends_with(";base64") {
            is_base64 = true;
            let stripped = meta.strip_suffix(";base64").unwrap();
            if !stripped.is_empty() {
                mime_type = stripped;
            }
        } else {
            mime_type = meta;
        }
    }

    let data = if is_base64 {
        decode_base64(data_str)?
    } else {
        // Formato URL-encoded nativo (Percent-Encoding)
        url_decode(data_str)?
    };

    Ok(DataUri { mime_type, data })
}

/// Decodificação de porcentagem simples (URL-encoding) para `data:text/plain,Hello%20World`.
fn url_decode(input: &str) -> ace_core::AceResult<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 < bytes.len() {
                let hex_str = std::str::from_utf8(&bytes[i+1..i+3]).map_err(|_| AceError::Parse {
                    message: "Sequência URL-encoded inválida (Não é UTF-8)".to_string(),
                })?;
                
                let byte = u8::from_str_radix(hex_str, 16).map_err(|_| AceError::Parse {
                    message: "Hexadecimal inválido na URL".to_string(),
                })?;
                
                output.push(byte);
                i += 3;
            } else {
                return Err(AceError::Parse {
                    message: "Sequência URL-encoded incompleta no final".to_string(),
                });
            }
        } else {
            output.push(bytes[i]);
            i += 1;
        }
    }
    
    Ok(output)
}
