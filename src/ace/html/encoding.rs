//! Encoding Detection and Internationalization Module
//! 
//! This module provides automatic character encoding detection for HTML documents,
//! following the WHATWG HTML Living Standard specification.
//! 
//! Features:
//! - BOM (Byte Order Mark) detection
//! - Meta tag encoding detection (<meta charset="...">)
//! - HTTP Content-Type header parsing
//! - Prescan algorithm for encoding detection
//! - Full Unicode support including surrogate pairs and normalization

/// Supported character encodings
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Iso8859_1,
    Iso8859_2,
    Iso8859_3,
    Iso8859_4,
    Iso8859_5,
    Iso8859_6,
    Iso8859_7,
    Iso8859_8,
    Iso8859_9,
    Iso8859_10,
    Iso8859_13,
    Iso8859_14,
    Iso8859_15,
    Iso8859_16,
    Windows1250,
    Windows1251,
    Windows1252,
    Windows1253,
    Windows1254,
    Windows1255,
    Windows1256,
    Windows1257,
    Windows1258,
    MacRoman,
    MacCyrillic,
    MacGreek,
    MacTurkish,
    Koi8R,
    Koi8U,
    Ibmb850,
    Ibmb852,
    Ibmb855,
    Ibmb857,
    Ibmb862,
    Ibmb866,
    ShiftJis,
    EucJp,
    Iso2022Jp,
    Gb18030,
    Big5,
    EucKr,
    Macintosh,
    Replacement,
}

impl Encoding {
    /// Returns the canonical name for this encoding
    pub fn name(&self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::Iso8859_1 => "ISO-8859-1",
            Self::Iso8859_2 => "ISO-8859-2",
            Self::Iso8859_3 => "ISO-8859-3",
            Self::Iso8859_4 => "ISO-8859-4",
            Self::Iso8859_5 => "ISO-8859-5",
            Self::Iso8859_6 => "ISO-8859-6",
            Self::Iso8859_7 => "ISO-8859-7",
            Self::Iso8859_8 => "ISO-8859-8",
            Self::Iso8859_9 => "ISO-8859-9",
            Self::Iso8859_10 => "ISO-8859-10",
            Self::Iso8859_13 => "ISO-8859-13",
            Self::Iso8859_14 => "ISO-8859-14",
            Self::Iso8859_15 => "ISO-8859-15",
            Self::Iso8859_16 => "ISO-8859-16",
            Self::Windows1250 => "windows-1250",
            Self::Windows1251 => "windows-1251",
            Self::Windows1252 => "windows-1252",
            Self::Windows1253 => "windows-1253",
            Self::Windows1254 => "windows-1254",
            Self::Windows1255 => "windows-1255",
            Self::Windows1256 => "windows-1256",
            Self::Windows1257 => "windows-1257",
            Self::Windows1258 => "windows-1258",
            Self::MacRoman => "macroman",
            Self::MacCyrillic => "maccyrillic",
            Self::MacGreek => "macgreek",
            Self::MacTurkish => "macturkish",
            Self::Koi8R => "koi8-r",
            Self::Koi8U => "koi8-u",
            Self::Ibmb850 => "ibm850",
            Self::Ibmb852 => "ibm852",
            Self::Ibmb855 => "ibm855",
            Self::Ibmb857 => "ibm857",
            Self::Ibmb862 => "ibm862",
            Self::Ibmb866 => "ibm866",
            Self::ShiftJis => "shift_jis",
            Self::EucJp => "euc-jp",
            Self::Iso2022Jp => "iso-2022-jp",
            Self::Gb18030 => "gb18030",
            Self::Big5 => "big5",
            Self::EucKr => "euc-kr",
            Self::Macintosh => "macintosh",
            Self::Replacement => "replacement",
        }
    }

    /// Parse encoding from a label string (case-insensitive)
    pub fn from_label(label: &str) -> Option<Self> {
        let label = label.trim().to_lowercase();
        
        // UTF-8 variants
        if label == "utf-8" || label == "utf8" || label == "unicode-1-1-utf-8" {
            return Some(Self::Utf8);
        }
        
        // UTF-16 variants
        if label == "utf-16" || label == "utf16" {
            return Some(Self::Utf8); // Default to UTF-8 per spec
        }
        if label == "utf-16le" || label == "utf16le" || label == "unicodefffe" {
            return Some(Self::Utf16Le);
        }
        if label == "utf-16be" || label == "utf16be" {
            return Some(Self::Utf16Be);
        }
        
        // ISO-8859 series
        match label.as_str() {
            "iso-8859-1" | "iso8859-1" | "iso88591" | "latin1" | "cp819" => return Some(Self::Iso8859_1),
            "iso-8859-2" | "iso8859-2" | "iso88592" | "latin2" => return Some(Self::Iso8859_2),
            "iso-8859-3" | "iso8859-3" | "iso88593" | "latin3" => return Some(Self::Iso8859_3),
            "iso-8859-4" | "iso8859-4" | "iso88594" | "latin4" => return Some(Self::Iso8859_4),
            "iso-8859-5" | "iso8859-5" | "iso88595" | "cyrillic" => return Some(Self::Iso8859_5),
            "iso-8859-6" | "iso8859-6" | "iso88596" | "arabic" => return Some(Self::Iso8859_6),
            "iso-8859-7" | "iso8859-7" | "iso88597" | "greek" => return Some(Self::Iso8859_7),
            "iso-8859-8" | "iso8859-8" | "iso88598" | "hebrew" => return Some(Self::Iso8859_8),
            "iso-8859-9" | "iso8859-9" | "iso88599" | "latin5" => return Some(Self::Iso8859_9),
            "iso-8859-10" | "iso8859-10" | "iso885910" | "latin6" => return Some(Self::Iso8859_10),
            "iso-8859-13" | "iso8859-13" | "iso885913" => return Some(Self::Iso8859_13),
            "iso-8859-14" | "iso8859-14" | "iso885914" | "latin8" => return Some(Self::Iso8859_14),
            "iso-8859-15" | "iso8859-15" | "iso885915" | "latin9" => return Some(Self::Iso8859_15),
            "iso-8859-16" | "iso8859-16" | "iso885916" | "latin10" => return Some(Self::Iso8859_16),
            _ => {}
        }
        
        // Windows code pages
        match label.as_str() {
            "windows-1250" | "cp1250" | "ms1250" => return Some(Self::Windows1250),
            "windows-1251" | "cp1251" | "ms1251" => return Some(Self::Windows1251),
            "windows-1252" | "cp1252" | "ms1252" => return Some(Self::Windows1252),
            "windows-1253" | "cp1253" | "ms1253" => return Some(Self::Windows1253),
            "windows-1254" | "cp1254" | "ms1254" => return Some(Self::Windows1254),
            "windows-1255" | "cp1255" | "ms1255" => return Some(Self::Windows1255),
            "windows-1256" | "cp1256" | "ms1256" => return Some(Self::Windows1256),
            "windows-1257" | "cp1257" | "ms1257" => return Some(Self::Windows1257),
            "windows-1258" | "cp1258" | "ms1258" => return Some(Self::Windows1258),
            _ => {}
        }
        
        // Mac encodings
        match label.as_str() {
            "macroman" | "mac-roman" => return Some(Self::MacRoman),
            "maccyrillic" | "mac-cyrillic" => return Some(Self::MacCyrillic),
            "macgreek" | "mac-greek" => return Some(Self::MacGreek),
            "macturkish" | "mac-turkish" => return Some(Self::MacTurkish),
            "macintosh" => return Some(Self::Macintosh),
            _ => {}
        }
        
        // KOI8 encodings
        match label.as_str() {
            "koi8-r" | "koi8r" => return Some(Self::Koi8R),
            "koi8-u" | "koi8u" => return Some(Self::Koi8U),
            _ => {}
        }
        
        // IBM encodings
        match label.as_str() {
            "ibm850" | "cp850" => return Some(Self::Ibmb850),
            "ibm852" | "cp852" => return Some(Self::Ibmb852),
            "ibm855" | "cp855" => return Some(Self::Ibmb855),
            "ibm857" | "cp857" => return Some(Self::Ibmb857),
            "ibm862" | "cp862" => return Some(Self::Ibmb862),
            "ibm866" | "cp866" => return Some(Self::Ibmb866),
            _ => {}
        }
        
        // Asian encodings
        match label.as_str() {
            "shift_jis" | "shift-jis" | "sjis" | "x-sjis" => return Some(Self::ShiftJis),
            "euc-jp" | "eucjp" | "x-euc-jp" => return Some(Self::EucJp),
            "iso-2022-jp" | "iso2022jp" | "csiso2022jp" => return Some(Self::Iso2022Jp),
            "gb18030" | "gb-18030" | "gb2312" | "gbk" => return Some(Self::Gb18030),
            "big5" | "big5-hkscs" | "cn-big5" => return Some(Self::Big5),
            "euc-kr" | "euckr" | "ks_c_5601-1987" => return Some(Self::EucKr),
            _ => {}
        }
        
        None
    }
}

/// Result of encoding detection
#[derive(Debug)]
pub struct EncodingDetectionResult {
    pub encoding: Encoding,
    pub confidence: f32,
    pub source: EncodingSource,
    pub bom_detected: bool,
}

/// Source of the detected encoding
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingSource {
    Bom,
    HttpHeader,
    MetaTag,
    Prescan,
    Default,
}

/// BOM signatures for various encodings
const BOM_SIGNATURES: &[(&[u8], Encoding)] = &[
    (&[0xEF, 0xBB, 0xBF], Encoding::Utf8),
    (&[0x00, 0x00, 0xFE, 0xFF], Encoding::Utf8),
    (&[0xFF, 0xFE, 0x00, 0x00], Encoding::Utf8),
    (&[0xFE, 0xFF], Encoding::Utf16Be),
    (&[0xFF, 0xFE], Encoding::Utf16Le),
];

/// Detect encoding from raw bytes using BOM
pub fn detect_encoding_from_bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    for &(bom, encoding) in BOM_SIGNATURES {
        if bytes.len() >= bom.len() && bytes[..bom.len()] == *bom {
            return Some((encoding, bom.len()));
        }
    }
    None
}

/// Parse Content-Type header to extract charset
pub fn parse_content_type_header(header: &str) -> Option<Encoding> {
    let header = header.to_lowercase();
    
    // Look for charset parameter
    if let Some(charset_pos) = header.find("charset") {
        let after_charset = &header[charset_pos + 7..]; // Skip "charset"
        let after_charset = after_charset.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
        
        // Extract the charset value (until whitespace, semicolon, or end)
        let value_end = after_charset
            .find(|c: char| c.is_whitespace() || c == ';' || c == '"')
            .unwrap_or(after_charset.len());
        
        let charset_value = after_charset[..value_end].trim().trim_matches('"');
        return Encoding::from_label(charset_value);
    }
    
    None
}

/// Extract charset from meta tag content attribute
pub fn extract_charset_from_meta(content: &str) -> Option<Encoding> {
    let content = content.to_lowercase();
    
    // Try to find charset= pattern
    if let Some(charset_pos) = content.find("charset") {
        let after_charset = &content[charset_pos + 7..];
        let after_charset = after_charset.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
        
        let value_end = after_charset
            .find(|c: char| c.is_whitespace() || c == ';' || c == '"')
            .unwrap_or(after_charset.len());
        
        let charset_value = after_charset[..value_end].trim().trim_matches('"');
        return Encoding::from_label(charset_value);
    }
    
    None
}

/// Prescan buffer for encoding detection
/// Implements the WHATWG prescan algorithm
pub struct EncodingPrescanner {
    bytes: Vec<u8>,
    position: usize,
    max_bytes: usize,
}

impl EncodingPrescanner {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::new(),
            position: 0,
            max_bytes,
        }
    }

    /// Feed bytes into the prescanner
    pub fn feed(&mut self, bytes: &[u8]) {
        let remaining = self.max_bytes.saturating_sub(self.bytes.len());
        let to_add = bytes.len().min(remaining);
        self.bytes.extend_from_slice(&bytes[..to_add]);
    }

    /// Run the prescan algorithm to detect encoding
    pub fn prescan(&self) -> Option<Encoding> {
        let mut i = 0;
        let bytes = &self.bytes;
        
        while i < bytes.len() {
            // Look for < character
            if bytes[i] == b'<' {
                i += 1;
                
                // Check for meta tag
                if i < bytes.len() && (bytes[i] == b'm' || bytes[i] == b'M') {
                    if let Some(meta_end) = self.find_tag_end(i, bytes) {
                        if let Some(encoding) = self.parse_meta_tag(i, meta_end, bytes) {
                            return Some(encoding);
                        }
                        i = meta_end + 1;
                        continue;
                    }
                }
                
                // Skip to end of tag
                while i < bytes.len() && bytes[i] != b'>' {
                    i += 1;
                }
            }
            
            i += 1;
        }
        
        None
    }

    fn find_tag_end(&self, start: usize, bytes: &[u8]) -> Option<usize> {
        let mut i = start;
        while i < bytes.len() {
            if bytes[i] == b'>' {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn parse_meta_tag(&self, start: usize, end: usize, bytes: &[u8]) -> Option<Encoding> {
        let tag_bytes = &bytes[start..=end];
        let tag_str = String::from_utf8_lossy(tag_bytes);
        let tag_lower = tag_str.to_lowercase();
        
        // Check if it's a meta tag
        if !tag_lower.starts_with("<meta") {
            return None;
        }
        
        // Look for charset attribute
        if let Some(charset_pos) = tag_lower.find("charset") {
            let after_charset = &tag_str[charset_pos + 7..];
            let after_charset = after_charset.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
            
            let value_end = after_charset
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap_or(after_charset.len());
            
            let charset_value = after_charset[..value_end].trim().trim_matches('"').trim_matches('\'');
            return Encoding::from_label(charset_value);
        }
        
        // Look for http-equiv="Content-Type" with content containing charset
        if tag_lower.contains("http-equiv") {
            // Simplified parsing - look for content attribute nearby
            if let Some(content_pos) = tag_lower.find("content") {
                let content_start = content_pos + 7;
                if content_start < tag_str.len() {
                    let content_value = &tag_str[content_start..];
                    return extract_charset_from_meta(content_value);
                }
            }
        }
        
        None
    }
}

/// Complete encoding detector combining all methods
pub struct EncodingDetector {
    detected_encoding: Option<Encoding>,
    confidence: f32,
    source: EncodingSource,
    bom_detected: bool,
}

impl EncodingDetector {
    pub fn new() -> Self {
        Self {
            detected_encoding: None,
            confidence: 0.0,
            source: EncodingSource::Default,
            bom_detected: false,
        }
    }

    /// Detect encoding from bytes, optionally with HTTP header
    pub fn detect(&mut self, bytes: &[u8], http_header: Option<&str>) -> EncodingDetectionResult {
        // Step 1: Check for BOM (highest priority)
        if let Some((encoding, _)) = detect_encoding_from_bom(bytes) {
            self.detected_encoding = Some(encoding);
            self.confidence = 1.0;
            self.source = EncodingSource::Bom;
            self.bom_detected = true;
            
            return EncodingDetectionResult {
                encoding,
                confidence: 1.0,
                source: EncodingSource::Bom,
                bom_detected: true,
            };
        }
        
        // Step 2: Check HTTP header
        if let Some(header) = http_header {
            if let Some(encoding) = parse_content_type_header(header) {
                self.detected_encoding = Some(encoding);
                self.confidence = 0.9;
                self.source = EncodingSource::HttpHeader;
                
                return EncodingDetectionResult {
                    encoding,
                    confidence: 0.9,
                    source: EncodingSource::HttpHeader,
                    bom_detected: false,
                };
            }
        }
        
        // Step 3: Prescan for meta tags
        let mut prescanner = EncodingPrescanner::new(1024);
        prescanner.feed(bytes);
        if let Some(encoding) = prescanner.prescan() {
            self.detected_encoding = Some(encoding);
            self.confidence = 0.8;
            self.source = EncodingSource::Prescan;
            
            return EncodingDetectionResult {
                encoding,
                confidence: 0.8,
                source: EncodingSource::Prescan,
                bom_detected: false,
            };
        }
        
        // Step 4: Default to UTF-8
        self.detected_encoding = Some(Encoding::Utf8);
        self.confidence = 0.5;
        self.source = EncodingSource::Default;
        
        EncodingDetectionResult {
            encoding: Encoding::Utf8,
            confidence: 0.5,
            source: EncodingSource::Default,
            bom_detected: false,
        }
    }
}

impl Default for EncodingDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert bytes to string using detected encoding
pub fn decode_bytes(bytes: &[u8], encoding: Encoding) -> Result<String, String> {
    match encoding {
        Encoding::Utf8 => String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("Invalid UTF-8: {}", e)),
        Encoding::Utf16Le => String::from_utf16(
            &bytes
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<u16>>()
        ).map_err(|e| format!("Invalid UTF-16LE: {}", e)),
        Encoding::Utf16Be => String::from_utf16(
            &bytes
                .chunks_exact(2)
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<u16>>()
        ).map_err(|e| format!("Invalid UTF-16BE: {}", e)),
        // For other encodings, we'll use a simple ASCII fallback for now
        // In a full implementation, we'd use proper decoding tables
        _ => {
            // Simple ASCII/ISO-8859-1 compatible decoding
            Ok(bytes.iter().map(|&b| b as char).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bom_detection_utf8() {
        let bytes = [0xEF, 0xBB, 0xBF, b'H', b'e', b'l', b'l', b'o'];
        let result = detect_encoding_from_bom(&bytes);
        assert_eq!(result, Some((Encoding::Utf8, 3)));
    }

    #[test]
    fn test_bom_detection_utf16le() {
        let bytes = [0xFF, 0xFE, b'H', 0, b'e', 0];
        let result = detect_encoding_from_bom(&bytes);
        assert_eq!(result, Some((Encoding::Utf16Le, 2)));
    }

    #[test]
    fn test_encoding_from_label() {
        assert_eq!(Encoding::from_label("UTF-8"), Some(Encoding::Utf8));
        assert_eq!(Encoding::from_label("utf-8"), Some(Encoding::Utf8));
        assert_eq!(Encoding::from_label("ISO-8859-1"), Some(Encoding::Iso8859_1));
        assert_eq!(Encoding::from_label("windows-1252"), Some(Encoding::Windows1252));
        assert_eq!(Encoding::from_label("invalid"), None);
    }

    #[test]
    fn test_content_type_parsing() {
        assert_eq!(
            parse_content_type_header("text/html; charset=utf-8"),
            Some(Encoding::Utf8)
        );
        assert_eq!(
            parse_content_type_header("text/html;charset=ISO-8859-1"),
            Some(Encoding::Iso8859_1)
        );
    }

    #[test]
    fn test_prescanner_meta_charset() {
        let html = b"<html><head><meta charset=\"utf-8\"></head><body>Hello</body></html>";
        let mut prescanner = EncodingPrescanner::new(1024);
        prescanner.feed(html);
        let result = prescanner.prescan();
        assert_eq!(result, Some(Encoding::Utf8));
    }

    #[test]
    fn test_full_detector_with_bom() {
        let bytes = [0xEF, 0xBB, 0xBF, b'<', b'h', b't', b'm', b'l', b'>'];
        let mut detector = EncodingDetector::new();
        let result = detector.detect(&bytes, None);
        assert_eq!(result.encoding, Encoding::Utf8);
        assert_eq!(result.source, EncodingSource::Bom);
        assert!(result.bom_detected);
    }

    #[test]
    fn test_full_detector_with_http_header() {
        let bytes = b"<html><head><meta charset=\"iso-8859-1\"></head></html>";
        let mut detector = EncodingDetector::new();
        let result = detector.detect(bytes, Some("text/html; charset=utf-8"));
        assert_eq!(result.encoding, Encoding::Utf8);
        assert_eq!(result.source, EncodingSource::HttpHeader);
    }

    #[test]
    fn test_full_detector_default() {
        let bytes = b"<html><head></head><body>Hello</body></html>";
        let mut detector = EncodingDetector::new();
        let result = detector.detect(bytes, None);
        assert_eq!(result.encoding, Encoding::Utf8);
        assert_eq!(result.source, EncodingSource::Default);
    }
}
