//! # Sistema Unificado de Tratamento de Erros e Rastreabilidade
//!
//! O `AceError` unifica todos os erros que podem ocorrer no ciclo de vida do navegador,
//! fornecendo contexto rico (localização no código-fonte, URLs, políticas de segurança violadas).

use std::fmt;
use thiserror::Error;

/// Representa a localização exata de um token ou erro em um arquivo de código-fonte (HTML/CSS/JS).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SourceLocation {
    /// URL ou identificador do documento de origem.
    pub url: Option<String>,
    /// Número da linha (1-indexed).
    pub line: usize,
    /// Número da coluna (1-indexed).
    pub column: usize,
    /// Offset em bytes a partir do início do stream de entrada.
    pub byte_offset: usize,
}

impl SourceLocation {
    /// Cria uma nova localização no código-fonte.
    #[inline]
    pub const fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            url: None,
            line,
            column,
            byte_offset,
        }
    }

    /// Cria uma localização com a URL do arquivo de origem.
    pub fn with_url(url: impl Into<String>, line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            url: Some(url.into()),
            line,
            column,
            byte_offset,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref url) = self.url {
            write!(f, "{}:{}:{} (byte {})", url, self.line, self.column, self.byte_offset)
        } else {
            write!(f, "{}:{} (byte {})", self.line, self.column, self.byte_offset)
        }
    }
}

/// Representa a unificação de todos os erros possíveis dentro do ecossistema Albedo.
#[derive(Error, Debug)]
pub enum AceError {
    /// Ocorre quando uma operação do Sistema Operacional falha (leitura de disco, sockets brutos).
    #[error("Erro de I/O do sistema: {0}")]
    Io(#[from] std::io::Error),

    /// Emitido pelas camadas de Parsing (HTML5, CSS3, JavaScript) quando encontram sintaxe inválida.
    #[error("Erro de parse [{location}]: {message}")]
    Parse {
        /// Posição no código de origem onde a falha foi detectada.
        location: SourceLocation,
        /// Mensagem descritiva do erro.
        message: String,
        /// Indica se a especificação WHATWG/W3C permite recuperação automática de erro.
        is_recoverable: bool,
    },

    /// Emitido por falhas na camada `ace_net`, como timeouts, problemas de TLS, falha de Handshake ou erros HTTP.
    #[error("Erro de rede ao acessar '{url}': {message}")]
    Network {
        /// URL da requisição que falhou.
        url: String,
        /// Código de status HTTP (se uma resposta foi recebida).
        status_code: Option<u16>,
        /// Descrição detalhada da falha.
        message: String,
    },

    /// Indica uma violação grave de política do navegador (CORS, CSP, SOP) ou rejeição de certificado TLS.
    #[error("Erro de segurança [{policy}]: {message}")]
    Security {
        /// Política de segurança violada (ex: "CSP", "CORS", "SOP", "TLS").
        policy: &'static str,
        /// Motivo detalhado do bloqueio.
        message: String,
    },

    /// Emitido por operações inválidas na Árvore DOM (ex: tentar inserir um nó como filho de si mesmo).
    #[error("Erro de DOM: {message}")]
    Dom {
        /// Contexto da operação inválida no DOM.
        message: String,
    },

    /// Emitido pelo motor de estilos (CSSOM, resolução de cascata ou media queries).
    #[error("Erro de estilo: {message}")]
    Style {
        /// Contexto do erro de estilo.
        message: String,
    },

    /// Emitido pelo motor de geometria/layout (BFC, IFC, Flexbox, Grid).
    #[error("Erro de layout: {message}")]
    Layout {
        /// Contexto do erro de layout.
        message: String,
    },

    /// Emitido pelo pipeline de pintura ou compositor de GPU (`wgpu`).
    #[error("Erro de renderização: {message}")]
    Render {
        /// Contexto do erro de renderização.
        message: String,
    },

    /// Ocorre durante a execução do motor JavaScript (compilação bytecode, GC ou runtime VM).
    #[error("Erro no motor JavaScript: {message}")]
    Js {
        /// A mensagem de exceção disparada pela VM.
        message: String,
        /// Stack trace associado, se disponível.
        stack: Option<String>,
    },

    /// Emitido pelo sistema de persistência, cache ou LocalStorage.
    #[error("Erro de armazenamento: {message}")]
    Storage {
        /// Contexto da falha de persistência.
        message: String,
    },

    /// Emitido pelo subsistema de comunicação inter-processos (IPC).
    #[error("Erro de IPC: {message}")]
    Ipc {
        /// Contexto da falha de canal ou serialização.
        message: String,
    },

    /// Emitido quando uma sequência de bytes não pôde ser decodificada no encoding esperado.
    #[error("Erro de encoding de caracteres: {message}")]
    Encoding {
        /// Detalhes do encoding inválido.
        message: String,
    },

    /// Operação assíncrona excedeu o tempo limite configurado.
    #[error("Operação excedeu o tempo limite de {duration_ms}ms")]
    Timeout {
        /// Duração em milissegundos que disparou o timeout.
        duration_ms: u64,
    },

    /// Operação foi cancelada via `AbortController` ou fechamento de aba.
    #[error("Operação abortada")]
    Aborted,

    /// Utilizado primariamente pelo sistema de armazenamento interno, cache HTTP
    /// ou DOM quando um recurso requisitado não existe.
    #[error("Recurso não encontrado: {0}")]
    NotFound(String),

    /// Falha genérica para estados inválidos da máquina de estados do DOM/Render Tree.
    #[error("Operação inválida: {0}")]
    InvalidOperation(String),
}

impl AceError {
    /// Construtor ergonômico para erros de parsing.
    pub fn parse(message: impl Into<String>, location: SourceLocation, is_recoverable: bool) -> Self {
        Self::Parse {
            location,
            message: message.into(),
            is_recoverable,
        }
    }

    /// Construtor ergonômico para erros de rede.
    pub fn network(url: impl Into<String>, message: impl Into<String>, status_code: Option<u16>) -> Self {
        Self::Network {
            url: url.into(),
            status_code,
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de segurança.
    pub fn security(policy: &'static str, message: impl Into<String>) -> Self {
        Self::Security {
            policy,
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de DOM.
    pub fn dom(message: impl Into<String>) -> Self {
        Self::Dom {
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de estilo.
    pub fn style(message: impl Into<String>) -> Self {
        Self::Style {
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de layout.
    pub fn layout(message: impl Into<String>) -> Self {
        Self::Layout {
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de renderização.
    pub fn render(message: impl Into<String>) -> Self {
        Self::Render {
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de JavaScript.
    pub fn js(message: impl Into<String>, stack: Option<String>) -> Self {
        Self::Js {
            message: message.into(),
            stack,
        }
    }

    /// Construtor ergonômico para erros de persistência.
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de IPC.
    pub fn ipc(message: impl Into<String>) -> Self {
        Self::Ipc {
            message: message.into(),
        }
    }

    /// Construtor ergonômico para erros de operação inválida.
    pub fn invalid_op(message: impl Into<String>) -> Self {
        Self::InvalidOperation(message.into())
    }

    /// Construtor ergonômico para recurso não encontrado.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    /// Retorna `true` se o erro for recuperável segundo as especificações do WHATWG/W3C.
    #[inline]
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Parse { is_recoverable, .. } => *is_recoverable,
            _ => false,
        }
    }

    /// Retorna a localização no código-fonte onde a falha ocorreu, se aplicável.
    #[inline]
    pub fn source_location(&self) -> Option<&SourceLocation> {
        match self {
            Self::Parse { location, .. } => Some(location),
            _ => None,
        }
    }

    /// Retorna o nome do subsistema de origem deste erro.
    #[inline]
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Parse { .. } => "parse",
            Self::Network { .. } => "network",
            Self::Security { .. } => "security",
            Self::Dom { .. } => "dom",
            Self::Style { .. } => "style",
            Self::Layout { .. } => "layout",
            Self::Render { .. } => "render",
            Self::Js { .. } => "js",
            Self::Storage { .. } => "storage",
            Self::Ipc { .. } => "ipc",
            Self::Encoding { .. } => "encoding",
            Self::Timeout { .. } => "timeout",
            Self::Aborted => "aborted",
            Self::NotFound(_) => "not_found",
            Self::InvalidOperation(_) => "invalid_operation",
        }
    }
}
