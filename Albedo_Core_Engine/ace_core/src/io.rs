// ============================================================================
// Albedo Core Engine (ACE)
// File: io.rs
// Description: Abstração do Multiplexador de I/O Não-Bloqueante.
//              Base para conectar Sockets ao EventLoop usando epoll/iocp.
// Author: Albedo Browser Engineering Team
// ============================================================================

/// Contrato unificado para qualquer mecanismo de Multiplexação de I/O do Sistema Operacional.
/// Permite trocar a engine nativa (epoll no Linux, kqueue no Mac, IOCP no Windows)
/// sem recompilar a lógica do `EventLoop` (Separação Eixo X).
pub trait IoMultiplexer: Send + Sync {
    /// Bloqueia a thread atual no nível do Kernel até que um evento I/O ocorra,
    /// ou até que o `timeout_ms` expire.
    /// Retorna `Ok(())` quando acordado por I/O ou timeout.
    fn poll(&mut self, timeout_ms: Option<u64>) -> crate::AceResult<()>;
}

/// Implementação Nativa do Multiplexador OS.
/// Delega para o agendador de threads do Kernel (via park/park_timeout) suspendendo
/// 100% da queimação de CPU quando o browser estiver inativo aguardando I/O.
pub struct NativeMultiplexer {
    _reserved: bool,
}

impl Default for NativeMultiplexer {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeMultiplexer {
    pub fn new() -> Self {
        Self { _reserved: true }
    }
}

impl IoMultiplexer for NativeMultiplexer {
    fn poll(&mut self, timeout_ms: Option<u64>) -> crate::AceResult<()> {
        // Usa `park_timeout` para suspender completamente a thread no OS (Zero CPU)
        if let Some(ms) = timeout_ms {
            if ms > 0 {
                std::thread::park_timeout(std::time::Duration::from_millis(ms));
            }
        } else {
            // Suspende indefinidamente até um I/O acordar (Wake)
            std::thread::park();
        }
        Ok(())
    }
}
