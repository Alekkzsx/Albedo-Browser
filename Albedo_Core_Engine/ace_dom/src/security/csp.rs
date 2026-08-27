//! # Content Security Policy (W3C CSP Level 3 Standard)
//!
//! Verificação e bloqueio nativo de injeções de script, estilos e recursos não autorizados
//! diretamente no pipeline de construção da árvore DOM.

use rustc_hash::FxHashMap;
use sha2::{Digest, Sha256};

/// Diretivas normativas do Content Security Policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CSPDirective {
    DefaultSrc,
    ScriptSrc,
    StyleSrc,
    ImgSrc,
    ConnectSrc,
    FontSrc,
    ObjectSrc,
    FrameSrc,
}

impl CSPDirective {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "default-src" => Some(Self::DefaultSrc),
            "script-src" => Some(Self::ScriptSrc),
            "style-src" => Some(Self::StyleSrc),
            "img-src" => Some(Self::ImgSrc),
            "connect-src" => Some(Self::ConnectSrc),
            "font-src" => Some(Self::FontSrc),
            "object-src" => Some(Self::ObjectSrc),
            "frame-src" => Some(Self::FrameSrc),
            _ => None,
        }
    }
}

/// Fontes permitidas por uma diretiva CSP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CSPSource {
    None,
    SelfSource,
    UnsafeInline,
    UnsafeEval,
    StrictDynamic,
    Nonce(String),
    Sha256(String),
    Host(String),
    Scheme(String),
}

/// Uma política consolidada de Content Security Policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CSPPolicy {
    directives: FxHashMap<CSPDirective, Vec<CSPSource>>,
    origin: Option<String>,
}

impl CSPPolicy {
    /// Cria uma nova política CSP vazia (permite tudo).
    pub fn new() -> Self {
        Self {
            directives: FxHashMap::default(),
            origin: None,
        }
    }

    /// Define a origem do documento (ex: `https://example.com`).
    pub fn with_origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }

    /// Faz o parsing de um cabeçalho HTTP `Content-Security-Policy`.
    pub fn parse(header_value: &str) -> Self {
        let mut policy = Self::new();
        for policy_directive in header_value.split(';') {
            let trimmed = policy_directive.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut tokens = trimmed.split_whitespace();
            let directive_name = match tokens.next() {
                Some(name) => name,
                None => continue,
            };

            let directive = match CSPDirective::from_name(directive_name) {
                Some(d) => d,
                None => continue,
            };

            let mut sources = Vec::new();
            for token in tokens {
                let token_clean = token.trim_matches('\'').to_ascii_lowercase();
                let src = match token_clean.as_str() {
                    "none" => CSPSource::None,
                    "self" => CSPSource::SelfSource,
                    "unsafe-inline" => CSPSource::UnsafeInline,
                    "unsafe-eval" => CSPSource::UnsafeEval,
                    "strict-dynamic" => CSPSource::StrictDynamic,
                    _ if token.starts_with("'nonce-") && token.ends_with('\'') => {
                        let nonce = token.trim_start_matches("'nonce-").trim_end_matches('\'');
                        CSPSource::Nonce(nonce.to_string())
                    }
                    _ if token.starts_with("'sha256-") && token.ends_with('\'') => {
                        let hash = token.trim_start_matches("'sha256-").trim_end_matches('\'');
                        CSPSource::Sha256(hash.to_string())
                    }
                    _ if token.ends_with(':') => {
                        CSPSource::Scheme(token.trim_end_matches(':').to_string())
                    }
                    _ => CSPSource::Host(token.to_string()),
                };
                sources.push(src);
            }

            policy.directives.insert(directive, sources);
        }
        policy
    }

    /// Retorna as fontes ativas para uma diretiva, aplicando fallback para `default-src`.
    fn get_sources_for_directive(&self, directive: CSPDirective) -> Option<&[CSPSource]> {
        if let Some(sources) = self.directives.get(&directive) {
            Some(sources.as_slice())
        } else {
            self.directives.get(&CSPDirective::DefaultSrc).map(|s| s.as_slice())
        }
    }

    /// Calcula o hash SHA-256 codificado em Base64 de um bloco de código textual.
    fn compute_sha256_base64(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        use std::fmt::Write;
        let mut hex = String::with_capacity(result.len() * 2);
        for b in result {
            let _ = write!(&mut hex, "{:02x}", b);
        }
        hex
    }

    /// Verifica se a execução de um script em linha (`<script>...</script>`) é permitida.
    pub fn allows_inline_script(&self, content: Option<&str>, nonce: Option<&str>) -> bool {
        let sources = match self.get_sources_for_directive(CSPDirective::ScriptSrc) {
            Some(s) => s,
            None => return true, // Sem diretiva script-src / default-src => Permitido
        };

        if sources.iter().any(|s| matches!(s, CSPSource::None)) {
            return false;
        }

        // Verifica Nonce
        if let Some(n) = nonce {
            if sources.iter().any(|s| match s {
                CSPSource::Nonce(expected_nonce) => expected_nonce == n,
                _ => false,
            }) {
                return true;
            }
        }

        // Verifica Hash SHA-256 se fornecido
        if let Some(code) = content {
            let hash = Self::compute_sha256_base64(code);
            if sources.iter().any(|s| match s {
                CSPSource::Sha256(expected_hash) => expected_hash.eq_ignore_ascii_case(&hash),
                _ => false,
            }) {
                return true;
            }
        }

        // Verifica 'unsafe-inline'
        sources.iter().any(|s| matches!(s, CSPSource::UnsafeInline))
    }

    /// Verifica se a inclusão de um script externo (`<script src="...">`) é permitida.
    pub fn allows_script_src(&self, url: &str) -> bool {
        self.allows_resource_src(CSPDirective::ScriptSrc, url)
    }

    /// Verifica se o carregamento de uma folha de estilo (`<link href="..." rel="stylesheet">`) é permitido.
    pub fn allows_style_src(&self, url: &str) -> bool {
        self.allows_resource_src(CSPDirective::StyleSrc, url)
    }

    /// Verifica se o carregamento de uma imagem (`<img src="...">`) é permitido.
    pub fn allows_img_src(&self, url: &str) -> bool {
        self.allows_resource_src(CSPDirective::ImgSrc, url)
    }

    /// Verifica se uma requisição de rede/fetch/XHR é permitida.
    pub fn allows_connect_src(&self, url: &str) -> bool {
        self.allows_resource_src(CSPDirective::ConnectSrc, url)
    }

    fn allows_resource_src(&self, directive: CSPDirective, url: &str) -> bool {
        let sources = match self.get_sources_for_directive(directive) {
            Some(s) => s,
            None => return true,
        };

        if sources.iter().any(|s| matches!(s, CSPSource::None)) {
            return false;
        }

        for source in sources {
            match source {
                CSPSource::SelfSource => {
                    if let Some(ref origin) = self.origin {
                        if url.starts_with(origin) || url.starts_with('/') {
                            return true;
                        }
                    } else if url.starts_with('/') {
                        return true;
                    }
                }
                CSPSource::Host(host) if url.contains(host) => return true,
                CSPSource::Scheme(scheme) if url.starts_with(scheme) => return true,
                _ => {}
            }
        }

        false
    }
}
