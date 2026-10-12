# 🧪 Infraestrutura e Estratégia de Testes do Motor ACE

> **Documento de Arquitetura de Testes e Conformidade**  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.1](../../PLANO.md) — §9)  
> **Última Atualização:** 2026-10-12  
> **Subsistemas Cobertos:** `ace_core`, `ace_dom`, `ace_net`, `ace_style`, `ace_ipc`, `ace_browser`, `ace_test_driver`

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.2.0** | 2026-10-12 | Consolidação de 1.024 Testes & Quitação de Dívidas | • Consolidação da verdade terrestre do workspace: **1.024 testes verdes (100% passing)** em 181 suítes.<br>• Quitação definitiva das dívidas técnicas DV-01 (`ace_ipc`), DV-09 (`tempfile`) e DV-11 (`html5lib-tests`).<br>• Vendorização canônica de `tests1.dat` em `ace_dom/tests/data/html5lib/` com runner oficial de conformidade W3C (7/7 cenários aprovados).<br>• Incorporação dos novos subsistemas operacionais `ace_ipc` (2.22M msg/s) e `ace_browser` (E2E full cycle). | • Registro de 1.024 testes verdes no workspace.<br>• Zero warnings no Clippy com `-D warnings`.<br>• Atualização do Estágio A para 100% concluído. |
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
| **Estágio A** | `html5lib-tests` (arquivos `.dat`) e `css-syntax` | Apenas os Parsers (sem renderização gráfica). Asserções puras sobre tokens e ASTs. | Fase 5 e Fase 6 | 🟢 `css-syntax` e cascata L5/L6 cobertos por 25 testes em `ace_style`; vendorização de `tests1.dat` concluída em `ace_dom/tests/data/html5lib/` com runner oficial `html5lib_compliance_runner.rs` (7/7 cenários aprovados, quitando DV-11) |
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
 
Métricas comprovadas em auditoria forense do código em `2026-10-12` (`cargo test --workspace`):

- **Total Consolidado:** **1.024 testes verdes (100% de sucesso, 0 falhas, 0 ignorados)** distribuídos em **181 suítes/binários de teste**.
- **`ace_core`:** **530 testes** (44 unitários + 483 de integração em 64 suítes + 3 doctests) — **100% aprovados** (Arenas EBR, `LayoutUnit`, `euclid`, `InlineVec`, `TripleBuffer`, Event Loop).
- **`ace_dom`:** **252 testes** em 47 suítes de integração — **100% aprovados** (Tokenizer FSM, Tree Builder 16-AAA, DSD, Live Ranges, Sanitizer e conformidade oficial `html5lib-tests` em `tests1.dat` com 7/7 cenários aprovados).
- **`ace_net`:** **213 testes** (108 unitários + 105 em 28 suítes de integração) — **100% aprovados** (HTTP/1.1, HTTP/2, HTTP/3/QUIC, Cache RFC 9111 L1/L2 WAL, DoH, Cookies CHIPS, scheduler RFC 9218 `CriticalResourceScheduler`, `CssLoaderService` e teste de estresse de 93s).
- **`ace_style`:** **25 testes** em 14 suítes de integração pesadas — **100% aprovados** (Style Structs, MPC, Rayon, Bloom filter, 8 origens de cascata, `@layer`, Writing Modes, `BoxGenerationKind`).
- **`ace_ipc`:** **2 testes** de alta concorrência e estresse — **100% aprovados** (1.000.000 msgs transmitidas a **2.221.596 msg/s** no bounded ring buffer e broadcast multi-tópico com 50.000 msgs por tópico sem perdas).
- **`ace_browser`:** **2 testes** de integração E2E unificada — **100% aprovados** (Ciclo completo URL -> Stream -> DOM -> Style -> RenderTree com telemetria IPC e mutação reativa; estresse massivo de 1.000 nós em 173.81ms e 25 mutações em 202.46ms).

---

## 5. Como Executar as Suítes

```bash
# Executar todos os testes do workspace (1.024 testes verdes)
cargo test --workspace

# Executar testes de subsistemas específicos
cargo test -p ace_core
cargo test -p ace_dom
cargo test -p ace_net
cargo test -p ace_style
cargo test -p ace_ipc
cargo test -p ace_browser

# Executar testes de conformidade canônica W3C/html5lib (DV-11)
cargo test -p ace_dom --test html5lib_compliance_runner

# Executar teste de estresse de alto volume do IPC (>1.5M msg/s)
cargo test -p ace_ipc --test stress_ipc_ring_buffer -- --nocapture

# Executar com validação de memory safety sob Miri (para módulos com unsafe em ace_core)
cargo miri test -p ace_core --test arena_test

# Executar checagem estrita de linters em todo o workspace (zero warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

