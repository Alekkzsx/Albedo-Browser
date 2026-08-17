//! # Tabelas e Codificador WHATWG Percent-Encoding (WHATWG URL Standard 4.4)
//!
//! Implementação com consulta em bitmap de 256 bits ($O(1)$ sem alocações) para os conjuntos
//! canônicos de percent-encoding da web: C0Control, Fragment, Query, SpecialQuery, Path, Userinfo e Component.

const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

/// Conjuntos de caracteres padronizados pelo WHATWG URL Standard para codificação percentual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PercentEncodeSet {
    /// C0 Controls (0x00..=0x1F e > 0x7E).
    C0Control,
    /// Fragment Percent-Encode Set.
    Fragment,
    /// Query Percent-Encode Set.
    Query,
    /// Special-Query Percent-Encode Set (inclui aspa simples `'`).
    SpecialQuery,
    /// Path Percent-Encode Set.
    Path,
    /// Userinfo Percent-Encode Set.
    Userinfo,
    /// Component Percent-Encode Set.
    Component,
}

impl PercentEncodeSet {
    /// Verifica se o byte especificado deve ser percent-encoded neste conjunto.
    #[inline(always)]
    pub const fn contains(self, byte: u8) -> bool {
        // C0 controls e non-ASCII são sempre codificados em todos os conjuntos
        if byte <= 0x1F || byte > 0x7E {
            return true;
        }

        match self {
            Self::C0Control => false,
            Self::Fragment => matches!(byte, b' ' | b'"' | b'<' | b'>' | b'`'),
            Self::Query => matches!(byte, b' ' | b'"' | b'#' | b'<' | b'>'),
            Self::SpecialQuery => matches!(byte, b' ' | b'"' | b'#' | b'<' | b'>' | b'\''),
            Self::Path => matches!(
                byte,
                b' ' | b'"' | b'#' | b'<' | b'>' | b'?' | b'^' | b'`' | b'{' | b'}'
            ),
            Self::Userinfo => matches!(
                byte,
                b' ' | b'"'
                    | b'#'
                    | b'<'
                    | b'>'
                    | b'?'
                    | b'^'
                    | b'`'
                    | b'{'
                    | b'}'
                    | b'/'
                    | b':'
                    | b';'
                    | b'='
                    | b'@'
                    | b'['
                    | b'\\'
                    | b']'
                    | b'|'
            ),
            Self::Component => matches!(
                byte,
                b' ' | b'"'
                    | b'#'
                    | b'<'
                    | b'>'
                    | b'?'
                    | b'^'
                    | b'`'
                    | b'{'
                    | b'}'
                    | b'/'
                    | b':'
                    | b';'
                    | b'='
                    | b'@'
                    | b'['
                    | b'\\'
                    | b']'
                    | b'|'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'+'
                    | b','
            ),
        }
    }
}

/// Retorna a representação percent-encoded de 3 bytes (`%XX`) caso o caractere precise ser escapado.
#[inline]
pub const fn percent_encode_byte(byte: u8, set: PercentEncodeSet) -> Option<[u8; 3]> {
    if set.contains(byte) {
        let hi = HEX_CHARS[(byte >> 4) as usize];
        let lo = HEX_CHARS[(byte & 0x0F) as usize];
        Some([b'%', hi, lo])
    } else {
        None
    }
}

/// Codifica uma string UTF-8 de acordo com o conjunto percent-encode especificado.
pub fn percent_encode(input: &str, set: PercentEncodeSet) -> String {
    percent_encode_bytes(input.as_bytes(), set)
}

/// Codifica uma fatia de bytes brutos conforme o conjunto percent-encode especificado.
pub fn percent_encode_bytes(bytes: &[u8], set: PercentEncodeSet) -> String {
    // Verificação rápida: se nenhum byte precisa de codificação, retorna string direta
    if !bytes.iter().any(|&b| set.contains(b)) {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    let mut output = String::with_capacity(bytes.len() * 3 / 2);
    for &byte in bytes {
        if let Some(encoded) = percent_encode_byte(byte, set) {
            output.push(encoded[0] as char);
            output.push(encoded[1] as char);
            output.push(encoded[2] as char);
        } else {
            output.push(byte as char);
        }
    }
    output
}
