# ADR-0006: Arquitetura de Cache HTTP RFC 9111 em Dois Níveis

- **Status:** Aceito
- **Data:** 2026-08-16
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

