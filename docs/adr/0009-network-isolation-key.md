# ADR-0009: Particionamento Triplo de Estado de Rede (NetworkIsolationKey)

- **Status:** Aceito & Implementado
- **Data:** 2026-08-16 (Implementado na Fase 3)
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

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado em produção no crate `ace_net` nos módulos de segurança, cache e cookies.

- **`ace_net` (`Albedo_Core_Engine/ace_net/`):**
  - Módulo `src/security/isolation.rs`: Estrutura canônica `NetworkIsolationKey` contendo a tripla `(TopLevelSite, FrameSite, is_cross_site)`.
  - Módulo `src/cache/partition.rs`: Particionamento formal do armazenamento de cache HTTP RFC 9111 em L1 (RAM) e L2 (disco), impedindo vazamento de histórico e ataques de temporização entre domínios.
  - Módulo `src/transport/pool.rs`: Segregação de sockets e pools de conexões TCP/TLS por chave de isolamento, prevenindo *connection reuse cross-site*.
  - Módulo `src/cookie/`: Suporte nativo a cookies particionados CHIPS (*Cookies Having Independent Partitioned State*), associando a persistência ao contexto de navegação primário.

