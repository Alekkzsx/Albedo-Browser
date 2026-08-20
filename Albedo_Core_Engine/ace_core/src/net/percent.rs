//! # Tabelas e Codificador WHATWG Percent-Encoding (WHATWG URL Standard 4.4)
//!
//! Implementação com consulta em bitmap de 256 bits estático ($O(1)$ sem alocações)
//! para os conjuntos canônicos de percent-encoding da web.

const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

/// Conjuntos de caracteres padronizados pelo WHATWG URL Standard para codificação percentual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PercentEncodeSet {
    /// C0 Controls (0x00..=0x1F e > 0x7E).
    C0Control = 0,
    /// Fragment Percent-Encode Set.
    Fragment = 1,
    /// Query Percent-Encode Set.
    Query = 2,
    /// Special-Query Percent-Encode Set (inclui aspa simples `'`).
    SpecialQuery = 3,
    /// Path Percent-Encode Set.
    Path = 4,
    /// Userinfo Percent-Encode Set.
    Userinfo = 5,
    /// Component Percent-Encode Set.
    Component = 6,
}

const fn make_encode_bitmap(set: PercentEncodeSet) -> [u64; 4] {
    let mut map = [0u64; 4];
    let mut b = 0u16;
    while b <= 255 {
        let byte = b as u8;
        let should_encode = if byte <= 0x1F || byte > 0x7E {
            true
        } else {
            match set {
                PercentEncodeSet::C0Control => false,
                PercentEncodeSet::Fragment => matches!(byte, b' ' | b'"' | b'<' | b'>' | b'`'),
                PercentEncodeSet::Query => matches!(byte, b' ' | b'"' | b'#' | b'<' | b'>'),
                PercentEncodeSet::SpecialQuery => matches!(byte, b' ' | b'"' | b'#' | b'<' | b'>' | b'\''),
                PercentEncodeSet::Path => matches!(
                    byte,
                    b' ' | b'"' | b'#' | b'<' | b'>' | b'?' | b'^' | b'`' | b'{' | b'}'
                ),
                PercentEncodeSet::Userinfo => matches!(
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
                PercentEncodeSet::Component => matches!(
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
        };

        if should_encode {
            let word = (byte >> 6) as usize;
            let bit = byte & 63;
            map[word] |= 1u64 << bit;
        }
        b += 1;
    }
    map
}

static ENCODE_BITMAPS: [[u64; 4]; 7] = [
    make_encode_bitmap(PercentEncodeSet::C0Control),
    make_encode_bitmap(PercentEncodeSet::Fragment),
    make_encode_bitmap(PercentEncodeSet::Query),
    make_encode_bitmap(PercentEncodeSet::SpecialQuery),
    make_encode_bitmap(PercentEncodeSet::Path),
    make_encode_bitmap(PercentEncodeSet::Userinfo),
    make_encode_bitmap(PercentEncodeSet::Component),
];

impl PercentEncodeSet {
    /// Consulta $O(1)$ sem branches se o byte deve ser escapado neste conjunto.
    #[inline(always)]
    pub const fn contains(self, byte: u8) -> bool {
        let map = ENCODE_BITMAPS[self as usize];
        (map[(byte >> 6) as usize] & (1u64 << (byte & 63))) != 0
    }
}

/// Retorna a representação percent-encoded de 3 bytes (`%XX`) caso o caractere precise ser escapado.
#[inline(always)]
pub const fn percent_encode_byte(byte: u8, set: PercentEncodeSet) -> Option<[u8; 3]> {
    if set.contains(byte) {
        let hi = HEX_CHARS[(byte >> 4) as usize];
        let lo = HEX_CHARS[(byte & 0x0F) as usize];
        Some([b'%', hi, lo])
    } else {
        None
    }
}

/// Codifica uma string UTF-8 evitando qualquer re-verificação de UTF-8 no fast path.
pub fn percent_encode(input: &str, set: PercentEncodeSet) -> String {
    let bytes = input.as_bytes();
    if !bytes.iter().any(|&b| set.contains(b)) {
        return input.to_string();
    }
    percent_encode_bytes(bytes, set)
}

/// Codifica uma fatia de bytes brutos com alocação e cópia direta em buffer de bytes.
pub fn percent_encode_bytes(bytes: &[u8], set: PercentEncodeSet) -> String {
    let has_encoded = bytes.iter().any(|&b| set.contains(b));
    if !has_encoded {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    let mut output = Vec::with_capacity(bytes.len() + 16);
    for &byte in bytes {
        if let Some(encoded) = percent_encode_byte(byte, set) {
            output.extend_from_slice(&encoded);
        } else {
            output.push(byte);
        }
    }

    // SAFETY: Todos os bytes escapados são ASCII (%XX) e bytes não-escapados preservam UTF-8 válido
    unsafe { String::from_utf8_unchecked(output) }
}
