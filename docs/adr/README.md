# 🏛️ Architecture Decision Records (ADRs) — Albedo Browser

> **Diretório de Registro de Decisões Arquiteturais**  
> **Status:** Ativo  
> **Conformidade:** Regra 3 da Constituição de Engenharia ([PLANO.md](../../PLANO.md) — §2): *Toda decisão arquitetural relevante vira um ADR*.

---

## 1. O Que É um ADR?

Um **Architecture Decision Record (ADR)** é um documento que captura uma decisão arquitetural importante tomada pela equipe, juntamente com seu contexto, justificativa técnica, alternativas consideradas e consequências (trade-offs).

Para propor uma nova decisão de arquitetura, duplique o [**Template Oficial (0000-template.md)**](./0000-template.md) com a numeração sequencial correspondente.

---

## 2. Catálogo Geral de ADRs do Projeto

| ADR | Título | Subsistema | Status | Resumo da Decisão |
| :---: | :--- | :--- | :---: | :--- |
| [**0000**](./0000-template.md) | Template de ADR | Governança | **Aceito** | Estrutura padrão para registros arquiteturais. |
| **0001** | Migração para Rust Edition 2024 | Workspace | **Pendente** | Decisão de migração de Edition 2021 para 2024 quando o ecossistema estiver maduro. |
| [**0002**](./0002-runtime-assincrono-tokio.md) | Runtime Assíncrono Primário (`tokio`) | `ace_core`, `ace_net` | **Aceito** | Adoção de `tokio` como fundação de I/O de rede não-bloqueante. |
| [**0003**](./0003-pilha-criptografia-tls-rustls.md) | Pilha de Criptografia e TLS (`rustls`) | `ace_net` | **Aceito** | Eliminação de dependências C/OpenSSL, utilizando `rustls` com backend `aws-lc-rs`. |
| [**0004**](./0004-modelo-memoria-dom-arena.md) | Modelo de Memória do DOM (Arena Geracional EBR) | `ace_dom` | **Aceito** | Arenas com Epoch-Based Reclamation evitando ciclos de referência e vazamentos de memória. |
| [**0005**](./0005-protocolo-binario-ipc-proprio.md) | Protocolo Binário de IPC Próprio | `ace_ipc` | **Aceito** | Protocolo de serialização zero-copy tipado entre processos do navegador. |
| [**0006**](./0006-cache-http-rfc9111.md) | Cache HTTP RFC 9111 em Dois Níveis (L1 + L2 WAL) | `ace_net` | **Aceito** | Memória volátil rápida (L1) combinada com WAL atômico em disco (L2). |
| **0007** | GC Unificado JS ↔ DOM para Ciclos de Referência | `ace_dom`, `ace_js` | **Aceito** | Barramento de rastreamento de GC para manter liveness de nós do DOM referenciados por JS. |
| **0008** | Happy Eyeballs v2 e DNS-over-HTTPS (DoH) Nativo | `ace_net` | **Aceito** | Conexões paralelas IPv4/IPv6 conforme RFC 8305. |
| [**0009**](./0009-network-isolation-key.md) | Particionamento Triplo de Estado de Rede (`NetworkIsolationKey`)| `ace_net` | **Aceito** | Defesa contra ataques de timing de cache particionando sockets e armazenamento por site. |
| **0010** | Compositor Off-Main-Thread e Display Lists | `ace_gfx` | **Proposto** | Separação da thread de composição da thread de layout/JavaScript para rolagem a 60+ FPS. |
| **0011** | Text Shaping e Font Fallback Multilíngue | `ace_layout` | **Proposto** | Seleção entre HarfRust / rustybuzz e integração de fontes locais via `fontdb`. |
| **0012** | Localização do CSSOM e Migração para `ace_style` na Fase 6 | `ace_dom`, `ace_style`| **Aceito** | Resolução da dívida DV-05: mover CSSOM temporário de `ace_dom` para `ace_style`. |
| **0013** | Pilha Tipográfica: HarfRust vs rustybuzz & skrifa vs ttf-parser | `ace_layout` | **Proposto** | Escolha das crates de fundação para layout tipográfico de alto desempenho. |
| **0014** | Adoção do AccessKit como Fundação de Acessibilidade | `ace_ui` | **Aceito** | Mapeamento da Accessibility Tree própria para as APIs nativas do SO via `accesskit`. |
| [**0015**](./0015-modelo-processos-site-isolation.md) | Modelo de Processos com Isolamento por Site (*Site Isolation*) | Arquitetura | **Aceito** | Cada site (eTLD+1) isolado em um processo de Renderer independente no SO. |
| **0016** | Política de Tipos de Erro (`thiserror` em libs, `anyhow` em bins) | `ace_core` | **Aceito** | Padronização dos tipos de erro nos crates de biblioteca pública. |
| **0017** | Tipos Geométricos 2D/3D via `euclid` | `ace_core` | **Aceito** | Uso da crate `euclid` para geometria tipada com unidades de espaço de renderização. |
| **0018** | Endurecimento da Denylist e Allowlist no `deny.toml` | Governança | **Proposto** | Proibição de bibliotecas pré-fabricadas concorrentes (Blink, Servo, Boa, etc.). |
| [**0019**](./0019-motor-js-interpretador-jitless.md) | Motor JS: Interpretador Bytecode no MVP e Modo *Jitless* Permanente | `ace_js` | **Aceito** | Eliminação do JIT no MVP; modo interpretado permanente como mitigação de vulnerabilidades. |
| **0020** | Rasterizador CPU de Fallback (`tiny-skia` vs `vello_cpu`) | `ace_gfx` | **Proposto** | Fallback seguro para ambientes de renderização sem suporte a GPU ou em CI. |
| [**0021**](./0021-regra-evidencia-status.md) | Regra de Evidência para Atualizações de Status | Governança | **Aceito** | Nenhuma fase ou subsistema avança para ✅ sem comprovação de testes e CI verde. |

