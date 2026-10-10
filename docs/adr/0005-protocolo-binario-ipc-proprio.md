# ADR-0005: Protocolo Binário de IPC Próprio

- **Status:** Aceito
- **Data:** 2026-08-16
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

