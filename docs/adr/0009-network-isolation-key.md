# ADR-0009: Particionamento Triplo de Estado de Rede (NetworkIsolationKey)

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_net`

---

## 1. Contexto e Problema

O compartilhamento global de caches de rede, sockets abertos e tabelas de DNS permite que sites maliciosos embutidos em iframes realizem ataques de canal lateral de temporização (*Cache Timing Attacks*) e rastreamento cross-site sem consentimento.

## 2. Decisão

Adotamos a chave tripla canônica **`NetworkIsolationKey`** `(TopLevelSite, FrameSite, is_cross_site)` para particionar estritamente:
- O Cache HTTP RFC 9111.
- O pool de conexões e sockets TCP/QUIC.
- O Cookie Jar (com suporte nativo a cookies CHIPS).

## 3. Consequências

- **Positivas:** Blindagem da privacidade do usuário contra rastreamento de terceiros e eliminação de ataques de temporização entre origens.
- **Negativas / Riscos:** Pequeno aumento na taxa de repetição de downloads para recursos genéricos (ex: bibliotecas JS de CDN compartilhadas entre múltiplos domínios distintos).

