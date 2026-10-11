# ADR-0012: Localização do CSSOM e Migração para ace_style na Fase 6

- **Status:** Aceito & Implementado (Fase 6)
- **Data:** 2026-10-07 (Implementado em 2026-10-11)
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
  - Cobertura de testes e suítes de estresse independentes (18 testes automatizados, 6 baterias de estresse pesado).
- **Negativas / Riscos:**
  - Exige que o `StyleEngine` receba dados do elemento através da interface da arena do DOM (`NodeId`, tags, atributos e classes) em vez de acoplamento direto de ponteiros.
