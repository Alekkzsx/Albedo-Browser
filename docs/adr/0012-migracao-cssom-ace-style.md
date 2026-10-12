# ADR-0012: Localização do CSSOM e Migração para ace_style na Fase 6

- **Status:** Aceito & Implementado
- **Data:** 2026-10-07 (Implementado em 2026-10-11 na Fase 6)
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_dom`, `ace_style`

---

## 1. Contexto e Problema

Durante as primeiras iterações do desenvolvimento do motor, um modelo embrionário de regras CSS e cálculo de estilo inline foi colocado provisoriamente dentro de `ace_dom/src/cssom` para permitir testes integrados. Essa decisão provisória gerou a dívida técnica formalizada como **DV-05** no plano diretor do projeto.

O crate `ace_dom` tem como responsabilidade única o ciclo de vida do DOM, o parser HTML5 e a arena de nós. Manter o CSSOM, o parser de CSS Syntax e o algoritmo de cascata dentro de `ace_dom` viola o desacoplamento do pipeline, infla o tempo de compilação do DOM e impede otimizações especializadas de estilo (como cache de nós irmãos e paralelização do cálculo de seletores).

## 2. Decisão

Aprovou-se a migração definitiva de todo o CSSOM, parser de sintaxe CSS e motor de cascata para o crate dedicado `Albedo_Core_Engine/ace_style` durante a execução da **Fase 6**:

1. **Parser Dedicado:** Implementação de tokenizer e parser streaming em conformidade com W3C CSS Syntax Level 3.
2. **Indexação por Baldes:** Criação do `RuleBucketMap` (ID, Class, Tag e Universal buckets) integrado com o `AncestorFilter` (Bloom filter) do `ace_dom`.
3. **Cascata e Camadas:** Matriz formal das 8 origens normativas da W3C e suporte completo à ordenação de `@layer` (CSS Cascade 5).
4. **Resolução de Variáveis:** Algoritmo em Grafo Acíclico Dirigido (DAG) com mitigação de estouro de pilha e proteção contra ataques DoS (*Billion Laughs CSS*).
5. **Otimizações:** `StyleSharingCache` para reaproveitamento de estilos em nós irmãos e geração da `RenderTree` imutável.
6. **Segurança:** Envolvimento de todo o subsistema em 100% Safe Rust com `#![forbid(unsafe_code)]`.

## 3. Consequências

- **Positivas:**
  - Resolução integral da dívida arquitetural **DV-05**.
  - Pipeline estritamente desacoplado (`ace_dom` -> `ace_style` -> `ace_layout`).
  - Suporte a paralelismo limpo na resolução de estilos via `rayon`.
  - Cobertura de testes e suítes de estresse independentes (25 testes automatizados em 14 binários).
- **Negativas / Riscos:**
  - Exige que o `StyleEngine` receba dados do elemento através da interface da arena do DOM (`NodeId`, tags, atributos e classes) em vez de acoplamento direto de ponteiros.

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado no crate `ace_style` na Fase 6, quitando integralmente a dívida técnica DV-05 com `#![forbid(unsafe_code)]` e 25 testes verdes em 14 binários.

- **`ace_style` (`Albedo_Core_Engine/ace_style/`):**
  - Diretiva de Segurança: `#![forbid(unsafe_code)]` na raiz de `src/lib.rs`.
  - Módulos do CSSOM e Parser: `src/parser/` e `src/cssom/` implementando tokenizer e parser streaming em conformidade com W3C CSS Syntax Level 3.
  - Cascata e Regras: `src/cascade/` implementando a matriz formal das 8 origens normativas da cascata W3C e resolução de `@layer` (CSS Cascade 5).
  - Resolução de Variáveis: `src/engine/` implementando resolução acíclica em DAG de `var()` com mitigação de ataques *Billion Laughs CSS*.
  - Classificação de Caixas: `src/engine/style_engine.rs` exportando o enum `BoxGenerationKind` (`Block`, `Inline`, `InlineBlock`, `Flex`, `Grid`, `Anonymous`, `TextRun`) como contrato com `ace_layout`.
  - Render Tree: `src/computed/` e `RenderTree` desacoplados da arena do DOM, gerando árvores imutáveis com `Arc<ComputedStyle>`.
- **Desacoplamento de `ace_dom`:**
  - O código temporário de CSSOM foi removido de `ace_dom`, mantendo o DOM estritamente focado no ciclo de vida de nós e parser HTML5.

