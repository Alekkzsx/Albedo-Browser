//! # Modos de Inserção da Construção da Árvore (WHATWG §12.2.6)
//!
//! Os 23 modos de inserção normativos para o algoritmo de construção da árvore HTML5.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertionMode {
    #[default]
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}
