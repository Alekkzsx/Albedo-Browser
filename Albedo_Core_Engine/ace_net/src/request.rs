//! # Modelagem de Requisições de Rede (`Request` & `RequestBuilder`)
//!
//! Representa a especificação completa de uma intenção de busca de recurso (Fetch),
//! suportando destinos semânticos, modos de segurança (CORS/SOP), políticas de redirecionamento,
//! chaves de isolamento e prioridades de escalonamento.

use crate::cache::partition::NetworkIsolationKey;
use crate::error::{NetError, NetResult};
use crate::priority::PriorityLevel;
use ace_core::security::referrer::ReferrerPolicy;
use bytes::Bytes;
pub use http::header::{HeaderMap, HeaderName, HeaderValue};
pub use http::Method;
use std::time::Duration;
use url::Url;

/// Destino semântico do recurso segundo a especificação WHATWG Fetch §2.2.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RequestDestination {
    /// Documento HTML principal ou de iframe.
    #[default]
    Document,
    /// Folha de estilo CSS (`<link rel="stylesheet">`, `@import`).
    Style,
    /// Script JavaScript executável (`<script>`).
    Script,
    /// Fonte tipográfica web (`@font-face`).
    Font,
    /// Elemento de imagem (`<img>`, CSS `background-image`).
    Image,
    /// Mídia contínua (`<audio>`, `<video>`).
    Media,
    /// Requisição de API programática (`fetch()`, `XMLHttpRequest`).
    Fetch,
    /// Outros sub-recursos (manifest, worker, track, etc.).
    Other,
}

impl RequestDestination {
    /// Retorna a prioridade default de navegador para este tipo de destino.
    pub const fn default_priority(self) -> PriorityLevel {
        match self {
            Self::Document => PriorityLevel::VeryHigh,
            Self::Style | Self::Font => PriorityLevel::High,
            Self::Script => PriorityLevel::High,
            Self::Image => PriorityLevel::Medium,
            Self::Media | Self::Fetch | Self::Other => PriorityLevel::Low,
        }
    }
}

/// Modo de isolamento de segurança da requisição segundo WHATWG Fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RequestMode {
    /// Permite apenas navegação para documentos HTML.
    #[default]
    Navigate,
    /// Exige validação de cabeçalhos de preflight e resposta CORS.
    Cors,
    /// Modo restrito sem leitura de corpo cross-origin (ex: imagens/scripts simples).
    NoCors,
    /// Rejeita requisições se o destino for de origem distinta.
    SameOrigin,
}

/// Modo de envio de credenciais (cookies, HTTP Auth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CredentialsMode {
    /// Nunca envia cookies ou credenciais.
    Omit,
    /// Envia credenciais apenas se a URL for da mesma origem.
    #[default]
    SameOrigin,
    /// Envia credenciais mesmo para URLs cross-origin (se CORS permitir).
    Include,
}

/// Política de tratamento de respostas HTTP de redirecionamento (3xx).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Segue automaticamente até N saltos (padrão 20).
    Follow(usize),
    /// Não segue automaticamente; retorna a resposta 3xx como está.
    Manual,
    /// Trata redirecionamento como erro de rede fatal.
    Error,
}

impl Default for RedirectPolicy {
    fn default() -> Self {
        Self::Follow(20)
    }
}

/// Requisição de rede encapsulada e imutável para despacho pelo `ResourceFetcher`.
#[derive(Debug, Clone)]
pub struct Request {
    pub url: Url,
    pub method: Method,
    pub headers: HeaderMap,
    pub body: Option<Bytes>,
    pub priority: PriorityLevel,
    pub destination: RequestDestination,
    pub mode: RequestMode,
    pub credentials: CredentialsMode,
    pub redirect_policy: RedirectPolicy,
    pub referrer_policy: ReferrerPolicy,
    pub referrer: Option<Url>,
    pub network_isolation_key: Option<NetworkIsolationKey>,
    pub timeout: Option<Duration>,
}

impl Request {
    /// Inicia a construção de uma nova requisição para a URL especificada.
    pub fn get(url: impl TryIntoUrl) -> NetResult<RequestBuilder> {
        let url = url.try_into_url()?;
        Ok(RequestBuilder::new(url, Method::GET))
    }

    /// Inicia a construção de uma nova requisição POST.
    pub fn post(url: impl TryIntoUrl) -> NetResult<RequestBuilder> {
        let url = url.try_into_url()?;
        Ok(RequestBuilder::new(url, Method::POST))
    }

    /// Cria um novo `RequestBuilder` com método genérico.
    pub fn builder(url: impl TryIntoUrl, method: Method) -> NetResult<RequestBuilder> {
        let url = url.try_into_url()?;
        Ok(RequestBuilder::new(url, method))
    }
}

/// Construtor fluente para montagem ergonômica de requisições.
#[derive(Debug)]
pub struct RequestBuilder {
    url: Url,
    method: Method,
    headers: HeaderMap,
    body: Option<Bytes>,
    priority: Option<PriorityLevel>,
    destination: RequestDestination,
    mode: RequestMode,
    credentials: CredentialsMode,
    redirect_policy: RedirectPolicy,
    referrer_policy: ReferrerPolicy,
    referrer: Option<Url>,
    network_isolation_key: Option<NetworkIsolationKey>,
    timeout: Option<Duration>,
}

impl RequestBuilder {
    pub fn new(url: Url, method: Method) -> Self {
        Self {
            url,
            method,
            headers: HeaderMap::new(),
            body: None,
            priority: None,
            destination: RequestDestination::Document,
            mode: RequestMode::Navigate,
            credentials: CredentialsMode::SameOrigin,
            redirect_policy: RedirectPolicy::default(),
            referrer_policy: ReferrerPolicy::default(),
            referrer: None,
            network_isolation_key: None,
            timeout: Some(Duration::from_secs(30)),
        }
    }

    /// Adiciona um cabeçalho HTTP à requisição.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Define o corpo da requisição em bytes.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Define explicitamente a prioridade de escalonamento.
    pub fn priority(mut self, priority: PriorityLevel) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Define o destino do recurso.
    pub fn destination(mut self, destination: RequestDestination) -> Self {
        self.destination = destination;
        self
    }

    /// Define a chave de isolamento de rede (NIK).
    pub fn network_isolation_key(mut self, nik: NetworkIsolationKey) -> Self {
        self.network_isolation_key = Some(nik);
        self
    }

    /// Define o timeout da requisição.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Define a URL de Referrer e a política.
    pub fn referrer(mut self, referrer: Url, policy: ReferrerPolicy) -> Self {
        self.referrer = Some(referrer);
        self.referrer_policy = policy;
        self
    }

    /// Finaliza e constrói a instância de `Request`.
    pub fn build(self) -> Request {
        let priority = self.priority.unwrap_or_else(|| self.destination.default_priority());

        Request {
            url: self.url,
            method: self.method,
            headers: self.headers,
            body: self.body,
            priority,
            destination: self.destination,
            mode: self.mode,
            credentials: self.credentials,
            redirect_policy: self.redirect_policy,
            referrer_policy: self.referrer_policy,
            referrer: self.referrer,
            network_isolation_key: self.network_isolation_key,
            timeout: self.timeout,
        }
    }
}

/// Trait auxiliar para conversão flexível em `url::Url`.
pub trait TryIntoUrl {
    fn try_into_url(self) -> NetResult<Url>;
}

impl TryIntoUrl for Url {
    fn try_into_url(self) -> NetResult<Url> {
        Ok(self)
    }
}

impl TryIntoUrl for &str {
    fn try_into_url(self) -> NetResult<Url> {
        Url::parse(self).map_err(|e| NetError::InvalidUrl(format!("'{}': {}", self, e)))
    }
}

impl TryIntoUrl for &String {
    fn try_into_url(self) -> NetResult<Url> {
        self.as_str().try_into_url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder_defaults_and_priority() {
        let req = Request::get("https://example.com/index.html")
            .unwrap()
            .destination(RequestDestination::Document)
            .build();

        assert_eq!(req.url.as_str(), "https://example.com/index.html");
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.priority, PriorityLevel::VeryHigh);

        let style_req = Request::get("https://example.com/style.css")
            .unwrap()
            .destination(RequestDestination::Style)
            .build();
        assert_eq!(style_req.priority, PriorityLevel::High);
    }
}
