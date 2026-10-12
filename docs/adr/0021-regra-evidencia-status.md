# ADR-0021: Regra de Evidência para Atualizações de Status

- **Status:** Aceito & Implementado
- **Data:** 2026-10-07 (Implementado no Pipeline de CI e Governança)
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** Governança Global, `PLANO.md`

---

## 1. Contexto e Problema

Auditorias no repositório revelaram discrepâncias onde subsistemas foram narrativamente declarados como "100% concluídos" ou "0 unsafe" sem que houvesse testes automatizados passando no CI ou código materializado correspondente (ex: `ace_ipc` sendo apenas um stub enquanto a Fase 2 era marcada como concluída).

## 2. Decisão

Adotamos a **Regra de Evidência (Regra de Ouro 5)** como política inegociável de governança. Nenhuma fase ou entrega avança para o status **✅ Concluído** sem cumprir três condições simultâneas:
1. **Caminho:** Arquivo de código rastreável e não-stub no repositório.
2. **Teste:** Suíte automatizada de testes que exercite a funcionalidade exigida pelo DoD.
3. **CI Verde:** Commit passando em todos os linters, formatação, deny e suítes de teste do workspace.

## 3. Consequências

- **Positivas:** Eliminação de dívidas técnicas invisíveis e garantia de que o projeto é auditável e confiável em nível corporativo.
- **Negativas / Riscos:** Exige maior rigor e impede atualizações precipitadas de status em documentações.

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e aplicado ativamente na governança de engenharia e no pipeline de CI do Albedo.

- **Validação de Código Real:**
  - Proibição de arquivos *stub* em fases marcadas como concluídas. Toda funcionalidade declarada nos relatórios e ADRs é rastreável a módulos concretos dentro de `Albedo_Core_Engine/`.
- **Validação Contínua por Testes:**
  - Suíte completa com exatamente 1.024 testes verdes executados via `cargo test --workspace` em 181 binários de teste.
  - Zero warnings no compilador e no Clippy (`cargo clippy --workspace --all-targets -- -D warnings`).
- **Resolução de Dívidas Técnicas:**
  - Quitação comprovada com testes automatizados de DV-01 (`ace_ipc` com taxa de 2.22M msg/s), DV-05 (migração de CSSOM para `ace_style`), DV-09 (`tempfile` no workspace) e DV-11 (runner normativo de `html5lib-tests` .dat com 7 de 7 cenários oficiais aprovados).
- **Rastreabilidade de Segurança:**
  - Registro exaustivo de exatamente 66 ocorrências de código `unsafe` concentradas exclusivamente em 11 arquivos de `ace_core`, com imposição de `#![forbid(unsafe_code)]` em `ace_ipc`, `ace_browser` e `ace_style`.

