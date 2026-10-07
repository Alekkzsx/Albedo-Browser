# 🧱 ace_core — Albedo Core Engine Foundation Crate

> **Camada:** Fundação / Nível 1  
> **Status:** 🟡 **Parcialmente Concluído** (Fase 2 do [PLANO.md](../../PLANO.md) — 101 testes unitários/integração aprovados; pendente IPC in-process em `ace_ipc`)  
> **Documento de Arquitetura Aprofundado:** [`docs/architecture/ace_core.md`](../../docs/architecture/ace_core.md)

O **`ace_core`** é a espinha dorsal de infraestrutura, matemática, alocação de memória e tipos primitivos de todo o **Albedo Core Engine (ACE)**. Ele provê abstrações de altíssimo desempenho, seguras por tipos e projetadas especificamente para os requisitos rigorosos de um navegador web moderno.

---

## 🧭 Inventário dos Módulos

O crate é estruturado em 18 subsistemas de baixo nível, completamente desacoplados de renderização visual ou parsing de tags:

| Módulo / Subdiretório | Responsabilidade e Primitivas Centrais |
| :--- | :--- |
| **`arena`** | Arenas geracionais com *Epoch-Based Reclamation* (EBR), prevenção contra ABA, bump allocators e desalocação determinística sem vazamentos cíclicos. |
| **`collections`** | Estruturas de dados sob medida: `InlineVec` (stack-first com spill para heap), `RingBuffer` de capacidade estática e `FixedBitSet` / `AtomicBitSet`. |
| **`cursor`** | Iteradores eficientes de bytes e caracteres (`ByteCursor`, `CharCursor`) com rastreamento integrado de linha e coluna (`SourceLocation`). |
| **`diagnostics`** | Diagnóstico de integridade, histórico circular de passos (`Breadcrumbs`) e chaves forenses para relatórios de crash (`CrashKeys`). |
| **`events`** & **`observer`** | Barramento de eventos e `ObserverList` reentrante imune a mutações concorrentes durante o despacho. |
| **`event_loop`** | Loop de eventos em conformidade com o WHATWG HTML Living Standard (fontes de tarefas tipadas, microtask checkpoint, isolamento de frames). |
| **`features`** & **`flags`** | Chaveamento lock-free de recursos em tempo de execução (`RuntimeFeatures`), `NodeFlags` e propagação de dirty flags. |
| **`math`** | Geometria 2D/3D tipada via `euclid` (`Point`, `Size`, `Rect2D`, `Transform`), ponto fixo base 60 (`LayoutUnit`) e conversão de cores CSS Color 4 (sRGB, Lab, Oklab, Oklch). |
| **`memory`** | Sistema pub-sub de pressão de memória (`MemoryPressureListener`), controle de limites e rastreadores de densidade de alocação. |
| **`net`** | Decodificação de Data URLs conforme WHATWG Fetch Standard, decodificação percentual e sniffing de tipos MIME por assinatura de bytes. |
| **`security`** | Modelo formal de origens seguras (RFC 6454), Same-Origin Policy (SOP), isolamento por site (eTLD+1), tokens CSPRNG (`UnguessableToken`) e matching CSP3 §6.7.2. |
| **`task`** | Orquestração paralela de tarefas leves com interceptação e contenção de panics (`spawn_safe`). |
| **`telemetry`** | Histogramas de latência atômicos UMA (sub-2ns) com cálculo de percentis (p50/p90/p99) para profiling em tempo real. |
| **`text`** | Construtores de strings de alta densidade (`StringBuilder`), segmentação de texto e mapeamento posicional UTF-8 $\leftrightarrow$ UTF-16. |
| **`version`** | Metadados compilados do motor (`BuildInfo`) e formatação do cabeçalho `User-Agent` RFC 9110. |
| **Raiz (`src/`)** | Primitivas globais: String interning O(1) (`Atom`), otimização de nicho (`Option<NodeId>` em 8B), relógio determinístico (`MockClock`) e hierarquia de erros (`AceError`). |

---

## 🛡️ Políticas de `unsafe` e Segurança de Memória

1. **Minimização Estrita:** O código no `ace_core` privilegia Safe Rust. Todo bloco `unsafe` isolado em estruturas lock-free possui um comentário explicativo `// SAFETY:`.
2. **Validação Miri e Loom:** Primitivas que manipulam ponteiros brutos (como arenas e buffers concorrentes) passam por validação automatizada sob o interpretador **Miri** para garantir ausência de comportamento indefinido (*Undefined Behavior*).

---

## 🧪 Como Executar os Testes

O `ace_core` conta com **101 testes automatizados** e **3 doctests**, cobrindo todas as primitivas com tempos de execução abaixo de 3 segundos:

```bash
# Executar toda a suíte do ace_core
cargo test -p ace_core

# Validar linter corporativo com zero warnings
cargo clippy -p ace_core --all-targets --all-features -- -D warnings
```
