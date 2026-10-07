# 📚 Albedo Browser & Engine — Central de Documentação Técnica

Bem-vindo à base de conhecimento e documentação de arquitetura do **Albedo Browser & Engine (ACE)**. Este diretório centraliza especificações técnicas, planos diretores de subsistemas, limites normativos de escopo, registros de decisões e relatórios de auditoria.

> [!NOTE]
> Para o planejamento estratégico geral, regras de governança e cronograma de todas as 14 fases de engenharia do motor, consulte o [**PLANO.md**](../PLANO.md) na raiz do repositório (Fonte da Verdade Suprema).

---

## 🧭 Índice Geral da Documentação

```
docs/
├── README.md                 # Este índice central e painel de status
├── architecture/             # Especificações profundas por subsistema (Master Plans)
│   ├── ace_core.md           # Arquitetura Fundacional, Arenas EBR, Concorrência & Matemática
│   ├── ace_net.md            # Arquitetura de Rede, Cache RFC 9111, DoH, HTTP/3 & Segurança
│   ├── ace_dom.md            # Parser HTML5, Árvore DOM imutável, CSSOM & Algoritmos WHATWG
│   └── test_infra.md         # Infraestrutura, filosofia e estratégia de testes em 4 estágios
├── scope/                    # Escopo normativo do MVP (v1.0 Desktop)
│   ├── mvp_scope.md          # O que entra na v1.0 e o que fica pós-MVP (Fonte de Escopo)
│   ├── css_properties.md     # Subconjunto de propriedades CSS suportadas no MVP
│   └── web_api.md            # Catálogo de APIs JavaScript e DOM essenciais para SPAs
├── reports/                  # Relatórios de validação, estresse e auditoria técnica
│   └── fase2_ace_core.md     # Relatório de auditoria e métricas dos testes do ace_core
└── adr/                      # Architecture Decision Records (Registros de Decisões)
    ├── README.md             # Catálogo de todos os 21 ADRs do projeto
    └── 0000-template.md      # Template oficial para novos ADRs
```

---

## 📜 Histórico de Revisões e Governança da Documentação

| Versão | Data | Contexto / Marco | Decisões & Escolhas de Governança | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.0.0** | 2026-10-07 | Auditoria e Reestruturação v7.0 | • Criação de `docs/architecture/ace_core.md` e `Albedo_Core_Engine/ace_core/README.md`.<br>• Catálogo unificado dos 21 ADRs em `docs/adr/README.md`.<br>• Alinhamento estrito com a Regra de Evidência e Paradigma Pragmático. | • Todos os 9 documentos sincronizados sem contradições.<br>• Painel de estado real auditado em conformidade com o código.<br>• Rastreabilidade temporal em tabelas padronizadas em todos os docs. |
| **1.0.0** | 2026-08-16 | Estruturação Inicial da Base de Conhecimento | • Separação de pastas em `architecture/`, `scope/`, `reports/`, `adr/`. | • Criação dos primeiros master plans de `ace_dom` e `ace_net`. |

---

## 📊 Painel de Estado Real dos Subsistemas (Auditado)

| Subsistema / Crate | Documento Vinculado | Fase no PLANO | Status Real | Evidência & Próximos Passos |
| :--- | :--- | :---: | :---: | :--- |
| **`ace_core`** (Fundação) | [`ace_core.md`](./architecture/ace_core.md) / [`Relatório`](./reports/fase2_ace_core.md) | Fase 2 | 🟡 Parcial | 101 testes unitários aprovados. Fase 2 permanece 🟡 até a implementação do IPC in-process em `ace_ipc` (DV-01). |
| **`ace_net`** (Rede & Cache) | [`ace_net.md`](./architecture/ace_net.md) | Fase 3 | 🚧 Fechamento | HTTP/1.1, H2, H3/QUIC, Cache RFC 9111 L1/L2 e DoH implementados. Suíte de 71+ testes e estresse de 93s passando. Em fechamento de PR para a `main`. |
| **`ace_dom`** (Parser & DOM) | [`ace_dom.md`](./architecture/ace_dom.md) | Fase 5 | 🟡 Parcial | Tokenizer FSM, Tree Builder (AAA) e Arena operacionais. Refatoração Clean Code do `process_token` ativa. Pendente vendorizar suíte oficial `html5lib-tests` (DV-11). |
| **`ace_style`** (Estilo & Cascata) | [`css_properties.md`](./scope/css_properties.md) | Fase 6 | ⏳ Planejado | Código inicial de CSSOM reside temporariamente em `ace_dom/src/cssom`. A Fase 6 migrará o CSSOM para `ace_style` e implementará CSS Syntax L3 (ADR-0012). |
| **`ace_js`** (Motor JavaScript) | [`web_api.md`](./scope/web_api.md) | Fase 10 | ⏳ Planejado | MVP planejado estritamente com interpretador de bytecode próprio (**sem JIT**, modo *jitless* permanente conforme ADR-0019). |
| **`ace_ipc`** (Comunicação) | [`adr/README.md`](./adr/README.md) (ADR-0005) | Fase 2 / 12 | 🟡 Stub | Atualmente contém apenas 1 linha (`pub fn init() {}`). Alvo imediato de implementação para fechar a Fase 2. |
| **Infra de Testes** | [`test_infra.md`](./architecture/test_infra.md) | Transversal | 🟡 Em Evolução | Estratégia de conformidade em 4 estágios aprovada (Estágio A: html5lib `.dat`; Estágio B: Reftests; Estágio C: testharness.js; Estágio D: WebDriver BiDi). |

---

## 🏛️ 1. Arquitetura dos Subsistemas (`docs/architecture/`)

- [**`ace_core.md`**](./architecture/ace_core.md):
  - Fundação do motor com 18 subsistemas de infraestrutura, matemática e concorrência.
  - Arenas geracionais com *Epoch-Based Reclamation* (EBR) e eliminação de ABA.
  - Otimização de nicho (`Option<NodeId>` em 8 bytes) e `TripleBuffer` atômico lock-free.
  - Event Loop WHATWG com filas segregadas por `TaskSource` e microtask checkpoints.
  - Tipos geométricos `euclid`, aritmética `LayoutUnit` base 60 e cores CSS Color 4 (Oklab/Oklch).
  - Modelo formal de origens seguras (RFC 6454), Same-Origin Policy e defesas contra Spectre.

- [**`ace_net.md`**](./architecture/ace_net.md):
  - Orquestrador de requisições `ResourceFetcher` com limite de 1MB para bufferização de cache e streaming assíncrono para payloads maiores.
  - Cache HTTP em memória e disco conforme a **RFC 9111** (L1 RAM + L2 transacional WAL).
  - Resolução DNS-over-HTTPS (DoH) com **Happy Eyeballs v2** (RFC 8305) e suporte a Early Hints (HTTP 103).
  - Particionamento triplo de estado de rede (`NetworkIsolationKey`), cookies CHIPS e isolamento contra ataques de canal lateral.
  - Conector nativo QUIC / HTTP/3 e telemetria W3C / HAR.

- [**`ace_dom.md`**](./architecture/ace_dom.md):
  - Tokenizador streaming WHATWG §12 com otimizações SIMD e recuperação de erros.
  - Tree Builder com os 16 passos completos do **Adoption Agency Algorithm (AAA)**.
  - Motor de seletores CSS4 Right-to-Left (RTL) com **Ancestor Bloom Filter** de 64 buckets.
  - Modelo de memória baseado em Arena SlotMap geracional (`Arena<NodeData>`, zero vazamentos cíclicos).
  - Suporte a Declarative Shadow DOM (DSD), MutationObserver, Live Ranges e Sanitizer API.

- [**`test_infra.md`**](./architecture/test_infra.md):
  - Arquitetura de testes opacos em 4 camadas (*Tier 1: Feature Coverage*, *Tier 2: Boundaries*, *Tier 3: Pairwise*, *Tier 4: Workloads Reais*).
  - Matriz dos 4 Estágios de Conformidade WPT/Test262 (Estágios A, B, C e D).
  - Inventário verificado das baterias de testes em execução no workspace.

---

## 🎯 2. Escopo Normativo do MVP (`docs/scope/`)

Para manter o foco na sobrevivência e estabilidade da v1.0, o projeto adota limites estritos de escopo:

- [**`mvp_scope.md`**](./scope/mvp_scope.md): Delimita a fronteira do que entra na versão 1.0 (sobrevivência e renderização de sites essenciais em desktop) versus pós-MVP (JIT complexo, aceleração total GPU para web, WebRTC, extensões).
- [**`css_properties.md`**](./scope/css_properties.md): Lista taxativa das propriedades CSS suportadas no MVP (Display, Positioning, Box Model, Tipografia, Cores, Flexbox, Grid básico e `@layer`).
- [**`web_api.md`**](./scope/web_api.md): Catálogo de APIs JavaScript do DOM, EventTarget, Timers, Fetch, Web Workers e History API que integram com o futuro motor `ace_js`.

---

## 📊 3. Relatórios e Auditorias (`docs/reports/`)

- [**`fase2_ace_core.md`**](./reports/fase2_ace_core.md): Relatório de auditoria técnica do `ace_core`, comprovando a aprovação de 101 testes unitários/integração e detalhando a dívida pendente do `ace_ipc` para a conclusão total da Fase 2.

---

## 📜 4. Governança e Decisões de Arquitetura (`docs/adr/`)

Conforme a **Regra 3** do projeto: *Toda decisão arquitetural relevante vira um ADR*.

- [**Catálogo de ADRs (`docs/adr/README.md`)**](./adr/README.md): Lista indexada de todos os 21 registros de decisões arquiteturais do projeto (ADR-0000 a ADR-0021).
- Novos registros devem ser criados na pasta `docs/adr/` numerados sequencialmente, utilizando o [**Template de ADR (0000-template.md)**](./adr/0000-template.md).
