# Original User Request

## 2026-08-27T18:10:00Z

Realizar uma auditoria exaustiva e formular o Plano Mestre Definitivo em nível extremo para o subsistema `ace_dom` do Albedo Browser, cobrindo conformidade total WHATWG/W3C, benchmarks arquiteturais contra motores SOTA (Blink, WebKit, Gecko, Servo, Ladybird), eliminação de lacunas (CSSOM, Tree Construction, WebIDL, WPT) e roadmap técnico de implementação.

Working directory: c:\Users\24802449\Documents\Github\Albedo-Browser
Integrity mode: development

## Requirements

### R1. Auditoria e Varredura Completa do `ace_dom` Atual
- Mapear exaustivamente todo o código existente em `Albedo_Core_Engine/ace_dom` (nós, arena, tokenizer, tree builder, query engine, observers, ranges, CSSOM, bindings e form validity).
- Identificar todas as discrepâncias, métodos faltantes da especificação DOM Level 4/HTML5, limitações algorítmicas e pontos de atenção de performance.

### R2. Pesquisa Exaustiva de Lacunas e Melhorias de Ponta (SOTA)
- Mapear todas as lacunas da especificação WHATWG HTML §12, W3C DOM, CSS Selectors 4, CSSOM Cascading & Inheritance e WebIDL.
- Comparar cada subsistema com as soluções de ponta da indústria:
  - Resolução de especificidade e valores computados do CSSOM.
  - Modos de inserção de tabelas, framesets e pontos de integração MathML/SVG.
  - Otimizações de zero-copy e SIMD no tokenizer.
  - APIs essenciais ausentes (`cloneNode`, `matches`, `closest`, `slot` projection, etc.).

### R3. Consolidação do Plano Mestre ao Extremo (`ACE_DOM_MASTER_PLAN.md`)
- Consolidar um documento técnico detalhado, exaustivo e pragmático contendo:
  - Matriz comparativa técnica completa.
  - Decomposição em Milestones com tarefas granulares, contratos de interface e estruturas de dados.
  - Roteiro de integração do runner oficial de Web Platform Tests (WPT).
  - Estratégia de compilação/bindings WebIDL e acoplamento com o futuro motor JS (`ace_js`).

### R4. Verificação e Revisão Crítica Adversária do Plano
- Submeter o plano mestre a uma auditoria interna rigorosa (verificador/challenger) para encontrar furos de especificação, riscos de regressão, complexidade assintótica oculta ou inviabilidades arquiteturais antes do sinal verde.

## Acceptance Criteria

### Qualidade e Rigor Técnico
- [ ] O documento `ACE_DOM_MASTER_PLAN.md` cobre 100% dos subsistemas do `ace_dom` com granularidade de implementação.
- [ ] Todos os métodos ausentes identificados possuem especificação de assinatura, complexidade assintótica e algoritmo associado.
- [ ] O plano de testes inclui o harness do Web Platform Tests (WPT) com execução de suítes `.dat`.
- [ ] A revisão adversária validou a consistência com `ace_core` e as regras de ouro do `PLANO.md` (Safe Rust, Arena geracional de 8 bytes, zero dependências proibidas).
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` e `cargo test --workspace` permanecem 100% verdes.
