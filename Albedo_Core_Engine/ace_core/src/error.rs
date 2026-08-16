use thiserror::Error;

/// Representa a unificação de todos os erros possíveis dentro do ecossistema Albedo.
///
/// O `AceError` foi projetado para atuar em conjunto com a crate `anyhow`. 
/// Ele categoriza os domínios de erro do navegador (Ex: Rede, Segurança, Parser)
/// sem expor detalhes excessivos de implementação de bibliotecas de terceiros na interface pública.
///
/// # Exemplos
///
/// ```
/// use ace_core::error::AceError;
///
/// fn process_network() -> Result<(), AceError> {
///     Err(AceError::NetworkError {
///         message: "DNS Resolution Failed".to_string(),
///     })
/// }
/// ```
#[derive(Error, Debug)]
pub enum AceError {
    /// Ocorre quando uma operação do Sistema Operacional falha (leitura de disco, sockets brutos).
    /// Envelopa o `std::io::Error` automaticamente via `#[from]`.
    #[error("Erro de I/O do sistema: {0}")]
    Io(#[from] std::io::Error),

    /// Emitido pelas camadas de Parsing (HTML, CSS, JavaScript) quando encontram sintaxe inválida
    /// ou falham em recuperar a leitura a partir de um estado inconsistente.
    #[error("Erro de parse: {message}")]
    ParseError { 
        /// Descrição detalhada do token, linha ou estrutura que causou a falha.
        message: String 
    },

    /// Emitido por falhas na camada `ace_net`, como timeouts, problemas de TLS, falha de Handshake ou erros HTTP.
    #[error("Erro de rede: {message}")]
    NetworkError { 
        /// Contexto da falha de rede (ex: "Timeout ao conectar no IP" ou "Conexão Resetada").
        message: String 
    },

    /// Indica uma violação grave de política do navegador (CORS, CSP, SOP) ou 
    /// rejeição de certificado SSL de um site na web.
    #[error("Erro de segurança: {message}")]
    SecurityError { 
        /// Motivo detalhado do bloqueio de segurança. Ex: "CSP bloqueou o carregamento do recurso inline".
        message: String 
    },

    /// Ocorre durante a execução do motor JavaScript (compilação JIT, Garbage Collection ou runtime).
    #[error("Erro no motor JavaScript: {message}")]
    JsError { 
        /// A mensagem de exceção disparada pela Virtual Machine.
        message: String 
    },
    
    /// Utilizado primariamente pelo sistema de armazenamento interno, cache HTTP
    /// ou DOM quando uma chave requisitada (ID) não existe.
    #[error("Recurso não encontrado: {0}")]
    NotFound(String),

    /// Falha genérica para estados inválidos da máquina de estados do DOM/Render Tree
    /// que não se encaixam nas categorias acima.
    #[error("Operação inválida: {0}")]
    InvalidOperation(String),
}
