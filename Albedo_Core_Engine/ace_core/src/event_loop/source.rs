//! # Fontes de Tarefas (WHATWG Task Sources)
//!
//! A especificação HTML5/WHATWG (Seção 8.1.6) define que tarefas assíncronas pertencem
//! a fontes distintas para permitir que o navegador priorize interações do usuário sobre timers ou rede.

/// Identifica a fonte de origem de uma tarefa no Event Loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskSource {
    /// Interação do usuário (teclado, clique, mouse, foco, touch). Prioridade MÁXIMA para garantir 60fps.
    UserInteraction,
    /// Mutações e eventos do DOM.
    DomManipulation,
    /// Respostas de requisições de rede (`fetch`, HTTP/HTTPS, WebSockets, DNS).
    Networking,
    /// Temporizadores agendados (`setTimeout`, `setInterval`).
    Timer,
    /// Tarefas do pipeline de renderização (`requestAnimationFrame`, recalc de layout/estilo).
    Rendering,
    /// Tarefas internas do motor Albedo (IPC, GC cycle collector, profiling).
    Internal,
}

impl TaskSource {
    /// Retorna o nível de prioridade numérica (menor número = maior prioridade de despacho).
    #[inline]
    pub const fn priority(self) -> u8 {
        match self {
            Self::UserInteraction => 0,
            Self::DomManipulation => 1,
            Self::Rendering => 2,
            Self::Networking => 3,
            Self::Timer => 4,
            Self::Internal => 5,
        }
    }
}
