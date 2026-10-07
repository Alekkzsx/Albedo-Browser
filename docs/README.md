# 📚 Albedo Browser & Engine — Central de Documentação Técnica

Bem-vindo à base de conhecimento e documentação de arquitetura do **Albedo Browser & Engine (ACE)**. Este diretório centraliza especificações técnicas, planos diretores de subsistemas, limites normativos de escopo e relatórios de auditoria.

> [!NOTE]
> Para o planejamento estratégico geral e cronograma de todas as 14 fases de engenharia do motor, consulte o [**PLANO.md**](../PLANO.md) na raiz do repositório.

---

## 🧭 Índice Geral da Documentação

```
docs/
├── README.md                 # Este índice central
├── architecture/             # Especificações profundas por subsistema (Master Plans)
│   ├── ace_net.md            # Arquitetura de Rede, Cache RFC 9111, DoH, HTTP/3 & Segurança
│   ├── ace_dom.md            # Parser HTML5, Árvore DOM imutável, CSSOM & Algoritmos WHATWG
│   └── test_infra.md         # Infraestrutura e filosofia de testes 4-Tier do motor
├── scope/                    # Escopo normativo do MVP (v1.0)
│   ├── mvp_scope.md          # O que entra e o que fica pós-MVP
│   ├── css_properties.md     # Subconjunto de propriedades CSS suportadas inicialmente
│   └── web_api.md            # Conjunto de Web APIs e objetos globais JS do MVP
├── reports/                  # Relatórios de validação, estresse e auditoria técnica
│   └── fase2_ace_core.md     # Relatório de aprovação dos testes E2E do ace_core
└── adr/                      # Architecture Decision Records (Registros de Decisões)
    └── 0000-template.md      # Template oficial para novos ADRs
```

---

## 🏛️ 1. Arquitetura dos Subsistemas (`docs/architecture/`)

Os documentos de arquitetura aprofundam a especificação de cada crate do motor **ACE**:

- [**`ace_net.md`**](./architecture/ace_net.md):
  - Orquestrador de requisições `ResourceFetcher` com streaming reativo sem buffering total.
  - Cache HTTP em memória e disco conforme a **RFC 9111** (L1 RAM + L2 transacional WAL).
  - Resolução DNS-over-HTTPS (DoH) com **Happy Eyeballs v2** (RFC 8305) e suporte a Early Hints (HTTP 103).
  - Particionamento triplo de estado de rede (`NetworkIsolationKey`), cookies CHIPS e defesas contra side-channels.
  - Conector QUIC / HTTP/3, WebTransport e telemetria W3C / HAR.

- [**`ace_dom.md`**](./architecture/ace_dom.md):
  - Tokenizador streaming WHATWG §12 com otimizações SIMD.
  - Tree Builder com os 16 passos completos do **Adoption Agency Algorithm (AAA)**.
  - Motor de seletores CSS4 Right-to-Left (RTL) com **Ancestor Bloom Filter** de 64 buckets.
  - Modelo de memória baseado em Arena SlotMap geracional (`100% Safe Rust`, zero vazamentos cíclicos).
  - Suporte a Declarative Shadow DOM (DSD), MutationObserver, Live Ranges e Sanitizer API.

- [**`test_infra.md`**](./architecture/test_infra.md):
  - Arquitetura de testes opacos em 4 camadas (*Tier 1: Feature Coverage*, *Tier 2: Boundaries*, *Tier 3: Pairwise*, *Tier 4: Workloads Reais*).
  - Critérios de aceitação para validação de conformidade e estresse de concorrência.

---

## 🎯 2. Escopo Normativo do MVP (`docs/scope/`)

Para manter o foco na sobrevivência e estabilidade da v1.0, o projeto adota limites estritos de escopo:

- [**`mvp_scope.md`**](./scope/mvp_scope.md): Delimita a fronteira do que é entregue na versão 1.0 (sobrevivência e renderização de sites essenciais) versus pós-MVP (JIT complexo, aceleração total GPU, WebRTC).
- [**`css_properties.md`**](./scope/css_properties.md): Lista taxativa das propriedades CSS suportadas no MVP (Display, Positioning, Box Model, Tipografia, Cores, Flexbox e Grid básico).
- [**`web_api.md`**](./scope/web_api.md): Catálogo de APIs JavaScript do DOM, EventTarget, Timers e Fetch que integram com o futuro motor `ace_js`.

---

## 📊 3. Relatórios e Auditorias (`docs/reports/`)

Registros comprobatórios de validação das fases entregues:

- [**`fase2_ace_core.md`**](./reports/fase2_ace_core.md): Relatório executivo de conformidade da Fase 2 (`ace_core`), atestando a aprovação de centenas de testes unitários e de integração através dos 4 tiers de robustez.

---

## 📜 4. Governança e Decisões de Arquitetura (`docs/adr/`)

Conforme a **Regra 3** do projeto: *Toda decisão arquitetural relevante vira um ADR*.

- Novos registros devem ser criados na pasta `docs/adr/` numerados sequencialmente (ex: `0001-escolha-do-modelo-de-memoria-dom.md`), utilizando o [**Template de ADR (0000-template.md)**](./adr/0000-template.md).

