# ADR-0006: Arquitetura de Cache HTTP RFC 9111 em Dois Níveis

- **Status:** Aceito & Implementado
- **Data:** 2026-08-16 (Implementado na Fase 3)
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_net`

---

## 1. Contexto e Problema

O desempenho do carregamento de páginas web depende criticamente da capacidade de evitar requisições redundantes à rede, obedecendo às regras de frescor, revalidação e diretivas de `Cache-Control` estipuladas pela especificação da IETF.

## 2. Decisão

Implementar no `ace_net` um **Cache Semântico em Dois Níveis** em conformidade estrita com a **RFC 9111**:
1. **L1 RAM:** Tabela de capacidade limitada na memória com política de descarte LRU para acesso instantâneo a sub-recursos ativos.
2. **L2 Disk (WAL):** Persistência atômica em disco com índice binário recuperável e log de escrita adiantada (*Write-Ahead Logging*).

## 3. Consequências

- **Positivas:** Conformidade normativa real com a web, suporte a `ETag`, `304 Not Modified`, `stale-while-revalidate` e resiliência a crashes do navegador.
- **Negativas / Riscos:** Complexidade de sincronização e concorrência entre threads. Mitigado com particionamento estrito de chaves.

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado em produção no crate `ace_net` com suporte total a RFC 9111 e particionamento por chave de rede.

- **`ace_net` (`Albedo_Core_Engine/ace_net/src/cache/`):**
  - Módulo `src/cache/storage.rs`: Cache volátil L1 em memória com desalocação LRU e limitação estrita de consumo de bytes.
  - Módulo `src/cache/disk.rs`: Cache persistente L2 em disco baseado em WAL transacional (*Write-Ahead Logging*), garantindo atomicidade e recuperação após falhas anormais do processo.
  - Módulo `src/cache/partition.rs`: Particionamento formal de chaves via `NetworkIsolationKey`, blindando o cache contra ataques de canal lateral de temporização (*Cache Timing Attacks*).
  - Módulo `src/cache/policy.rs`: Motor de políticas RFC 9111 implementando cálculo de vida útil de frescor (*freshness lifetime*), diretivas `max-age`, `no-store`, `no-cache`, `must-revalidate`, `stale-while-revalidate`, e revalidação condicional (cabeçalhos `If-None-Match` / `ETag` e `If-Modified-Since` / `Last-Modified` com resposta `304 Not Modified`).
- **Integração no Pipeline:**
  - `src/engine/css_loader.rs`: Consulta direta ao cache L1/L2 antes de despachar requisições para a rede.

