# ADR-0005: Protocolo Binário de IPC Próprio

- **Status:** Aceito & Implementado
- **Data:** 2026-08-16 (Implementado na Fase 2 / Resolução de DV-01)
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_ipc`, Arquitetura Multi-Processo

---

## 1. Contexto e Problema

Com a divisão do Albedo em múltiplos processos (Browser, Renderer por site e Network Service), a comunicação inter-processo (IPC) é o gargalo central de latência de quadros e navegação. Utilizar serialização textual (JSON) ou formatos com reflexão dinâmica degrada a performance de forma inaceitável.

## 2. Decisão

Desenvolver um **protocolo binário tipado de alta velocidade próprio** em `ace_ipc`, gerando payloads de serialização direta zero-copy orientada a enums tipados em Rust, sem depender de formatos lentos em tempo de execução.

## 3. Consequências

- **Positivas:** Latência de despacho sub-microssegundo, validação estrita de tipos na fronteira de processos e segurança contra injeção de mensagens malformadas.
- **Negativas / Riscos:** Exige implementação manual rigorosa e testes exaustivos das mensagens do protocolo.

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado no crate `ace_ipc` com quitação integral da dívida técnica DV-01 e throughput medido de 2.221.596 msg/s.

- **`ace_ipc` (`Albedo_Core_Engine/ace_ipc/`):**
  - Diretiva de Segurança: `#![forbid(unsafe_code)]` ativada em `src/lib.rs`.
  - Módulo `src/protocol.rs`: Enum mestre `IpcMessage` (`Navigate`, `LoadResource`, `ResourceData`, `DomMutation`, `RenderFrame`, `InputEvent`, `Shutdown`) e payloads tipados (`DomMutationPayload`, `ResourceRequestPayload`, `ResourceDataPayload`, `RenderFramePayload`, `InputEventPayload`).
  - Módulo `src/channel.rs`: Implementação de `bounded_channel(capacity)` e `unbounded_channel()` construídos sobre `crossbeam-channel` com semântica de ring-buffer delimitado, provendo `IpcSender<T>` e `IpcReceiver<T>`.
  - Módulo `src/router.rs`: Roteador Pub/Sub thread-safe multi-tópico (`IpcRouter`) com tópicos `Topic::Navigation`, `Topic::Network`, `Topic::Dom`, `Topic::Render`, `Topic::Input`.
  - Módulo `src/error.rs`: Enum `IpcError` tipado via `thiserror`.
- **Validação de Estresse:**
  - `tests/stress_ipc_ring_buffer.rs`: Transmissão concorrente de 1.000.000 de mensagens por 4 threads em 0,450s atingindo **2.221.596 msg/s** sem perda de dados (superando o critério de sucesso > 1.5M msg/s).

