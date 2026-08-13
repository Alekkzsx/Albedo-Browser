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

/// Implementação padrão (Fallback/Mock) inicial.
/// Na Fase 3 (Rede), esse multiplexador ganhará a infraestrutura nativa do `windows-sys` (IOCP)
/// ou o uso de Sockets reais não-bloqueantes. No momento, serve para testar a mecânica
/// do EventLoop.
pub struct MockMultiplexer {
    _reserved: bool,
}

impl Default for MockMultiplexer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMultiplexer {
    pub fn new() -> Self {
        Self { _reserved: true }
    }
}

impl IoMultiplexer for MockMultiplexer {
    fn poll(&mut self, timeout_ms: Option<u64>) -> crate::AceResult<()> {
        // Simula a ida ao Kernel para aguardar pacotes na placa de rede.
        // Se um timeout foi fornecido (ex: timer do EventLoop para renderização), dormimos.
        if let Some(ms) = timeout_ms {
            if ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
        }
        Ok(())
    }
}
