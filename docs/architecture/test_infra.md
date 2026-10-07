# 🧪 Infraestrutura e Estratégia de Testes do Motor ACE

> **Documento de Arquitetura de Testes e Conformidade**  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.0](../../PLANO.md) — §9)  
> **Última Atualização:** 2026-10-07  
> **Subsistemas Cobertos:** `ace_core`, `ace_dom`, `ace_net`, `ace_test_driver`

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
| **Estágio A** | `html5lib-tests` (arquivos `.dat`) e `css-syntax` | Apenas os Parsers (sem renderização gráfica). Asserções puras sobre tokens e ASTs. | Fase 5 e Fase 6 | 🟡 Pendente vendorização dos arquivos `.dat` em `ace_test_driver` |
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

Métricas verificadas em `2026-10-07`:

- **`ace_core`:** 101 testes unitários/integração + 3 doctests — **100% aprovados** (`0.00s` a `0.69s`).
- **`ace_dom`:** 22 testes de reentrância do tokenizer + dezenas de testes de Tree Builder, Live Ranges, Shadow DOM e Sanitizer — **100% aprovados**.
- **`ace_net`:** 71 testes unitários + testes de Cache RFC 9111, Cookies CHIPS, DoH, descompressão em streaming e teste de estresse de memória de 93s — **100% aprovados**.

---

## 5. Como Executar as Suítes

```bash
# Executar todos os testes do workspace (rápido)
cargo test --workspace

# Executar testes de um subsistema específico
cargo test -p ace_core
cargo test -p ace_dom
cargo test -p ace_net

# Executar com validação de memory safety sob Miri (para módulos com unsafe)
cargo miri test -p ace_core --test arena_test
```
