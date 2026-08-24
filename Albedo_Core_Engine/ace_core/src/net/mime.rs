//! # MIME Types e WHATWG MIME Sniffing Standard
//!
//! Tipagem de mídias da web, constantes estáticas pré-alocadas e identificação automática de tipos MIME a partir de assinaturas de bytes.

use crate::error::AceError;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::fmt;

/// Representação estruturada de um MIME Type (Content-Type) conforme a especificação WHATWG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeType {
    /// O tipo primário em minúsculas (ex: "text", "image", "application").
    pub type_: SmolStr,
    /// O subtipo em minúsculas (ex: "html", "png", "javascript").
    pub subtype: SmolStr,
    /// Parâmetros adicionais (ex: charset="utf-8", boundary="...").
    pub parameters: FxHashMap<SmolStr, SmolStr>,
}

impl MimeType {
    /// Cria um novo `MimeType` a partir de tipo e subtipo.
    pub fn new(type_: impl Into<SmolStr>, subtype: impl Into<SmolStr>) -> Self {
        Self {
            type_: type_.into().to_ascii_lowercase().into(),
            subtype: subtype.into().to_ascii_lowercase().into(),
            parameters: FxHashMap::default(),
        }
    }

    /// Retorna a essência do MIME Type no formato `type/subtype` (ex: "text/html").
    pub fn essence(&self) -> String {
        format!("{}/{}", self.type_, self.subtype)
    }

    /// Retorna o valor de um parâmetro opcional (ex: "charset").
    pub fn get_parameter(&self, name: &str) -> Option<&str> {
        let key: SmolStr = name.to_ascii_lowercase().into();
        self.parameters.get(&key).map(|v| v.as_str())
    }

    /// Alias conveniente para `get_parameter`.
    #[inline]
    pub fn get_param(&self, name: &str) -> Option<&str> {
        self.get_parameter(name)
    }

    // --- Constantes de Conveniência ---
    pub fn text_html() -> Self {
        Self::new("text", "html")
    }

    pub fn text_css() -> Self {
        Self::new("text", "css")
    }

    pub fn text_plain() -> Self {
        Self::new("text", "plain")
    }

    pub fn application_javascript() -> Self {
        Self::new("application", "javascript")
    }

    pub fn application_json() -> Self {
        Self::new("application", "json")
    }

    pub fn application_octet_stream() -> Self {
        Self::new("application", "octet-stream")
    }

    pub fn image_png() -> Self {
        Self::new("image", "png")
    }

    pub fn image_jpeg() -> Self {
        Self::new("image", "jpeg")
    }

    pub fn image_gif() -> Self {
        Self::new("image", "gif")
    }

    pub fn image_webp() -> Self {
        Self::new("image", "webp")
    }

    pub fn image_avif() -> Self {
        Self::new("image", "avif")
    }

    pub fn image_bmp() -> Self {
        Self::new("image", "bmp")
    }

    pub fn image_ico() -> Self {
        Self::new("image", "x-icon")
    }

    pub fn image_tiff() -> Self {
        Self::new("image", "tiff")
    }

    pub fn image_svg() -> Self {
        Self::new("image", "svg+xml")
    }

    pub fn font_woff() -> Self {
        Self::new("font", "woff")
    }

    pub fn font_woff2() -> Self {
        Self::new("font", "woff2")
    }

    pub fn font_ttf() -> Self {
        Self::new("font", "ttf")
    }

    pub fn font_otf() -> Self {
        Self::new("font", "otf")
    }

    pub fn video_mp4() -> Self {
        Self::new("video", "mp4")
    }

    pub fn video_webm() -> Self {
        Self::new("video", "webm")
    }

    pub fn audio_mpeg() -> Self {
        Self::new("audio", "mpeg")
    }

    pub fn audio_wav() -> Self {
        Self::new("audio", "wav")
    }

    /// Analisa uma string de cabeçalho `Content-Type` conforme o algoritmo da especificação WHATWG.
    pub fn parse(input: &str) -> Result<Self, AceError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AceError::invalid_op("MIME type string vazia"));
        }

        let mut parts = trimmed.split(';');
        let main_part = parts.next().unwrap_or("").trim();

        let mut type_subtype = main_part.splitn(2, '/');
        let type_ = type_subtype
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let subtype = type_subtype
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if type_.is_empty() || subtype.is_empty() {
            return Err(AceError::invalid_op(format!(
                "MIME type malformado: '{}'",
                input
            )));
        }

        let mut parameters = FxHashMap::default();
        for param in parts {
            let mut kv = param.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                let key = k.trim().to_ascii_lowercase();
                let mut val = v.trim();
                if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                    val = &val[1..val.len() - 1];
                }
                if !key.is_empty() {
                    parameters.insert(SmolStr::new(key), SmolStr::new(val));
                }
            }
        }

        Ok(Self {
            type_: SmolStr::new(type_),
            subtype: SmolStr::new(subtype),
            parameters,
        })
    }

    /// Retorna o charset configurado nos parâmetros, se presente (ex: "utf-8").
    #[inline]
    pub fn charset(&self) -> Option<&str> {
        self.parameters.get("charset").map(|s| s.as_str())
    }

    /// Retorna `true` se este MIME type representar um documento HTML.
    #[inline]
    pub fn is_html(&self) -> bool {
        self.type_ == "text" && self.subtype == "html"
    }

    /// Retorna `true` se este MIME type representar uma folha de estilo CSS.
    #[inline]
    pub fn is_css(&self) -> bool {
        self.type_ == "text" && self.subtype == "css"
    }

    /// Retorna `true` se este MIME type representar código JavaScript.
    #[inline]
    pub fn is_javascript(&self) -> bool {
        (self.type_ == "application"
            && (self.subtype == "javascript"
                || self.subtype == "x-javascript"
                || self.subtype == "ecmascript"))
            || (self.type_ == "text"
                && (self.subtype == "javascript"
                    || self.subtype == "ecmascript"
                    || self.subtype == "jscript"))
    }

    /// Retorna `true` se este MIME type representar uma imagem suportada.
    #[inline]
    pub fn is_image(&self) -> bool {
        self.type_ == "image"
    }

    /// Retorna `true` se este MIME type representar uma fonte web (WOFF, WOFF2, TTF, OTF).
    #[inline]
    pub fn is_font(&self) -> bool {
        self.type_ == "font"
            || (self.type_ == "application"
                && (self.subtype.starts_with("font-") || self.subtype == "font-woff"))
    }

    /// Retorna `true` se este MIME type representar dados JSON.
    #[inline]
    pub fn is_json(&self) -> bool {
        self.subtype == "json" || self.subtype.ends_with("+json")
    }

    /// Retorna `true` se este MIME type representar dados XML ou SVG.
    #[inline]
    pub fn is_xml(&self) -> bool {
        self.subtype == "xml" || self.subtype.ends_with("+xml")
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.type_, self.subtype)?;
        for (k, v) in &self.parameters {
            write!(f, "; {}={}", k, v)?;
        }
        Ok(())
    }
}

impl std::str::FromStr for MimeType {
    type Err = AceError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Identifica (sniff) o MIME type provável a partir dos bytes iniciais do recurso (Magic Numbers / WHATWG Sniffing).
#[inline]
pub fn sniff_mime_type(bytes: &[u8]) -> &'static str {
    let len = bytes.len();
    if len == 0 {
        return "text/plain";
    }

    // 1. Matching otimizado de inteiros 64-bit / 32-bit (Magic Numbers)
    if len >= 8 {
        let first8 = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let first4 = (first8 >> 32) as u32;

        // PNG: \x89PNG\r\n\x1a\n (0x89504E470D0A1A0A)
        if first8 == 0x8950_4E47_0D0A_1A0A {
            return "image/png";
        }

        // GIF87a / GIF89a: 0x474946383761... / 0x474946383961...
        let first6 = first8 >> 16;
        if first6 == 0x4749_4638_3761 || first6 == 0x4749_4638_3961 {
            return "image/gif";
        }

        // PDF: %PDF- (0x255044462D)
        if (first8 >> 24) == 0x0025_5044_462D {
            return "application/pdf";
        }

        // RIFF Container (WEBP / WAVE)
        if first4 == 0x5249_4646 { // "RIFF"
            if len >= 12 {
                let tag = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
                if tag == 0x5745_4250 { // "WEBP"
                    return "image/webp";
                }
                if tag == 0x5741_5645 { // "WAVE"
                    return "audio/wav";
                }
            }
        }

        // FTYP Box (MP4 / AVIF)
        let ftyp = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        if ftyp == 0x6674_7970 { // "ftyp"
            if len >= 12 {
                let brand = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
                if brand == 0x6176_6966 || brand == 0x6176_6973 { // "avif" / "avis"
                    return "image/avif";
                }
            }
            return "video/mp4";
        }

        // Fontes Web
        if first4 == 0x774F_4646 { // "wOFF"
            return "font/woff";
        }
        if first4 == 0x774F_4632 { // "wOF2"
            return "font/woff2";
        }
        if first4 == 0x0001_0000 || first4 == 0x7472_7565 || first4 == 0x7479_7031 { // "\0\1\0\0" | "true" | "typ1"
            return "font/ttf";
        }
        if first4 == 0x4F54_544F { // "OTTO"
            return "font/otf";
        }

        // WebM: 0x1A45DFA3
        if first4 == 0x1A45_DFA3 {
            return "video/webm";
        }

        // Icon (.ico): 0x00000100 | 0x00000200
        if first4 == 0x0000_0100 || first4 == 0x0000_0200 {
            return "image/x-icon";
        }

        // TIFF: "II*\0" (0x49492A00) | "MM\0*" (0x4D4D002A)
        if first4 == 0x4949_2A00 || first4 == 0x4D4D_002A {
            return "image/tiff";
        }

        // JPEG: \xFF\xD8\xFF
        if (first8 >> 40) == 0xFF_D8_FF {
            return "image/jpeg";
        }

        // ID3 (MP3)
        if (first8 >> 40) == 0x49_44_33 {
            return "audio/mpeg";
        }

        // BMP: "BM" (0x424D)
        if (first8 >> 48) == 0x42_4D {
            return "image/bmp";
        }
    } else {
        // Buffers curtos (< 8 bytes)
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return "image/gif";
        }
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return "image/png";
        }
        if bytes.starts_with(b"\xFF\xD8\xFF") {
            return "image/jpeg";
        }
        if bytes.starts_with(b"BM") {
            return "image/bmp";
        }
        if bytes.starts_with(b"\x00\x00\x01\x00") || bytes.starts_with(b"\x00\x00\x02\x00") {
            return "image/x-icon";
        }
        if bytes.starts_with(b"II*\x00") || bytes.starts_with(b"MM\x00*") {
            return "image/tiff";
        }
        if bytes.starts_with(b"wOFF") {
            return "font/woff";
        }
        if bytes.starts_with(b"wOF2") {
            return "font/woff2";
        }
        if bytes.starts_with(b"\x00\x01\x00\x00") || bytes.starts_with(b"true") || bytes.starts_with(b"typ1") {
            return "font/ttf";
        }
        if bytes.starts_with(b"OTTO") {
            return "font/otf";
        }
        if bytes.starts_with(b"%PDF-") {
            return "application/pdf";
        }
        if bytes.starts_with(b"\x1A\x45\xDF\xA3") {
            return "video/webm";
        }
        if bytes.starts_with(b"ID3") {
            return "audio/mpeg";
        }
    }

    if is_mp3_frame_header(bytes) {
        return "audio/mpeg";
    }

    // 2. Sniffing de texto/HTML/SVG Zero-Allocation com validação de fronteira UTF-8
    let slice = &bytes[..len.min(512)];
    let sample = match std::str::from_utf8(slice) {
        Ok(s) => s.trim_start(),
        Err(e) => {
            let valid_len = e.valid_up_to();
            if valid_len > 0 {
                match std::str::from_utf8(&slice[..valid_len]) {
                    Ok(s) => s.trim_start(),
                    Err(_) => return "application/octet-stream",
                }
            } else {
                return "application/octet-stream";
            }
        }
    };

    // Zero-allocation case-insensitive checking (sem alocar String temporária de 512 bytes!)
    if starts_with_ignore_ascii_case(sample, "<!doctype html")
        || starts_with_ignore_ascii_case(sample, "<html")
        || starts_with_ignore_ascii_case(sample, "<head")
        || starts_with_ignore_ascii_case(sample, "<body")
        || starts_with_ignore_ascii_case(sample, "<title")
    {
        return "text/html";
    }

    if contains_ignore_ascii_case(sample, "<svg")
        || contains_ignore_ascii_case(sample, "<!doctype svg")
        || (starts_with_ignore_ascii_case(sample, "<?xml") && contains_ignore_ascii_case(sample, "<svg"))
    {
        return "image/svg+xml";
    }

    if starts_with_ignore_ascii_case(sample, "<?xml") {
        return "application/xml";
    }

    "text/plain"
}

#[inline(always)]
fn starts_with_ignore_ascii_case(haystack: &str, prefix: &str) -> bool {
    haystack.len() >= prefix.len()
        && haystack.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

#[inline(always)]
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    if haystack.len() < n.len() {
        return false;
    }
    haystack.as_bytes().windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Valida um cabeçalho de frame de áudio MPEG (MP3) conforme as restrições da especificação WHATWG MIME Sniffing.
#[inline]
fn is_mp3_frame_header(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    let b0 = bytes[0];
    let b1 = bytes[1];
    let b2 = bytes[2];
    let b3 = bytes[3];

    // Sync: 11 bits em 1 (0xFFE0)
    if b0 != 0xFF || (b1 & 0xE0) != 0xE0 {
        return false;
    }
    // MPEG Version != reserved (0b01)
    if (b1 & 0x18) == 0x08 {
        return false;
    }
    // Layer != reserved (0b00)
    if (b1 & 0x06) == 0x00 {
        return false;
    }
    // Bitrate index: 0b1111 (15) e 0b0000 (0) são inválidos
    let bitrate = (b2 & 0xF0) >> 4;
    if bitrate == 0x0F || bitrate == 0x00 {
        return false;
    }
    // Sampling rate index: 0b11 (3) é reservado
    if (b2 & 0x0C) == 0x0C {
        return false;
    }
    // Emphasis: 0b10 (2) é reservado
    if (b3 & 0x03) == 0x02 {
        return false;
    }

    true
}
