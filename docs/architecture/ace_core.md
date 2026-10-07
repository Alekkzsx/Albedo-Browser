# 🗺️ PLANO MESTRE DEFINITIVO: SUBSISTEMA `ace_core` (Albedo Browser)

> **Versão:** 1.0.0 — *Definitive Engineering Master Plan*  
> **Classificação:** Arquitetura Fundacional do Motor de Renderização (Core Foundation Subsystem)  
> **Subsistema:** `Albedo_Core_Engine/ace_core` (Fase 2 do [PLANO.md](../../PLANO.md))  
> **Status Atual:** 🟡 **Parcialmente Concluído** (101 testes unitários/integração + 3 doctests 100% aprovados; pendente implementação do IPC in-process em `ace_ipc` conforme DV-01)  
> **Padrões Normativos:** WHATWG HTML Living Standard (Event Loop, Task Sources & Microtasks), WHATWG Fetch (Data URLs & MIME Sniffing), RFC 6454 (The Web Origin Concept), W3C CSS Color Module Level 4 & 5 (Color Spaces & Interpolação), W3C Performance Timeline Level 2.

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.0.0** | 2026-10-07 | Auditoria e Consolidação Forense (v7.0) | • Adoção de `euclid` para geometria tipada (ADR-0017).<br>• Padronização de `thiserror` nas APIs de biblioteca (ADR-0016).<br>• Desduplicação de Data URLs unificando em `data_url.rs`.<br>• Identificação da pendência do `ace_ipc` (DV-01). | • Criação deste Plano Mestre Definitivo.<br>• Criação do README do crate.<br>• Suíte de 101 testes unitários consolidada e verde.<br>• 101 linhas de código legado removidas em `utils.rs`. |
| **0.9.0** | 2026-08-17 | Fundação Inicial da Fase 2 | • Criação das arenas geracionais EBR.<br>• Niche Optimization de `Option<NodeId>` em 8 bytes.<br>• Event Loop com filas por `TaskSource`. | • Entrega dos 18 módulos fundacionais com 82 testes iniciais. |

---

## 1. Visão Executiva & Princípios Fundacionais (A Fundação do Motor)

O subsistema **`ace_core`** constitui o solo rochoso sobre o qual todos os demais componentes do **Albedo Core Engine** se apoiam. Em obediência estrita ao **Paradigma Pragmático** da Constituição de Engenharia ([PLANO.md](../../PLANO.md) — §2):
- **O que Orquestramos:** Primitivas fundamentais de concorrência (`crossbeam`, `rayon`, `parking_lot`), tipos geométricos genéricos (`euclid`), logging (`tracing`) e tratamento de erros de biblioteca (`thiserror`).
- **O que Forjamos:** O modelo de alocação de memória geracional imune a vazamentos cíclicos (DOM Arena), o agendador do Event Loop WHATWG, o cálculo subpixel de ponto fixo (`LayoutUnit`), o sistema de string interning O(1) (`Atom`) e o modelo formal de segurança de origens do navegador (SOP).

---

## 2. Arquitetura de Memória e Concorrência

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       ARQUITETURA DE MEMÓRIA ace_core                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────┐  ┌───────────────────────┐  ┌────────────────┐  │
│  │   Generational Arena   │  │   TripleBuffer (Lock) │  │  InlineVec<T>  │  │
│  │   (Epoch-Based Recl.)  │  │   (Zero-Copy Atômico) │  │ (Stack-First)  │  │
│  │   Índices: NodeId 8B   │  │   Render Producer     │  │ Spill Dinâmico │  │
│  │   Anti-ABA Versioning  │  │   Display Consumer    │  │ Sem Alocação   │  │
│  └────────────────────────┘  └───────────────────────┘  └────────────────┘  │
│               │                          │                       │          │
│               v                          v                       v          │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │         Estruturas Lock-Free, Atômicos e Alocação O(1)                │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Arenas Geracionais com EBR (Epoch-Based Reclamation)
O DOM é um grafo complexo com referências bidirecionais e cíclicas (pais, filhos, irmãos). Utilizar `Rc<RefCell<Node>>` causaria overhead desastroso de contadores de referência e vazamentos crônicos de memória.
- **Solução ace_core:** Uma arena baseada em vetores contíguos indexados por identificadores geracionais de 64 bits (`NodeId`), onde os 32 bits superiores codificam o índice e os 32 bits inferiores codificam a geração.
- **Eliminação de ABA:** Tentativas de acessar nós reciclados com identificadores antigos resultam em erro determinístico sem corrupção de memória.

### 2.2 Niche Optimization (`Option<NodeId>`)
Navegadores manipulam milhões de nós. Cada byte economizado em ponteiros representa megabytes poupados no heap.
- Utilizando `NonZeroU64`, `Option<NodeId>` possui exatamente **8 bytes** de memória (o valor zero atua como sentinela de `None`), eliminando o byte de tag de discriminação de enums do Rust.

### 2.3 TripleBuffer Atômico Lock-Free
Para desacoplar a thread de renderização da thread de apresentação na tela (compositor), o `ace_core` implementa o padrão *Triple Buffering*:
- A thread produtora e a thread consumidora trocam índices de buffers através de operações atômicas `compare_exchange`, garantindo taxa de atualização suave (60–120 FPS) sem lock contention.

---

## 3. Módulos e Subsistemas Centrais

### 3.1 Event Loop WHATWG (`event_loop/`)
Implementação estrita do ciclo de vida da página conforme o WHATWG HTML Living Standard:
- **Task Sources:** Filas segregadas para I/O de rede, eventos de usuário, mutações do DOM e timers.
- **Microtask Queue:** Esvaziamento determinístico em pontos de verificação (*Microtask Checkpoint*) antes de liberar o controle para tarefas normais ou renderização.
- **Anti-Starvation:** Política de intercalação que impede que rajadas contínuas de microtarefas congelem as tarefas de renderização.

### 3.2 Matemática, Geometria e Cor (`math/`)
- **`LayoutUnit`:** Aritmética de ponto fixo base 60 (onde 1px = 60 unidades) com operações saturadas, eliminando erros de arredondamento de ponto flutuante em subdivisões de subpixel.
- **CSS Color 4 & 5:** Conversores e interpoladores para espaços de cor perceptualmente uniformes: sRGB, CIELAB, Oklab e Oklch com adaptação cromática de Bradford D65 $\leftrightarrow$ D50.

### 3.3 Modelo de Origens e Segurança Web (`security/`)
- **Same-Origin Policy:** Parsing canônico de esquemas, portas e hosts segundo a RFC 6454.
- **Isolamento de Sites:** Resolução de domínio registrável (*eTLD+1*) servindo de base para o futuro isolamento de processos (*Site Isolation*).
- **Defesas Temporais:** Quantização de timestamp para mitigar vazamentos de informação baseados em canais laterais de temporização (ataques estilo Spectre).

---

## 4. Estado Atual e Dívidas Verificadas

| Componente | Estado Auditado | Notas de Engenharia |
| :--- | :---: | :--- |
| **Primitivas do `ace_core`** | ✅ Concluído | 101 testes unitários passando, 0 falhas, 0 warnings. |
| **Desduplicação de Data URLs** | ✅ Concluído | Unificado com a especificação WHATWG Fetch em `data_url.rs`. |
| **Comunicação IPC (`ace_ipc`)** | 🟡 Pendente | O crate `ace_ipc` é atualmente um stub. Exige implementação dos canais in-process e das mensagens `Navigate`, `RenderFrame` e `InputEvent` para cumprimento integral do DoD da Fase 2. |

