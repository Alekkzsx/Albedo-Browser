//! # MIME Types e WHATWG MIME Sniffing Standard
//!
//! Tipagem de mídias da web e identificação automática de tipos MIME a partir de assinaturas de bytes.

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

    /// Analisa uma string de cabeçalho `Content-Type` conforme o algoritmo da especificação WHATWG.
    pub fn parse(input: &str) -> Result<Self, AceError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AceError::invalid_op("MIME type string vazia"));
        }

        let mut parts = trimmed.split(';');
        let main_part = parts.next().unwrap_or("").trim();

        let mut type_subtype = main_part.splitn(2, '/');
        let type_ = type_subtype.next().unwrap_or("").trim().to_ascii_lowercase();
        let subtype = type_subtype.next().unwrap_or("").trim().to_ascii_lowercase();

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
                // Remove aspas se presentes
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
        (self.type_ == "application" && (self.subtype == "javascript" || self.subtype == "x-javascript" || self.subtype == "ecmascript"))
            || (self.type_ == "text" && (self.subtype == "javascript" || self.subtype == "ecmascript" || self.subtype == "jscript"))
    }

    /// Retorna `true` se este MIME type representar uma imagem suportada.
    #[inline]
    pub fn is_image(&self) -> bool {
        self.type_ == "image"
    }

    /// Retorna `true` se este MIME type representar uma fonte web (WOFF, WOFF2, TTF, OTF).
    #[inline]
    pub fn is_font(&self) -> bool {
        self.type_ == "font" || (self.type_ == "application" && (self.subtype.starts_with("font-") || self.subtype == "font-woff"))
    }

    /// Retorna `true` se este MIME type representar dados JSON.
    #[inline]
    pub fn is_json(&self) -> bool {
        self.subtype == "json" || self.subtype.ends_with("+json")
    }

    /// Retorna `true` se este MIME type representar dados XML.
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

/// Identifica (sniff) o MIME type provável a partir dos bytes iniciais do recurso (Magic Numbers / WHATWG Sniffing).
pub fn sniff_mime_type(bytes: &[u8]) -> &'static str {
    if bytes.is_empty() {
        return "text/plain";
    }

    // Assinaturas mágicas de imagens
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "image/png";
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return "image/jpeg";
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return "image/gif";
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "image/webp";
    }
    if bytes.starts_with(b"wOFF") {
        return "font/woff";
    }
    if bytes.starts_with(b"wOF2") {
        return "font/woff2";
    }
    if bytes.starts_with(b"%PDF-") {
        return "application/pdf";
    }

    // Sniffing de texto/HTML (ignora espaços iniciais)
    let sample = match std::str::from_utf8(&bytes[..bytes.len().min(512)]) {
        Ok(s) => s.trim_start(),
        Err(_) => return "application/octet-stream",
    };

    let sample_lower = sample.to_ascii_lowercase();
    if sample_lower.starts_with("<!doctype html")
        || sample_lower.starts_with("<html")
        || sample_lower.starts_with("<head")
        || sample_lower.starts_with("<body")
        || sample_lower.starts_with("<title")
    {
        return "text/html";
    }

    if sample_lower.starts_with("<?xml") {
        return "application/xml";
    }

    "text/plain"
}
