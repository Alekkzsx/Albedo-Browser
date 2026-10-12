//! # As 8 Origens Normativas da Cascata CSS (CSS Cascading and Inheritance Level 5 §4)
//!
//! Ordem estrita de precedência entre User-Agent, Usuário, Autor, Animações e Transições.

/// Origem de uma folha de estilo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum StyleSheetOrigin {
    /// Folha padrão do navegador (User-Agent Stylesheet)
    UserAgent,
    /// Folha de estilos definida pelo usuário (Acessibilidade, preferências)
    User,
    /// Folha de estilos definida pelo autor da página (HTML `<style>`, `<link rel="stylesheet">`)
    #[default]
    Author,
}

/// As 8 origens normativas da cascata CSS em ordem crescente de precedência.
/// Conforme a especificação W3C: valores maiores vencem valores menores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CascadeOrigin {
    /// 1. User-Agent normal
    UserAgentNormal = 1,
    /// 2. User normal
    UserNormal = 2,
    /// 3. Author normal
    AuthorNormal = 3,
    /// 4. Animações CSS (`@keyframes`)
    Animation = 4,
    /// 5. Author `!important`
    AuthorImportant = 5,
    /// 6. User `!important`
    UserImportant = 6,
    /// 7. User-Agent `!important`
    UserAgentImportant = 7,
    /// 8. Transições CSS ativas
    Transition = 8,
}

impl CascadeOrigin {
    /// Determina a origem da cascata para uma declaração com base na origem da folha e na flag `!important`.
    pub fn from_sheet_origin(sheet_origin: StyleSheetOrigin, important: bool) -> Self {
        if important {
            match sheet_origin {
                StyleSheetOrigin::Author => Self::AuthorImportant,
                StyleSheetOrigin::User => Self::UserImportant,
                StyleSheetOrigin::UserAgent => Self::UserAgentImportant,
            }
        } else {
            match sheet_origin {
                StyleSheetOrigin::Author => Self::AuthorNormal,
                StyleSheetOrigin::User => Self::UserNormal,
                StyleSheetOrigin::UserAgent => Self::UserAgentNormal,
            }
        }
    }

    /// Retorna `true` se esta origem for `!important`.
    pub fn is_important(&self) -> bool {
        matches!(
            self,
            Self::AuthorImportant | Self::UserImportant | Self::UserAgentImportant
        )
    }
}
