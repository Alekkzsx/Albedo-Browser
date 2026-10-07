# ADR-0021: Regra de Evidência para Atualizações de Status

- **Status:** Aceito
- **Data:** 2026-10-07
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

