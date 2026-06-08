//! # Albedo Core Engine (ACE)
//!
//! Este módulo representa o coração do navegador Albedo. O **ACE** é responsável por todo o processo
//! de interpretação e renderização de conteúdo web, desde o parseamento do HTML até a execução de JavaScript.
//!
//! ## Arquitetura do Módulo
//!
//! - **html**: Implementação completa do parser de HTML5, incluindo tokenizer, tree builder e serializadores.
//!   Suporta parsing incremental (streaming) e otimizações para documentos grandes.
//!
//! - **engine**: O motor principal que orquestra DOM, CSS, Layout e Renderização.
//!   Contém a lógica de construção da árvore de estilos, cálculo de layout (reflow) e geração da display list.
//!
//! - **runtime**: Ambiente de execução JavaScript baseado em QuickJS.
//!   Implementa os bindings para as Web APIs (Window, Document, Element, Fetch, etc.) permitindo que o JS
//!   interaja com o DOM e o sistema de rede.
//!
//! - **url**: Utilitários para parseamento e manipulação de URLs, seguindo a especificação WHATWG URL Standard.
//!   Inclui suporte a percent-encoding, punycode e query parameters.
//!
//! - **util/utils**: Funções auxiliares de baixo nível, como allocators personalizados e helpers de string.
//!
//! - **crypto**: Wrappers seguros para operações criptográficas (SHA256, geração de random bytes).
//!
//! - **json**: Parser e serializer JSON otimizado para uso interno.
//!
//! - **contracts**: Definição de interfaces e traits comuns usados entre os sub-módulos para garantir consistência.
//!
//! ## Fluxo de Dados
//!
//! 1. O módulo `html` recebe bytes brutos e produz uma árvore DOM.
//! 2. O módulo `engine` aplica estilos CSS a essa DOM e calcula o layout geométrico.
//! 3. O módulo `runtime` injeta scripts na página e permite manipulação dinâmica da DOM.
//! 4. Alterações na DOM disparam re-cálculos no `engine`, que atualiza a cena para o renderer gráfico.

pub mod html;
pub mod engine;
pub mod runtime;
pub mod util;
pub mod crypto;
pub mod json;
pub mod url;
pub mod contracts;
pub mod utils;

// Re-exports seletivos para evitar ambiguidade e facilitar o uso externo
pub use html::{HtmlTokenizer, StreamingHtmlParser};
pub use engine::AceEngine;
