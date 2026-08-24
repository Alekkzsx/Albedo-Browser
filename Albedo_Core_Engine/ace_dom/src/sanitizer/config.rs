//! # Configuração do Sanitizador HTML (WHATWG §8.6 HTML Sanitization)

use ace_core::intern::Atom;

/// Configuração de regras e listas de permissão/bloqueio para o Sanitizador.
#[derive(Debug, Clone)]
pub struct SanitizerConfig {
    /// Lista de tags explicitamente permitidas (se `None`, aceita todas exceto as bloqueadas).
    pub allow_elements: Option<Vec<Atom>>,
    /// Lista de tags perigosas terminantemente bloqueadas.
    pub block_elements: Vec<Atom>,
    /// Lista de atributos permitidos (se `None`, aceita todos exceto os bloqueados).
    pub allow_attributes: Option<Vec<Atom>>,
    /// Lista de atributos bloqueados (ex: manipuladores `on*`).
    pub block_attributes: Vec<Atom>,
    /// Permitir comentários HTML no resultado.
    pub allow_comments: bool,
}

impl Default for SanitizerConfig {
    /// Cria a configuração padrão com defesas rigorosas contra ataques XSS.
    fn default() -> Self {
        let block_elements = vec![
            Atom::new("script"),
            Atom::new("iframe"),
            Atom::new("object"),
            Atom::new("embed"),
            Atom::new("applet"),
            Atom::new("frame"),
            Atom::new("frameset"),
            Atom::new("noembed"),
            Atom::new("noframes"),
            Atom::new("noscript"),
        ];

        let block_attributes = vec![
            Atom::new("formaction"),
            Atom::new("action"),
        ];

        Self {
            allow_elements: None,
            block_elements,
            allow_attributes: None,
            block_attributes,
            allow_comments: false,
        }
    }
}
