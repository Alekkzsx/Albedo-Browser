//! # Trusted Types (W3C Trusted Types Specification)
//!
//! Tipagem de segurança estrita para blindagem de sinks XSS sensíveis (`innerHTML`, `outerHTML`, `eval`).

use smol_str::SmolStr;

/// Representa uma string HTML validada por uma política de segurança confiável.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedHTML(SmolStr);

impl TrustedHTML {
    /// Cria uma nova instância de `TrustedHTML` diretamente (deve ser chamado apenas por políticas confiáveis).
    pub fn from_trusted_source(html: impl Into<SmolStr>) -> Self {
        Self(html.into())
    }

    /// Retorna a representação textual como `&str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Representa um script validado e seguro para execução no motor JavaScript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedScript(SmolStr);

impl TrustedScript {
    pub fn from_trusted_source(script: impl Into<SmolStr>) -> Self {
        Self(script.into())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Representa uma URL segura para carregamento dinâmico de scripts ou web workers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedScriptURL(SmolStr);

impl TrustedScriptURL {
    pub fn from_trusted_source(url: impl Into<SmolStr>) -> Self {
        Self(url.into())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Função de transformação e higienização para Trusted Types.
pub type TrustedTransformFn = Box<dyn Fn(&str) -> SmolStr + Send + Sync>;

/// Política de geração e transformação de Trusted Types.
pub struct TrustedTypePolicy {
    pub name: SmolStr,
    create_html_fn: Option<TrustedTransformFn>,
    create_script_fn: Option<TrustedTransformFn>,
}

impl TrustedTypePolicy {
    pub fn new(name: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            create_html_fn: None,
            create_script_fn: None,
        }
    }

    pub fn with_create_html(
        mut self,
        f: impl Fn(&str) -> SmolStr + Send + Sync + 'static,
    ) -> Self {
        self.create_html_fn = Some(Box::new(f));
        self
    }

    pub fn with_create_script(
        mut self,
        f: impl Fn(&str) -> SmolStr + Send + Sync + 'static,
    ) -> Self {
        self.create_script_fn = Some(Box::new(f));
        self
    }

    pub fn create_html(&self, input: &str) -> TrustedHTML {
        if let Some(ref f) = self.create_html_fn {
            TrustedHTML(f(input))
        } else {
            TrustedHTML(SmolStr::new(input))
        }
    }

    pub fn create_script(&self, input: &str) -> TrustedScript {
        if let Some(ref f) = self.create_script_fn {
            TrustedScript(f(input))
        } else {
            TrustedScript(SmolStr::new(input))
        }
    }
}
