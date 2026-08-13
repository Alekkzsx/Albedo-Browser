// ============================================================================
// Albedo Core Engine (ACE)
// File: error.rs
// Description: Sistema unificado de tratamento de erros, provendo uma
//              hierarquia base customizada sem uso de dependências externas.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::borrow::Cow;
use std::fmt;

// ----------------------------------------------------------------------------
// Error Types
// ----------------------------------------------------------------------------

/// `AceError` é a enumeração raiz para todas as condições de falha dentro do motor Albedo.
///
/// Como política arquitetural restrita, não utilizamos crates de terceiros como `thiserror` ou `anyhow`.
/// Este enumerador agrupa todos os subsistemas críticos (Rede, Layout, DOM, etc.)
/// em categorias exclusivas para simplificar a rastreabilidade e a propagação.
#[derive(Debug)]
pub enum AceError {
    /// Ocorre em falhas de Sistema Operacional (ex: leitura de arquivos, sockets).
    /// Envolve um erro de origem (`std::io::Error`) para cadeia de chamadas (stacktrace).
    Io {
        source: std::io::Error,
        context: Cow<'static, str>,
    },

    /// Ocorre em caso de anomalias no parser HTML/CSS.
    Parse { message: Cow<'static, str> },

    /// Erros relacionados a falhas de conexão, timeout ou protocolos.
    Network { message: Cow<'static, str> },

    /// Violações de política (ex: CORS, CSP, SOP).
    Security { message: Cow<'static, str> },

    /// Falhas na resolução do Box Model ou algoritmos de geometria espacial.
    Layout { message: Cow<'static, str> },

    /// Problemas na VM, Compilação JIT ou interopabilidade DOM/JS.
    Js { message: Cow<'static, str> },

    /// Erros durante validação TLS ou algoritmos de criptografia.
    Crypto { message: Cow<'static, str> },

    /// Falhas de persistência local (LocalStorage, IndexedDB interno).
    Storage { message: Cow<'static, str> },

    /// Problemas no pipeline gráfico e rasterização na CPU.
    Render { message: Cow<'static, str> },

    /// Inconsistências na manipulação da árvore de documentos.
    Dom { message: Cow<'static, str> },

    /// Problemas com computação em cascata e herança de estilos.
    Style { message: Cow<'static, str> },

    /// Falhas de decodificação de imagens, vídeo e áudio.
    Media { message: Cow<'static, str> },

    /// Interrupções na comunicação inter-processo nativa do browser.
    Ipc { message: Cow<'static, str> },

    /// Deadlocks, envenenamento de Mutex, ou panes de Thread Pool.
    Thread { message: Cow<'static, str> },

    /// Erro não classificado (uso estrito como último recurso).
    Unknown { message: Cow<'static, str> },
}

impl AceError {
    /// Permite encadear uma mensagem de contexto adicional a um erro existente.
    pub fn with_context<C: Into<Cow<'static, str>>>(self, ctx: C) -> Self {
        match self {
            AceError::Io { source, context } => AceError::Io {
                source,
                context: format!("{}: {}", ctx.into(), context).into(),
            },
            // Para as demais variantes, envolvemos o erro num Unknown ou
            // formatamos a string. Por simplicidade de alocação:
            _ => {
                let current_msg = format!("{}", self);
                AceError::Unknown {
                    message: format!("{}: {}", ctx.into(), current_msg).into(),
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait Implementations
// ----------------------------------------------------------------------------

impl From<std::io::Error> for AceError {
    fn from(error: std::io::Error) -> Self {
        AceError::Io {
            context: Cow::Borrowed("System I/O failure"),
            source: error,
        }
    }
}

impl fmt::Display for AceError {
    /// Formata a mensagem de erro provendo contexto claro ao usuário ou sistema de log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AceError::Io { context, .. } => write!(f, "IO Error: {}", context),
            AceError::Parse { message } => write!(f, "Parse Error: {}", message),
            AceError::Network { message } => write!(f, "Network Error: {}", message),
            AceError::Security { message } => write!(f, "Security Error: {}", message),
            AceError::Layout { message } => write!(f, "Layout Error: {}", message),
            AceError::Js { message } => write!(f, "JS Engine Error: {}", message),
            AceError::Crypto { message } => write!(f, "Crypto Error: {}", message),
            AceError::Storage { message } => write!(f, "Storage Error: {}", message),
            AceError::Render { message } => write!(f, "Render Error: {}", message),
            AceError::Dom { message } => write!(f, "DOM Error: {}", message),
            AceError::Style { message } => write!(f, "Style Error: {}", message),
            AceError::Media { message } => write!(f, "Media Error: {}", message),
            AceError::Ipc { message } => write!(f, "IPC Error: {}", message),
            AceError::Thread { message } => write!(f, "Threading Error: {}", message),
            AceError::Unknown { message } => write!(f, "Unknown Error: {}", message),
        }
    }
}

impl std::error::Error for AceError {
    /// Delega à causa raiz quando disponível (fundamental para RUST_BACKTRACE).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AceError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

// ----------------------------------------------------------------------------
// Type Aliases
// ----------------------------------------------------------------------------

/// Um alias simplificado para encapsular funções que podem falhar dentro da Engine Albedo.
pub type AceResult<T> = Result<T, AceError>;
