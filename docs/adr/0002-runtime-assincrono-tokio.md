# ADR-0002: Adoção do Runtime Assíncrono Tokio

- **Status:** Aceito & Implementado
- **Data:** 2026-08-16 (Implementado na Fase 3)
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_core`, `ace_net`

---

## 1. Contexto e Problema

Um navegador web moderno lida com centenas de conexões de rede simultâneas, timers assíncronos e I/O de disco sem poder bloquear a interface do usuário ou o ciclo de renderização. Precisávamos de um runtime assíncrono robusto e testado em batalha.

## 2. Decisão

Adotamos a crate **`tokio`** (com suas utilidades `tokio-util`) como a Fundação oficial para o runtime assíncrono e I/O não-bloqueante no Albedo.

## 3. Consequências

- **Positivas:** Ecossistema maduro, integração nativa com `hyper` e `quinn`, pool de threads de alto desempenho com *work-stealing*.
- **Negativas / Riscos:** Dependência de uma crate externa substancial. Mitigado isolando o Tokio estritamente na camada de transporte de `ace_net`, enquanto o Event Loop da página web (`ace_core::event_loop`) possui agendador próprio conforme o padrão WHATWG.

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado em produção com 100% de aprovação nos testes do workspace.

- **`ace_net` (`Albedo_Core_Engine/ace_net/Cargo.toml`):**
  - Dependência: `tokio = { version = "1.40", features = ["full"] }`.
  - Camada de transporte: `src/transport/` utiliza `tokio::net::TcpStream`, canais MPSC e timers para gerenciar pools de conexões e multiplexação assíncrona HTTP/1.1 e HTTP/2.
  - Orquestração assíncrona: `src/engine/critical_scheduler.rs` e `src/engine/css_loader.rs` executam despacho assíncrono concorrente sobre o runtime Tokio.
- **`ace_core` (`Albedo_Core_Engine/ace_core/Cargo.toml`):**
  - Dependência: `tokio = { version = "1.40", features = ["sync", "rt", "time"] }`.
  - Primitivas de sincronização assíncrona e timers auxiliares de alta resolução.
- **Desacoplamento do Event Loop da Página:**
  - O Event Loop de execução da página web (`ace_core::event_loop`) opera com fila de microtarefas e tarefas conforme o padrão WHATWG, mantendo a camada de rede do Tokio estritamente confinada ao I/O assíncrono sem contaminar a ordem determinística do DOM e scripts.

