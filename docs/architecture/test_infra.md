# 🧪 Infraestrutura e Estratégia de Testes do Motor ACE

> **Documento de Arquitetura de Testes e Conformidade**  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.0](../../PLANO.md) — §9)  
> **Última Atualização:** 2026-10-11  
> **Subsistemas Cobertos:** `ace_core`, `ace_dom`, `ace_net`, `ace_style`, `ace_test_driver`

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.1.0** | 2026-10-11 | Implementação do `ace_style` (Fase 6) | • Bateria de 6 suítes pesadas de estresse adversário (Fuzzing L3, DoH/DoS, concorrência, Billion Laughs CSS).<br>• Incorporação da cobertura normativa de CSS Syntax e Cascata. | • Inclusão do `ace_style` na matriz de testes com 18 testes e 7 binários.<br>• Verificação estrita de ausência de unsafe (`#![forbid(unsafe_code)]`). |
| **2.0.0** | 2026-10-07 | Alinhamento com PLANO v7.0 | • Adoção da Estratégia de Conformidade WPT em 4 Estágios (Estágios A, B, C e D).<br>• Incorporação formal da Regra de Evidência (Regra de Ouro 5). | • Expansão do documento para cobrir o motor completo além do `ace_core`.<br>• Registro das métricas ativas do workspace (101 testes em `ace_core`, 22+ em `ace_dom`, 71+ em `ace_net`). |
| **1.0.0** | 2026-08-16 | Definição Inicial | • Arquitetura de testes opacos 4-Tier para o `ace_core`. | • Especificação dos 219 casos de teste conceituais da fundação. |

---

## 1. Filosofia de Testes

1. **Testes Opaque-Box Orientados a Especificações:** Todo teste é modelado a partir das especificações normativas (WHATWG, W3C, RFCs da IETF e ECMA-262), exercitando interfaces e tipos públicos dos crates sem depender de detalhes internos frágeis de implementação.
2. **Evidência Antes de Status (Regra de Ouro 5):** Nenhuma funcionalidade é declarada concluída sem testes automatizados que comprovem o comportamento e passem verde no CI.
3. **Determinismo Absoluto:** Testes não dependem de estado compartilhado de rede pública, tempos de espera arbitrários (`sleep`) ou relógios do sistema não controlados (uso obrigatório de `MockClock` para temporizações).

---

## 2. A Estratégia de Conformidade em 4 Estágios (WPT & Test262)

Para evitar a ilusão de rodar o consórcio Web Platform Tests (WPT) completo antes do motor possuir as camadas necessárias, a conformidade normativa é escalonada em 4 estágios progressivos:

| Estágio | Suíte Normativa Alvo | Pré-requisito Técnico | Momento de Ativação | Status Atual |
| :---: | :--- | :--- | :---: | :---: |
| **Estágio A** | `html5lib-tests` (arquivos `.dat`) e `css-syntax` | Apenas os Parsers (sem renderização gráfica). Asserções puras sobre tokens e ASTs. | Fase 5 e Fase 6 | 🟢 `css-syntax` e cascata L5/L6 cobertos por 18 testes em `ace_style`; pendente vendorização dos arquivos `.dat` em `ace_test_driver` |
| **Estágio B** | *Layout-dump* e Reftests (sem JavaScript) | Motor de Layout e Pintura gerando coordenadas geométricas e bitmaps a partir de HTML estático. | Fase 7 e Fase 8 | ⏳ Aguardando Fase 7 |
| **Estágio C** | `testharness.js` embutido localmente | Motor `ace_js` integrado via WebIDL ao `ace_dom`, permitindo executar asserções JS diretamente na engine. | Fase 10 | ⏳ Aguardando Fase 10 |
| **Estágio D** | `wptrunner` oficial via **WebDriver BiDi** + Test262 | Servidor WebDriver BiDi controlando o navegador end-to-end com WebSockets. | Fase 13 | ⏳ Planejado pós-Fase 12 |

---

## 3. Arquitetura 4-Tier para Subsistemas de Baixo Nível (`ace_core`)

Para subsistemas fundamentais de infraestrutura e memória, os testes unitários e de integração são organizados em 4 camadas de rigor:

### Tier 1: Cobertura Funcional Isolada (Feature Coverage)
- Valida o comportamento nominal de cada primitiva (`InlineVec`, `TripleBuffer`, `Arena`, `UnguessableToken`, `LayoutUnit`, etc.).
- Mínimo de 5 casos de teste por primitiva.

### Tier 2: Casos de Borda e Limítrofes (Boundaries & Corners)
- Valores extremos (overflow aritmético, buffers vazios, strings nulas, multi-byte UTF-8 divididos em fronteiras de chunks, reentrância em observers).
- Testes com `Miri` para garantir ausência de *Undefined Behavior* em blocos `unsafe`.

### Tier 3: Combinações Cruzadas de Módulos (Pairwise Interactions)
- Integração de subsistemas complementares (ex.: `Origin` + `Referrer` + `Token`, `EventLoop` + `Timers` + `Microtasks`, `Color` Oklch + `LayoutUnit` snapping).

### Tier 4: Cenários Reais de Aplicação (Browser Workloads)
- Simulações de pipelines reais do navegador:
  1. Pipeline de navegação completo (parsing de URL, resolução de referrer, MIME sniffing).
  2. Produtor-consumidor de animação a 120 FPS via `TripleBuffer`.
  3. Carga massiva do Event Loop alternando microtarefas, timers e tarefas normais sem starvation.
  4. Mutação em lote de nós na Arena DOM com cálculo de flags de estilo sujo (*dirty flags*).
  5. Carga de estresse de memória com streaming de rede (`stress_memory_test.rs`).

---

## 4. Bateria Atual de Testes no Workspace
 
Métricas verificadas em `2026-10-11`:

- **`ace_core`:** 101 testes unitários/integração + 3 doctests — **100% aprovados** (`0.00s` a `0.69s`).
- **`ace_dom`:** 22 testes de reentrância do tokenizer + dezenas de testes de Tree Builder, Live Ranges, Shadow DOM e Sanitizer — **100% aprovados**.
- **`ace_net`:** 71 testes unitários + testes de Cache RFC 9111, Cookies CHIPS, DoH, descompressão em streaming e teste de estresse de memória de 93s — **100% aprovados**.
- **`ace_style`:** 24 testes automatizados aprovados (0 falhas) em 13 binários, incluindo 12 baterias de estresse pesado (`stress_style_structs_memory_bench`, `stress_mpc_hit_rate_scale`, `stress_invalidation_reactive_benchmark`, `stress_pseudo_elements_hierarchical`, `stress_math_functions_deep_fuzz`, `stress_logical_properties_bidi_matrix`, `stress_cascade_layer_matrix`, `stress_concurrent_style_engine`, `stress_parser_fuzz_adversarial`, `stress_rule_matching_scale`, `stress_style_sharing_cache`, `stress_variables_cycle_explosion`, `unit_values_and_parser_test`) — **100% aprovados** e 0 warnings no Clippy com `-D warnings`.

---

## 5. Como Executar as Suítes

```bash
# Executar todos os testes do workspace (rápido)
cargo test --workspace

# Executar testes de um subsistema específico
cargo test -p ace_core
cargo test -p ace_dom
cargo test -p ace_net
cargo test -p ace_style

# Executar com validação de memory safety sob Miri (para módulos com unsafe)
cargo miri test -p ace_core --test arena_test

# Executar checagem estrita de linters (zero tolerância)
cargo clippy -p ace_style --all-targets --all-features -- -D warnings
```
