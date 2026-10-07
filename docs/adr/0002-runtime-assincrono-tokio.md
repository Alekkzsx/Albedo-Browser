# ADR-0002: Adoção do Runtime Assíncrono Tokio

- **Status:** Aceito
- **Data:** 2026-08-16
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

