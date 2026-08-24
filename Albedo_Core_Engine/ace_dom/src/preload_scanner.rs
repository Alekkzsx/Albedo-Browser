//! # Preload Scanner Especulativo (Lookahead não-bloqueante de sub-recursos)
//!
//! Varre o stream HTML bruto à frente do parser principal para descobrir e disparar
//! requisições de rede antecipadas para folhas de estilo, scripts, imagens e fontes.

use smol_str::SmolStr;

/// Tipo de sub-recurso descoberto pelo scanner especulativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreloadKind {
    /// Folha de estilos CSS (`<link rel="stylesheet">`)
    Stylesheet,
    /// Script JavaScript (`<script src="...">`)
    Script,
    /// Imagem (`<img src="...">`)
    Image,
    /// Pré-carregamento explícito (`<link rel="preload">`)
    Preload,
    /// Mídia de áudio/vídeo (`<video poster="...">`, `<source src="...">`)
    Media,
}

/// Requisição de pré-carregamento de sub-recurso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreloadRequest {
    /// URL de destino do recurso.
    pub url: SmolStr,
    /// Categoria do recurso.
    pub kind: PreloadKind,
    /// Tipo de destino para preloads explícitos (`as="style"`, `as="script"`, `as="font"`).
    pub as_type: Option<SmolStr>,
    /// Condição de media query associada (`media="print"`, `media="screen"`).
    pub media: Option<SmolStr>,
}

/// O scanner de pré-carregamento especulativo.
#[derive(Debug, Default)]
pub struct PreloadScanner;

impl PreloadScanner {
    /// Cria um novo `PreloadScanner`.
    pub fn new() -> Self {
        Self
    }

    /// Varre uma fatia de texto buscando declarações de sub-recursos.
    pub fn scan(&self, input: &str) -> Vec<PreloadRequest> {
        let mut requests = Vec::new();
        let bytes = input.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'<' {
                i += 1;
                // Pula comentários
                if bytes[i..].starts_with(b"!--") {
                    if let Some(end) = input[i..].find("-->") {
                        i += end + 3;
                        continue;
                    }
                }

                // Lê o nome da tag
                let start_tag = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
                    i += 1;
                }
                let tag_name = &input[start_tag..i];

                if tag_name.eq_ignore_ascii_case("link")
                    || tag_name.eq_ignore_ascii_case("script")
                    || tag_name.eq_ignore_ascii_case("img")
                    || tag_name.eq_ignore_ascii_case("video")
                    || tag_name.eq_ignore_ascii_case("source")
                {
                    // Lê os atributos até o fechamento da tag '>'
                    let attr_start = i;
                    while i < bytes.len() && bytes[i] != b'>' {
                        i += 1;
                    }
                    let attr_str = &input[attr_start..i];
                    if let Some(req) = Self::parse_preload_attributes(tag_name, attr_str) {
                        requests.push(req);
                    }
                }
            } else {
                i += 1;
            }
        }

        requests
    }

    /// Faz o parsing dos atributos relevantes de uma tag para extração da URL de sub-recurso.
    fn parse_preload_attributes(tag: &str, attrs_raw: &str) -> Option<PreloadRequest> {
        let mut href = None;
        let mut src = None;
        let mut poster = None;
        let mut rel = None;
        let mut as_type = None;
        let mut media = None;

        let parts = Self::extract_attributes(attrs_raw);
        for (name, val) in parts {
            let n = name.to_ascii_lowercase();
            match n.as_str() {
                "href" => href = Some(val),
                "src" => src = Some(val),
                "poster" => poster = Some(val),
                "rel" => rel = Some(val),
                "as" => as_type = Some(val),
                "media" => media = Some(val),
                _ => {}
            }
        }

        if tag.eq_ignore_ascii_case("link") {
            let rel_str = rel?.to_ascii_lowercase();
            let url_str = href?;
            if rel_str.contains("stylesheet") {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Stylesheet,
                    as_type: None,
                    media: media.map(SmolStr::new),
                });
            } else if rel_str.contains("preload") {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Preload,
                    as_type: as_type.map(SmolStr::new),
                    media: media.map(SmolStr::new),
                });
            }
        } else if tag.eq_ignore_ascii_case("script") {
            if let Some(url_str) = src {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Script,
                    as_type: None,
                    media: None,
                });
            }
        } else if tag.eq_ignore_ascii_case("img") {
            if let Some(url_str) = src {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Image,
                    as_type: None,
                    media: None,
                });
            }
        } else if tag.eq_ignore_ascii_case("video") {
            if let Some(url_str) = poster {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Image,
                    as_type: None,
                    media: None,
                });
            }
        } else if tag.eq_ignore_ascii_case("source") {
            if let Some(url_str) = src {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Media,
                    as_type: None,
                    media: media.map(SmolStr::new),
                });
            }
        }

        None
    }

    /// Extrai pares (nome, valor) de atributos com suporte a aspas simples e duplas.
    fn extract_attributes(attrs_raw: &str) -> Vec<(String, String)> {
        let mut list = Vec::new();
        let bytes = attrs_raw.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            // Pula espaços
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }

            // Lê nome
            let name_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' && bytes[i] != b'/' && bytes[i] != b'>' {
                i += 1;
            }
            let name = attrs_raw[name_start..i].trim().to_string();
            if name.is_empty() {
                i += 1;
                continue;
            }

            // Pula espaços antes do '='
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            let mut val = String::new();
            if i < bytes.len() && bytes[i] == b'=' {
                i += 1; // Pula '='
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < bytes.len() {
                    let quote = bytes[i];
                    if quote == b'"' || quote == b'\'' {
                        i += 1;
                        let val_start = i;
                        while i < bytes.len() && bytes[i] != quote {
                            i += 1;
                        }
                        val = attrs_raw[val_start..i].to_string();
                        if i < bytes.len() {
                            i += 1; // Pula quote de fechamento
                        }
                    } else {
                        let val_start = i;
                        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                            i += 1;
                        }
                        val = attrs_raw[val_start..i].to_string();
                    }
                }
            }

            list.push((name, val));
        }

        list
    }
}
