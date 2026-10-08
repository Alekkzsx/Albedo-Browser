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

    /// Varre uma fatia de texto buscando declarações de sub-recursos com aceleração SIMD.
    pub fn scan(&self, input: &str) -> Vec<PreloadRequest> {
        let mut requests = Vec::new();
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Fast-path SIMD: salta diretamente até o próximo delimitador de tag '<'
            if let Some(pos) = memchr::memchr(b'<', &bytes[i..]) {
                i += pos + 1;
            } else {
                break;
            }

            if i >= len {
                break;
            }

            // Pula comentários <!-- ... -->
            if bytes[i..].starts_with(b"!--") {
                if let Some(end) = input[i..].find("-->") {
                    i += end + 3;
                } else {
                    break;
                }
                continue;
            }

            // Pula declarações <!DOCTYPE ...> ou CDATA
            if bytes[i] == b'!' {
                while i < len && bytes[i] != b'>' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            // Pula tags de fechamento </... >
            if bytes[i] == b'/' {
                while i < len && bytes[i] != b'>' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            // Lê o nome da tag
            let start_tag = i;
            while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
                i += 1;
            }
            let tag_name = &input[start_tag..i];

            let is_target = tag_name.eq_ignore_ascii_case("link")
                || tag_name.eq_ignore_ascii_case("script")
                || tag_name.eq_ignore_ascii_case("img")
                || tag_name.eq_ignore_ascii_case("video")
                || tag_name.eq_ignore_ascii_case("source");

            // Lê os atributos respeitando aspas para evitar encerramento prematuro em '>'
            let attr_start = i;
            let mut quote = None;
            while i < len {
                let b = bytes[i];
                if let Some(q) = quote {
                    if b == q {
                        quote = None;
                    }
                } else if b == b'"' || b == b'\'' {
                    quote = Some(b);
                } else if b == b'>' {
                    break;
                }
                i += 1;
            }

            if is_target {
                let attr_str = &input[attr_start..i];
                if let Some(req) = Self::parse_preload_attributes(tag_name, attr_str) {
                    requests.push(req);
                }
            }

            if i < len && bytes[i] == b'>' {
                i += 1;
            }
        }

        requests
    }

    /// Faz o parsing dos atributos relevantes de uma tag para extração da URL de sub-recurso.
    fn parse_preload_attributes(tag: &str, attrs_raw: &str) -> Option<PreloadRequest> {
        let mut href: Option<&str> = None;
        let mut src: Option<&str> = None;
        let mut poster: Option<&str> = None;
        let mut rel: Option<&str> = None;
        let mut as_type: Option<&str> = None;
        let mut media: Option<&str> = None;

        Self::for_each_attribute(attrs_raw, |name, val| {
            if name.eq_ignore_ascii_case("href") {
                if href.is_none() { href = Some(val); }
            } else if name.eq_ignore_ascii_case("src") {
                if src.is_none() { src = Some(val); }
            } else if name.eq_ignore_ascii_case("poster") {
                if poster.is_none() { poster = Some(val); }
            } else if name.eq_ignore_ascii_case("rel") {
                if rel.is_none() { rel = Some(val); }
            } else if name.eq_ignore_ascii_case("as") {
                if as_type.is_none() { as_type = Some(val); }
            } else if name.eq_ignore_ascii_case("media") {
                if media.is_none() { media = Some(val); }
            }
        });

        if tag.eq_ignore_ascii_case("link") {
            let rel_str = rel?;
            let url_str = href?;
            if rel_str.split_ascii_whitespace().any(|r| r.eq_ignore_ascii_case("stylesheet")) {
                return Some(PreloadRequest {
                    url: SmolStr::new(url_str),
                    kind: PreloadKind::Stylesheet,
                    as_type: None,
                    media: media.map(SmolStr::new),
                });
            } else if rel_str.split_ascii_whitespace().any(|r| {
                r.eq_ignore_ascii_case("preload") || r.eq_ignore_ascii_case("modulepreload")
            }) {
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

    /// Itera zero-copy pelos pares (nome, valor) de atributos com suporte a aspas.
    fn for_each_attribute<'a>(attrs_raw: &'a str, mut f: impl FnMut(&'a str, &'a str)) {
        let bytes = attrs_raw.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Pula espaços
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= len || bytes[i] == b'>' || bytes[i] == b'/' {
                break;
            }

            // Lê nome
            let name_start = i;
            while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' && bytes[i] != b'/' && bytes[i] != b'>' {
                i += 1;
            }
            let name = attrs_raw[name_start..i].trim();
            if name.is_empty() {
                i += 1;
                continue;
            }

            // Pula espaços antes do '='
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            let mut val = "";
            if i < len && bytes[i] == b'=' {
                i += 1; // Pula '='
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < len {
                    let quote = bytes[i];
                    if quote == b'"' || quote == b'\'' {
                        i += 1;
                        let val_start = i;
                        while i < len && bytes[i] != quote {
                            i += 1;
                        }
                        val = &attrs_raw[val_start..i];
                        if i < len {
                            i += 1; // Pula quote de fechamento
                        }
                    } else {
                        let val_start = i;
                        while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                            i += 1;
                        }
                        val = &attrs_raw[val_start..i];
                    }
                }
            }

            f(name, val);
        }
    }
}
