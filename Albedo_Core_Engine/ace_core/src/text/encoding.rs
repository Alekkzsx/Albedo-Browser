//! # Encodings de Texto Web e Detecção de BOM (WHATWG Encoding Standard)
//!
//! Identificação de encodings legados da web e detecção automática de Byte Order Mark (BOM).

/// Encodings canônicos suportados pelo padrão WHATWG Encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WebEncoding {
    #[default]
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
    Iso8859_1,
    Gbk,
    ShiftJis,
    EucKr,
}

impl WebEncoding {
    /// Retorna o nome canônico do encoding (ex: `"UTF-8"`).
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::Windows1252 => "windows-1252",
            Self::Iso8859_1 => "ISO-8859-1",
            Self::Gbk => "GBK",
            Self::ShiftJis => "Shift_JIS",
            Self::EucKr => "EUC-KR",
        }
    }

    /// Retorna `true` se o encoding pertencer à família Unicode.
    #[inline]
    pub const fn is_unicode(&self) -> bool {
        matches!(self, Self::Utf8 | Self::Utf16Le | Self::Utf16Be)
    }

    /// Mapeia uma string de rótulo (label) para o encoding canônico correspondente.
    pub fn from_label(label: &str) -> Option<Self> {
        let trimmed = label.trim().to_ascii_lowercase();
        match trimmed.as_str() {
            "utf-8" | "utf8" | "unicode-1-1-utf-8" => Some(Self::Utf8),
            "utf-16le" | "utf-16" => Some(Self::Utf16Le),
            "utf-16be" => Some(Self::Utf16Be),
            "windows-1252" | "ansi_x3.4-1968" | "cp1252" => Some(Self::Windows1252),
            "iso-8859-1" | "latin1" | "l1" => Some(Self::Iso8859_1),
            "gbk" | "gb2312" | "chinese" => Some(Self::Gbk),
            "shift_jis" | "sjis" | "ms932" => Some(Self::ShiftJis),
            "euc-kr" | "korean" => Some(Self::EucKr),
            _ => None,
        }
    }
}

/// Inspeciona os bytes iniciais de um buffer para detectar a presença de um Byte Order Mark (BOM).
///
/// Se um BOM for encontrado, retorna a tupla `(WebEncoding, usize)` contendo o encoding detectado
/// e a quantidade de bytes do BOM a serem pulados no fluxo de parsing.
pub fn detect_bom(bytes: &[u8]) -> Option<(WebEncoding, usize)> {
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        Some((WebEncoding::Utf8, 3))
    } else if bytes.starts_with(b"\xFF\xFE") {
        Some((WebEncoding::Utf16Le, 2))
    } else if bytes.starts_with(b"\xFE\xFF") {
        Some((WebEncoding::Utf16Be, 2))
    } else {
        None
    }
}
