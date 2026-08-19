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
    /// Tarefas do pipeline de renderização (`requestAnimationFrame`, recalc de layout/estilo).
    Rendering,
    /// Navegação e histórico de sessões (History Traversal).
    HistoryTraversal,
    /// Respostas de requisições de rede (`fetch`, HTTP/HTTPS, WebSockets, DNS).
    Networking,
    /// Temporizadores agendados (`setTimeout`, `setInterval`).
    Timer,
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
            Self::HistoryTraversal => 3,
            Self::Networking => 4,
            Self::Timer => 5,
            Self::Internal => 6,
        }
    }

    /// Lista todas as fontes de tarefas em ordem de prioridade padrão WHATWG.
    pub const ALL_SOURCES: [TaskSource; 7] = [
        Self::UserInteraction,
        Self::DomManipulation,
        Self::Rendering,
        Self::HistoryTraversal,
        Self::Networking,
        Self::Timer,
        Self::Internal,
    ];
}
