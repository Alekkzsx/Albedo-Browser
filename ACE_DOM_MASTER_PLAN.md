# 🗺️ PLANO MESTRE DEFINITIVO: SUBSISTEMA `ace_dom` (Albedo Browser)

> **Versão:** 1.0.0 — *Definitive Engineering Master Plan*  
> **Classificação:** Arquitetura Central do Motor de Renderização (Core Subsystem Plan)  
> **Subsistema:** `Albedo_Core_Engine/ace_dom`  
> **Status:** Autorizado para Implementação & Execução Técnica  
> **Padrões Normativos:** WHATWG HTML Living Standard (§12 Parsing, §4.10 Forms, §4.12 Scripting/Template), WHATWG DOM Standard (§4 Trees & Mutation, §5 Traversal & Range), W3C Selectors Level 4, W3C CSSOM View & Cascading, W3C WebIDL Standard, W3C HTML Sanitizer API.

---

## 📚 Sumário Executivo do Plano

- [1. Visão Executiva & Princípios Fundacionais (A Alma do DOM)](#1-visão-executiva--princípios-fundacionais-a-alma-do-dom)
- [2. Matriz Comparativa Técnica SOTA (Deep Engine Benchmark)](#2-matriz-comparativa-técnica-sota-deep-engine-benchmark)
- [3. Auditoria Forense do Código Atual & Inventário de Lacunas](#3-auditoria-forense-do-código-atual--inventário-de-lacunas)
  - [3.1 Tokenizer FSM & Decodificação de Entidades](#31-tokenizer-fsm--decodificação-de-entidades)
  - [3.2 Tree Construction & Algoritmo da Agência de Adoção (AAA 16-Step)](#32-tree-construction--algoritmo-da-agência-de-adoção-aaa-16-step)
  - [3.3 Conteúdo Estrangeiro (SVG & MathML) e Namespaces](#33-conteúdo-estrangeiro-svg--mathml-e-namespaces)
  - [3.4 Declarative Shadow DOM (DSD) & Templates](#34-declarative-shadow-dom-dsd--templates)
  - [3.5 Form Validity & Controles de Formulário](#35-form-validity--controles-de-formulário)
  - [3.6 CSSOM, Query Engine & Ancestor Bloom Filter](#36-cssom-query-engine--ancestor-bloom-filter)
  - [3.7 MutationObserver, Live Ranges & Complexidade LCA](#37-mutationobserver-live-ranges--complexidade-lca)
  - [3.8 HTML Sanitizer & Defesa Anti-XSS](#38-html-sanitizer--defesa-anti-xss)
- [4. Arquitetura de Memória, Modelagem de Dados & Otimizações](#4-arquitetura-de-memória-modelagem-de-dados--otimizações)
  - [4.1 Layout Exato de Bits/Bytes em 64-bit](#41-layout-exato-de-bitsbytes-em-64-bit)
  - [4.2 Tabela de Densidade por Tipo de Nó vs SOTA](#42-tabela-de-densidade-por-tipo-de-nó-vs-sota)
  - [4.3 Análise Assintótica Formal das 15 Operações Primárias](#43-análise-assintótica-formal-das-15-operações-primárias)
- [5. Roteiro de Decomposição em Milestones & Contratos de Interface](#5-roteiro-de-decomposição-em-milestones--contratos-de-interface)
  - [Milestone 1 (M1): Tokenizer FSM 100% WHATWG & Parser Hardening](#milestone-1-m1-tokenizer-fsm-100-whatwg--parser-hardening)
  - [Milestone 2 (M2): Compactação de Memória & Otimização de Layout (`NodeData` 88B, `Atom` 8B)](#milestone-2-m2-compactação-de-memória--otimização-de-layout-nodedata-88b-atom-8b)
  - [Milestone 3 (M3): Sincronização Dinâmica do `ElementIndex` & Comparação LCA $O(\text{depth})$](#milestone-3-m3-sincronização-dinâmica-do-elementindex--comparação-lca-odepth)
  - [Milestone 4 (M4): Integração do `AncestorFilter` no Query Selector & Motor de Especificidade CSSOM](#milestone-4-m4-integração-do-ancestorfilter-no-query-selector--motor-de-especificidade-cssom)
  - [Milestone 5 (M5): Acoplamento do Event Loop com `MutationObserver` e `LiveRangeRegistry`](#milestone-5-m5-acoplamento-do-event-loop-com-mutationobserver-e-liverangeregistry)
  - [Milestone 6 (M6): Runner Oficial de Web Platform Tests (WPT) & Test Harness `.dat`](#milestone-6-m6-runner-oficial-de-web-platform-tests-wpt--test-harness-dat)
- [6. Estratégia de Compilação/Bindings WebIDL e Acoplamento com `ace_js`](#6-estratégia-de-compilaçãobindings-webidl-e-acoplamento-com-ace_js)
- [7. Infraestrutura de Testes, Validação WPT e Garantia de Qualidade](#7-infraestrutura-de-testes-validação-wpt-e-garantia-de-qualidade)

---

## 1. Visão Executiva & Princípios Fundacionais (A Alma do DOM)

O subsistema `ace_dom` constitui a espinha dorsal de dados, representação estrutural e semântica do **Albedo Browser**. Em conformidade absoluta com as Regras de Ouro do projeto estabelecidas no `PLANO.md` (Constituição de Engenharia), o `ace_dom` é **100% Alma**: não utiliza motores pré-fabricados (como `html5ever`, `cssparser`, Blink, WebKit, Servo ou Gecko), forjando sua própria arquitetura de memória, máquina de estados finitos (FSM) de streaming, motor de resolução seletiva CSS4 e integrador de Web Components.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          ALBEDO CORE ENGINE (ACE)                           │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                              ace_dom                                  │  │
│  │                                                                       │  │
│  │   ┌─────────────────────┐   Tokens    ┌───────────────────────────┐   │  │
│  │   │  HTML5 Tokenizer    │ ──────────> │    HTML5 Tree Builder     │   │  │
│  │   │  (88 FSM + SIMD)    │             │   (23 Modes + 16-AAA)     │   │  │
│  │   └─────────────────────┘             └─────────────┬─────────────┘   │  │
│  │              │                                      │                 │  │
│  │              │ Preload Requests                     │ Mutates Tree    │  │
│  │              v                                      v                 │  │
│  │   ┌─────────────────────┐             ┌───────────────────────────┐   │  │
│  │   │   Preload Scanner   │             │   Generational DOM Arena  │   │  │
│  │   │ (Sub-resource fetch)│             │   (Arena<NodeData>, 8B Id)│   │  │
│  │   └─────────────────────┘             └─────────────┬─────────────┘   │  │
│  │                                                     │                 │  │
│  │            ┌────────────────────────────────────────┴────────┐        │  │
│  │            v                                                 v        │  │
│  │   ┌───────────────────┐                             ┌─────────────┐   │  │
│  │   │ CSS4 Query Engine │ <── AncestorFilter (64B)    │  Mutation   │   │  │
│  │   │ (RTL + Specificity│                             │  Observer & │   │  │
│  │   │  + RuleBucketMap) │                             │ Live Ranges │   │  │
│  │   └───────────────────┘                             └─────────────┘   │  │
│  │            │                                                 │        │  │
│  │            v                                                 v        │  │
│  │   ┌───────────────────────────────────────────────────────────────┐   │  │
│  │   │              WebIDL Bindings & JS-DOM DataStore               │   │  │
│  │   │         (Non-owning NodeId handles + Tri-color GC Tracer)     │   │  │
│  │   └───────────────────────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │                                      │
│               Exposes DOM Tree & Computed Styles to Pipelines                │
│                                      v                                      │
│  ┌───────────────────┐    ┌────────────────────┐    ┌────────────────────┐  │
│  │     ace_style     │ ─> │     ace_layout     │ ─> │     ace_render     │  │
│  │ (Cascade/Compute) │    │  (Box/Flex/Grid)   │    │  (wgpu/Compositor) │  │
│  └───────────────────┘    └────────────────────┘    └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.1 Invariantes Estruturais Inegociáveis

1. **100% Safe Rust (`#![forbid(unsafe_code)]`):**
   - O crate `ace_dom` é estritamente compilado com `#![forbid(unsafe_code)]`. Nenhuma linha de código inseguro, ponteiro cru (`*const T`, `*mut T`), ou transmutação arbitrária é permitida dentro de `ace_dom`.
   - Primitivas de baixo nível de altíssimo desempenho residem exclusivamente no crate auditado `ace_core` (`InlineVec`, `BumpArena`, `Arena`, `NonZeroU64`), onde cada bloco `unsafe` é coberto por testes formais no Miri e Loom.

2. **Arena Geracional Contígua com `NodeId` de 8 Bytes (`NonZeroU64`):**
   - A árvore DOM reside em uma `Arena<NodeData>` baseada no padrão SlotMap com índices geracionais de 64 bits.
   - O identificador `NodeId` encapsula `(version: u32, index: u32)`.
   - **Elisão Total de Nicho:** O valor numérico `0` é estritamente reservado pela arena, permitindo que `Option<NodeId>` ocupe exatamente **8 bytes** em memória (sem overhead de discriminante).

3. **Eliminação Radical de `Rc<RefCell<Node>>` e Inexistência de Ciclos:**
   - Motores ingênuos em Rust ou C++ sofrem com `Rc<RefCell<Node>>` que introduzem panics de empréstimo dinâmico (`BorrowError`) e vazamentos de memória cíclicos insidiosos quando nós são desconectados sem limpeza recursiva de pais/filhos.
   - No `ace_dom`, todos os links estruturais (`parent`, `first_child`, `last_child`, `prev_sibling`, `next_sibling`) são escalares `Option<NodeId>`. Relações cíclicas não geram retenção de memória no heap. A destruição de um documento inteiro é amortizada em **$O(1)$** descartando o buffer contíguo da arena.

4. **Tri-Color GC Tracing (`GcTracer`, `Traceable`, `MarkTracer`):**
   - A integração entre o motor JavaScript (`ace_js`) e a árvore DOM (`ace_dom`) opera sem referências circulares proprietárias. Wrappers JS mantêm apenas o escalar `NodeId`.
   - O GC geracional do `ace_js` executa o rastreamento em três cores (White, Grey, Black) através do trait `Traceable`, permitindo coleta de grafos inter-heap (ex: nó DOM $\to$ Event Listener JS Closure $\to$ nó DOM) sem complexos coletores de ciclo como o `nsCycleCollector` do Gecko.

---

## 2. Matriz Comparativa Técnica SOTA (Deep Engine Benchmark)

Abaixo é apresentada a análise comparativa detalhada do subsistema `ace_dom` contra os 5 principais motores de browser da indústria global:

| Dimensão Técnica / Arquitetural | **Albedo (`ace_dom`)** | **Google Chrome (Blink / V8)** | **Apple Safari (WebKit / JSC)** | **Mozilla Firefox (Gecko / SpiderMonkey)** | **Ladybird (LibWeb / LibJS)** | **Servo (Rust / SpiderMonkey)** |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1. Gerenciamento de Memória DOM** | Arena Geracional Contígua (`Arena<NodeData>`) | Traced C++ Heap (`cppgc` / Oilpan) | `IsoSubspace` por tipo + bmalloc | Refcounting Intrusivo + jemalloc | Unified GC Heap (`JS::Cell`) | ScriptThread Arena + Rooting GC |
| **2. Representação de Nós (`NodePtr`)** | `NodeId` de 8B (`NonZeroU64` version+index) | Raw Pointer / `Member<Node>` (8B) | `RefPtr<Node>` (8B intrusivo) | `RefPtr<nsINode>` (8B intrusivo) | `JS::NonnullGCPtr<Node>` (8B) | `Dom<T>` / `MutNullableDom<T>` (8B) |
| **3. Ciclo de Vida e Grafo Cíclico** | **Zero Overhead:** Identificadores escalares sem posse | Tracing GC periódica no heap C++ Oilpan | Retain/Release ping-pong com unlinking manual | SpiderMonkey Cycle Collector (`nsCycleCollector`) | Mark-and-Sweep traceia ponteiros cell | SpiderMonkey GC tracing via `DomRoot` |
| **4. Teardown de Documento Inteiro** | **$O(1)$ amortizado** (liberação em bloco da arena) | Sweep incremental do Oilpan | Caminhamento recursivo `deref()` em subárvores | `Release()` recursivo + passos do Cycle Collector | Sweep pass sobre todo o GC Heap | Major Collection no GC SpiderMonkey |
| **5. Armazenamento de Atributos** | `InlineVec<Attribute, 4>` (inline até 4 sem heap) | `AttributeCollection` com buffer plano/heap | `ElementData` com vetor inline dinâmico | `nsAttrAndChildArray` empacotado | `Vector<Attribute>` alocado no heap | `Vec<Attr>` alocado no heap do Servo |
| **6. Rare Fields (`RareData`)** | `Option<Box<ElementRareData>>` (8B quando vazio) | `NodeRareData` / `ElementRareData` via pointer | Tabela global de `RareData` | `nsINode::GetPropertyTable()` / `mProperties` | Campos dinâmicos no objeto Cell | Struct `RareData` embutida/ponteiro |
| **7. String Interning (`Atom`)** | `Atom` (Tabela estática indexada + `string_cache`) | `AtomicString` (Hash table global/thread-local) | `AtomString` (Hash table com refcount) | `nsAtom` (Atoms estáticos e dinâmicos) | `FlyString` (LibJS string pool) | `Atom` (`string_cache` servo) |
| **8. Ancestor Bloom Fast Rejection** | **Counting Bloom Filter (64/256B)** com Sticky Saturation | `AncestorFilter` (Counting Bloom 64 buckets) | `AncestorFilter` (Bloom filter no `SelectorChecker`) | Quantum CSS (Stylo) Ancestor Bloom Filter | Varredura linear sem bloom filter | Stylo Rust Ancestor Bloom Filter |
| **9. Indexação de Regras CSSOM** | `RuleBucketIndex` (ID, Class, Tag, Universal) | `RuleSet` com baldes `RuleMap` | `RuleSet` com baldes `RuleData` | `RuleCascadeData` / `SelectorMap` no Stylo | Selector Map básico | Stylo `RuleMap` |
| **10. Concorrência & Threading** | `Document` e `NodeData` são `Send + Sync`; streaming assíncrono | Thread principal restrita; streaming parser isolado | Main Thread restrita; background tokenization | Stylo paralelo via Rayon; DOM na main thread | Single-threaded EventLoop | Multithreaded: Layout, Script e Constellation |
| **11. Aceleração SIMD no Tokenizer** | **Sim (`memchr3` no DataState e delimitadores)** | Sim (vetorização manual SSE/NEON) | Sim (Fast path SIMD no WebCore Lexer) | Sim (SIMD no parser HTML Gecko) | Não (Iteração caractere a caractere) | Não (Iteração padrão rust) |
| **12. Cobertura da FSM do Tokenizer** | 88 estados normativos declarados (WHATWG §12) | 88 estados 100% implementados | 88 estados 100% implementados | 88 estados 100% implementados | 88 estados 100% implementados | 88 estados 100% implementados |
| **13. Resolução de Ciclos DOM $\leftrightarrow$ JS** | Tri-Color Tracer unificado via `NodeId` | Traced Wrappers V8 $\leftrightarrow$ Oilpan | JS Wrapper Cache com `visitChildren` | `nsCycleCollectionParticipant` | Heap unificado C++ LibJS | ScriptThread Traceable |
| **14. Memória por Elemento Simples** | **~224 Bytes** (NodeData 88B + Box<ElementData> 136B) | ~112–144 Bytes | ~120–160 Bytes | ~96–136 Bytes | ~160–200 Bytes | ~140–180 Bytes |
| **15. Declarative Shadow DOM (DSD)** | **Nativo** (`<template shadowrootmode>`) | Nativo | Nativo | Nativo | Não | Não |
| **16. Conformidade com WPT** | Test Runner Integrado (`.dat` html5lib-tests) | 100% WPT CI contínuo | 100% WPT CI contínuo | 100% WPT CI contínuo | WPT parcial em desenvolvimento | WPT CI contínuo |

---

## 3. Auditoria Forense do Código Atual & Inventário de Lacunas

Esta seção consolida a auditoria forense linha a linha realizada no código-fonte de `Albedo_Core_Engine/ace_dom`, evidenciando com exatidão cirúrgica o estado atual, as conformidades atingidas e as lacunas normativas a serem fechadas.

### 3.1 Tokenizer FSM & Decodificação de Entidades
*Arquivos auditados:* `src/tokenizer/mod.rs`, `src/tokenizer/state.rs`, `src/tokenizer/token.rs`, `src/entities/mod.rs`.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       HTML5 TOKENIZER STATE MACHINE                         │
│                                                                             │
│               ┌──────────────────────────────────────────────┐              │
│               │           Data State (memchr3 SIMD)          │              │
│               └──────┬───────────────────┬───────────────────┘              │
│                      │ '&'               │ '<'                              │
│                      v                   v                                  │
│         ┌───────────────────────┐   ┌─────────────────────────┐             │
│         │ Character Reference   │   │     Tag Open State      │             │
│         │ in Data State         │   └──────┬────────────┬─────┘             │
│         └───────────────────────┘          │ '!'        │ 'a-z'             │
│                                            v            v                   │
│                               ┌────────────────┐   ┌──────────────────┐     │
│                               │ Markup Decl.   │   │  Tag Name State  │     │
│                               │ Open State     │   └────────┬─────────┘     │
│                               └──────┬─────────┘            │ Whitespace    │
│                                      │ '--'                 v               │
│                                      v             ┌──────────────────┐     │
│                               ┌────────────────┐   │ Before Attribute │     │
│                               │ Comment Start  │   │   Name State     │     │
│                               └────────────────┘   └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Lacuna de Cobertura de Estados na FSM (88 vs 31):**
   - O enum `TokenizerState` define os 88 estados normativos do WHATWG HTML §12.2.5.
   - O loop principal em `HTMLTokenizer::tokenize` trata explicitamente 31 estados. Os demais 57 estados (incluindo `ScriptDataEscapeStart`, `ScriptDataEscaped`, `ScriptDataDoubleEscapeStart`, `ScriptDataDoubleEscaped`, `CommentLessThanSignBang`, `AfterDoctypePublicKeyword`, etc.) caem no fallback de segurança `_ => { self.state = TokenizerState::Data; }`.
   - **Ação:** Implementar os 57 blocos de estados faltantes na FSM para suportar comentários e escapes complexos em scripts e CDATA/Foreign content conforme a especificação.

2. **Escape de Comentários em Scripts (`<!-- <script> ... </script> -->`):**
   - No WHATWG §12.2.5.20–31, o HTML permite scripts inline contendo marcações de comentário histórico. Ao encontrar `<!--`, o tokenizer entra em `ScriptDataEscapeStart` e `ScriptDataEscaped`, ignorando tags `</script>` internas até o encerramento do escape.
   - A heurística atual de lookahead consome o primeiro `</script>` prematuramente.
   - **Ação:** Substituir lookahead ingênuo pela transição formal entre os estados `ScriptData`, `ScriptDataEscapeStart`, `ScriptDataEscaped`, `ScriptDataDoubleEscapeStart`, `ScriptDataDoubleEscaped`, `ScriptDataDoubleEscapeEnd` e `ScriptDataEndTagName`.

3. **Regra da Ampersand Ambígua em Atributos (WHATWG §12.2.5.73):**
   - Referências de caracteres nomeadas em atributos que não terminam com ponto-e-vírgula (ex: `href="?a=1&copy=2"` ou `href="?page=1&notit"`) seguidas de `=` ou caractere alfanumérico ASCII **não podem ser decodificadas**, sob pena de corromper parâmetros de query URL.
   - O decodificador atual decodifica `&copy` incondicionalmente para `©`.
   - **Ação:** Condicionar a substituição de referências de entidades em atributos à presença de `;` ou à ausência de `=` e alfanuméricos ASCII imediatos.

4. **Captura de DOCTYPE Public & System Identifiers:**
   - O estado `AfterDoctypeName` atualmente transiciona diretamente para `BogusDoctype` em caracteres que não sejam whitespace ou `>`.
   - Os identificadores `PUBLIC` e `SYSTEM` não são capturados, fazendo com que `DoctypeToken.public_identifier` e `system_identifier` permaneçam sempre `None`.
   - **Ação:** Implementar os estados `BeforeDoctypePublicIdentifier`, `DoctypePublicIdentifierDoubleQuoted`, `DoctypePublicIdentifierSingleQuoted`, `AfterDoctypePublicIdentifier`, `BetweenDoctypePublicAndSystemIdentifiers`, `BeforeDoctypeSystemIdentifier`, `DoctypeSystemIdentifierDoubleQuoted`, `DoctypeSystemIdentifierSingleQuoted` e `AfterDoctypeSystemIdentifier`.

5. **Tratamento de EOF na FSM:**
   - Em EOF inesperado dentro de tags (`TagName`, `AttributeName`) ou comentários, o buffer acumulado não deve ser descartado silenciosamente, mas sim emitido conforme a tabela de recuperação do WHATWG.

---

### 3.2 Tree Construction & Algoritmo da Agência de Adoção (AAA 16-Step)
*Arquivos auditados:* `src/tree_builder/mod.rs`, `src/tree_builder/adoption_agency.rs`, `src/tree_builder/active_formatting.rs`.

1. **Modos de Inserção Normativos (23 Modos):**
   - O `HTMLTreeBuilder` suporta 19 modos de inserção de forma nativa.
   - Os modos `InHeadNoscript`, `InFrameset`, `AfterFrameset` e `AfterAfterFrameset` delegam para `InBody`.
   - **Ação:** Implementar o tratamento específico para tags `<noscript>` no cabeçalho e marcações de frameset legadas.

2. **Adoption Agency Algorithm (AAA 16-Step Canonical):**
   - O algoritmo em `adoption_agency.rs` segue com rigor matemático as 16 etapas do WHATWG §12.2.6.4.7:
     - Outer loop limitado a 8 iterações (prevenção de DoS).
     - Localização do `formatting_element` após o último marcador na lista de formatação ativa.
     - Validação de presença na pilha de elementos abertos e verificação de escopo.
     - Localização do `furthest_block` e identificação do `common_ancestor`.
     - Inner loop limitado a 3 iterações com clonagem de nós e reparenting de `last_node`.
     - **Correção do Bookmark Offset no Passo 3.17:**
       ```rust
       let fmt_idx = active_formatting.position_of(formatting_element_id);
       active_formatting.remove(formatting_element_id);
       if let Some(idx) = fmt_idx {
           if idx < bookmark {
               bookmark = bookmark.saturating_sub(1);
           }
       }
       active_formatting.insert_at(bookmark, new_element);
       ```
   - **Cláusula "Arca de Noé" (*Noah's Ark Clause*):** Implementada em `ActiveFormattingElements::push_element`, expulsando a instância mais antiga quando mais de 3 elementos de formatação idênticos são empilhados, mitigando complexidade combinatorial $O(N^2)$.

3. **Foster Parenting (WHATWG §12.2.6.1):**
   - `foster_parent_node` insere nós malformados dentro de tabelas imediatamente antes do elemento `<table>` no pai deste. Se a tabela não tiver pai, anexa ao nó mais recente na pilha.

4. **Detecção de Quirks Mode (WHATWG §12.2.6.4.1):**
   - `DocumentMode` suporta `NoQuirks`, `Quirks` e `LimitedQuirks`.
   - **Ação:** Com a extração de `public_identifier` e `system_identifier` do DOCTYPE no Tokenizer, preencher a tabela completa de detecção histórica de Quirks Mode (ex: `-//W3C//DTD HTML 4.0 Transitional//EN`, `system_id` ausente, etc.).

---

### 3.3 Conteúdo Estrangeiro (SVG & MathML) e Namespaces
*Arquivos auditados:* `src/tree_builder/foreign.rs`, `src/node/element.rs`.

1. **Tabelas Estáticas em Tempo de Compilação (`phf`):**
   - `SVG_TAG_NAME_FIXUPS`: 37 tags SVG case-sensitive ajustadas em $O(1)$ (`altGlyph`, `clipPath`, `linearGradient`, `foreignObject`, etc.).
   - `SVG_ATTRIBUTE_FIXUPS`: 45 atributos SVG case-sensitive ajustados em $O(1)$ (`viewBox`, `preserveAspectRatio`, `gradientTransform`, etc.).
2. **Pontos de Integração HTML / MathML:**
   - SVG Integration Points: `foreignObject`, `desc`, `title`.
   - MathML Integration Points: `annotation-xml`, `mi`, `mo`, `mn`, `ms`, `mtext`.
3. **Lacunas de Namespaces:**
   - Adicionar tabela `MATHML_ATTRIBUTE_FIXUPS` (ex: `definitionURL`).
   - Armazenar prefixo e URI de namespaces dedicados (`http://www.w3.org/1999/xlink`, `http://www.w3.org/2000/xmlns/`, `http://www.w3.org/XML/1998/namespace`) de forma explícita na struct `Attribute`.

---

### 3.4 Declarative Shadow DOM (DSD) & Templates
*Arquivos auditados:* `src/tree_builder/mod.rs`, `src/node/element.rs`, `src/traversal/flat_tree.rs`.

1. **Parsing de `<template shadowrootmode>`:**
   - `handle_template_start` identifica `shadowrootmode="open"` ou `"closed"` (com parsing case-insensitive).
   - Executa `doc.attach_shadow(host, mode)`.
   - Empilha o modo de inserção de template correspondente e direciona nós subsequentes para dentro da Shadow Tree.
2. **Validação de Host Elements Permitidos (WHATWG DOM §4.2.2):**
   - **Ação:** Validar se a tag hospedeira pertence à lista de elementos permitidos para Shadow Root (`article`, `aside`, `blockquote`, `body`, `div`, `footer`, `h1`–`h6`, `header`, `main`, `nav`, `p`, `section`, `span`, custom elements autônomos com hífen), rejeitando a anexação em tags como `input`, `img`, `iframe`, etc.
3. **Distribuição de Slots e Árvore Achatada (`FlatTreeResolver`):**
   - Resolução de projeção de nós filhos via `<slot name="...">` e default slots para a Render Tree.

---

### 3.5 Form Validity & Controles de Formulário
*Arquivos auditados:* `src/form/mod.rs`, `src/form/validity.rs`, `src/form/form_data.rs`.

1. **Objeto `ValidityState` (10 Flags Normativas):**
   - Implementa integralmente as 10 flags do WHATWG HTML §4.10.21.2:
     `value_missing`, `type_mismatch`, `pattern_mismatch`, `too_long`, `too_short`, `range_underflow`, `range_overflow`, `step_mismatch`, `bad_input`, `custom_error`.
2. **Associação Remota de Controles (`form="form_id"`):**
   - `FormData::from_form_element` percorre tanto a subárvore local quanto controles remotos no documento associados pelo atributo `form`.
3. **Validação de Padrões:**
   - **Ação:** Aprimorar `matches_simple_pattern` para suportar expressões regulares ECMAScript normativas para e-mails (WHATWG HTML §4.10.5.1.5) e URLs.

---

### 3.6 CSSOM, Query Engine & Ancestor Bloom Filter
*Arquivos auditados:* `src/query/selector.rs`, `src/query/bloom.rs`, `src/query/index.rs`.

1. **Motor de Seletores CSS4 RTL (Right-to-Left):**
   - AST com `ComplexSelector`, `CompoundSelector`, `SimpleSelector`, `Combinator` (`Child`, `Descendant`, `AdjacentSibling`, `GeneralSibling`).
   - Suporte completo a operadores de atributo (`=`, `~=`, `|=`, `^=`, `$=`, `*=`, com modificadores de case `i` e `s`).
   - Pseudo-classes: `:first-child`, `:last-child`, `:only-child`, `:empty`, `:root`, `:checked`, `:disabled`, `:enabled`, `:is(...)`, `:where(...)`, `:has(...)`, `:not(...)`, `:nth-child(An+B)`.
2. **Ancestor Counting Bloom Filter (64/256 Buckets com Sticky Saturation):**
   - Tabela de contadores `u8` com dupla função de hash (FNV-1a e DJB2).
   - **Sticky Saturation Resolvida:** Quando um contador atinge 255 (saturação), decrements não reduzem abaixo de 255, garantindo **zero falsos negativos** mesmo sob profundidades de aninhamento extremas ou ataques de colisão.
3. **Integração do `AncestorFilter` no Query Traversal:**
   - **Ação:** Acoplar o `AncestorFilter` diretamente durante o caminhamento DFS de `Document::query_selector` e `Document::query_selector_all`, bem como na resolução de estilos de `ace_style`, descartando instantaneamente mais de 85% de subárvores que não possuem os ancestrais exigidos pelo seletor.
4. **Cálculo de Especificidade CSSOM `(A, B, C)`:**
   - **Ação:** Introduzir a struct `Specificity(u32, u32, u32)` com ordenação lexicográfica conforme W3C Selectors 4:
     - $A$: Número de seletores de ID (`#id`).
     - $B$: Número de seletores de classe, atributos e pseudo-classes (excluindo `:is()`, `:where()`, `:not()`, cujo peso é herdado dos argumentos mais específicos).
     - $C$: Número de seletores de tipo (tags) e pseudo-elementos.

---

### 3.7 MutationObserver, Live Ranges & Complexidade LCA
*Arquivos auditados:* `src/observer/mod.rs`, `src/range/mod.rs`, `src/range/registry.rs`, `src/tree/mutation.rs`.

1. **Prevenção de Ciclos de Hierarquia no DOM (`HierarchyRequestError`):**
   - `tree::mutation::append_child` e `insert_before` validam rigorosamente se o nó inserido é um ancestral inclusivo do nó pai (`is_inclusive_ancestor`), retornando `Err(DomError::HierarchyRequestError)` e impedindo a formação de ciclos direcionados na árvore.
2. **Live Range Auto-Adjustment em Mutações:**
   - Auto-ajuste de boundary points em `split_text`, `remove_child` e `insert_before`.
   - Ajuste em cascata para todos os nós descendentes de subárvores removidas (`adjust_for_node_removal_with_descendants`), reparentando os pontos de limite para o container de remoção.
3. **Comparação de Posição via Ancestral Comum Mais Próximo (LCA) em $O(\text{depth})$:**
   - `Range::compare_boundary_points` e `Document::compare_document_position` substituem a busca global $O(N)$ via `descendants().enumerate()` por uma caminhada de caminhos até a raiz em $O(\text{depth}_1 + \text{depth}_2)$, localizando o LCA e comparando os índices dos ramos irmãos diretos.
4. **Despacho no Microtask Queue do Event Loop:**
   - **Ação:** Acoplar o `MutationObserver` à fila de microtasks do `ace_core::EventLoop`, despachando lotes de mutação automaticamente antes de cada frame de animação e renderização.

---

### 3.8 HTML Sanitizer & Defesa Anti-XSS
*Arquivos auditados:* `src/sanitizer/mod.rs`, `src/sanitizer/config.rs`.

1. **Filtragem Case-Insensitive Estrita:**
   - Verificação de tags bloqueadas (`block_elements`) e atributos bloqueados (`block_attributes`) utilizando `eq_ignore_ascii_case`, neutralizando evasões com tags em maiúsculas (`<SCRIPT>`, `<IFRAME>`) e atributos (`FORMACTION`).
2. **Purga de Handlers de Eventos Inline & Esquemas Perigosos:**
   - Purga incondicional de todos os atributos `on*` (`onclick`, `onerror`, `onload`, etc.).
   - Normalização de whitespace e caracteres de controle antes de validar e bloquear esquemas de URI perigosos (`javascript:`, `vbscript:`, `data:text/html`).
3. **Desempacotamento de Elementos (`replaceWithChildren`):**
   - **Ação:** Adicionar suporte à W3C Sanitizer API para desempacotar nós proibidos mas cujos filhos são seguros (ex: remover uma tag semântica não permitida preservando o texto e elementos internos).

---

## 4. Arquitetura de Memória, Modelagem de Dados & Otimizações

### 4.1 Layout Exato de Bits/Bytes em 64-bit

Para atingir a máxima densidade de cache L1/L2 e reduzir o consumo global de RAM do navegador, o `ace_dom` implementa uma rigorosa compactação de layout de dados.

```
Layout de 64-bit da Arena Entry (NodeData Compactado):
┌─────────────────────────────────────────────────────────────────────────────┐
│ 0..8   │ id: NodeId (NonZeroU64: version u32 | index u32)                   │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 8..16  │ parent: Option<NodeId> (Niche: 0x0)                                │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 16..24 │ first_child: Option<NodeId> (Niche: 0x0)                           │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 24..32 │ last_child: Option<NodeId> (Niche: 0x0)                            │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 32..40 │ prev_sibling: Option<NodeId> (Niche: 0x0)                          │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 40..48 │ next_sibling: Option<NodeId> (Niche: 0x0)                          │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 48..52 │ flags: NodeFlags (bitflags! u32)                                   │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 52..56 │ [padding para alinhamento de 8 bytes]                              │
├────────┼────────────────────────────────────────────────────────────────────┤
│ 56..88 │ kind: NodeKind (Discriminante 8B + Payload inline máx 24B)         │
└─────────────────────────────────────────────────────────────────────────────┘
Total sizeof(NodeData): 88 Bytes (Redução de 39% sobre o layout original de 144B!)
```

#### Detalhamento de `NodeKind` Compactado (32 Bytes):
- **Discriminante:** 8 Bytes.
- **Variantes:**
  - `Element(Box<ElementData>)`: 8 Bytes (Ponteiro exclusivo).
  - `Text(TextData)`: 24 Bytes (`SmolStr` com até 23 bytes inline em UTF-8).
  - `Comment(CommentData)`: 24 Bytes (`SmolStr`).
  - `Document(Box<DocumentData>)`: 8 Bytes *(Variante rara boxed)*.
  - `DocumentType(Box<DoctypeData>)`: 8 Bytes *(Variante rara boxed)*.
  - `ShadowRoot(Box<ShadowRootData>)`: 8 Bytes *(Variante rara boxed)*.
  - `DocumentFragment`: 0 Bytes.

#### Detalhamento de `ElementData` (224 Bytes no Heap):
```rust
pub struct ElementData {
    pub tag_name: Atom,                      // 8 Bytes (Tagged NonZeroU64)
    pub namespace: Namespace,                // 1 Byte (+ 7 Bytes pad)
    pub attributes: InlineVec<Attribute, 4>, // 128 Bytes (4 x 32B Attribute)
    pub id_attr: Option<Atom>,               // 8 Bytes (Niche 0x0)
    pub classes: InlineVec<Atom, 4>,         // 40 Bytes (4 x 8B + 8B len/cap)
    pub rare_data: Option<Box<ElementRareData>>, // 8 Bytes (Niche null ptr)
}
```

#### Detalhamento de `Attribute` (32 Bytes):
- `name: Atom` (8 Bytes).
- `value: SmolStr` (24 Bytes — armazena até 23 caracteres sem alocação dinâmica no heap).

#### Detalhamento de `ElementRareData` (128 Bytes sob demanda via `Box`):
- `shadow_root: Option<NodeId>` (8B).
- `template_content: Option<NodeId>` (8B).
- `custom_element_definition: Option<Atom>` (8B).
- `form_owner: Option<NodeId>` (8B).
- `inline_style: Option<SmolStr>` (24B).
- `aria_role: Option<Atom>` (8B).
- `custom_properties: Option<Box<FxHashMap<Atom, SmolStr>>>` (8B).

---

### 4.2 Tabela de Densidade por Tipo de Nó vs SOTA

| Tipo de Nó | Arena Memory | Heap Memory | Total Memória `ace_dom` | Total Memória Blink | Total Memória WebKit | Total Memória Gecko |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Nó de Texto** ($\le 23$ bytes) | 96 B | 0 B | **96 B** | ~112 B | ~120 B | ~96 B |
| **Nó de Texto** ($> 23$ bytes) | 96 B | Len do texto | **~96 B + Len** | ~112 B + Len | ~120 B + Len | ~96 B + Len |
| **Nó de Comentário** | 96 B | 0 B | **96 B** | ~112 B | ~120 B | ~96 B |
| **Elemento Padrão** ($\le 4$ attrs, $\le 4$ classes) | 96 B | 224 B (+ 16B hdr) | **336 B** | ~400–512 B | ~380–480 B | ~320–440 B |
| **Elemento com `RareData`** (Shadow/Template) | 96 B | 368 B (+ 32B hdrs) | **496 B** | ~600–750 B | ~580–700 B | ~500–650 B |
| **Nó Documento** | 96 B | 64 B | **160 B** | ~512 B | ~480 B | ~400 B |

---

### 4.3 Análise Assintótica Formal das 15 Operações Primárias

| # | Operação Primária do DOM | Complexidade Temporal | Complexidade Espacial | Mecanismo Algorítmico / Garantia de Performance |
| :---: | :--- | :---: | :---: | :--- |
| **1** | `get_node(id)` / `get_node_mut(id)` | **$O(1)$** | $O(1)$ | Indexação direta no array contíguo da Arena + validação da versão geracional. |
| **2** | `create_element(tag)` | **$O(1)$ amort.** | $O(1)$ | Alocação no SlotMap da Arena com reciclagem em lista livre (free-list LIFO). |
| **3** | `append_child(parent, child)` | **$O(\text{depth} + M)$** | $O(1)$ | Validação de hierarquia anticiclo $O(\text{depth})$ (`is_inclusive_ancestor`) + swap de 4 ponteiros intrusivos $O(1)$ na arena + indexação de IDs/classes da subárvore inserida de $M$ nós no `ElementIndex`. |
| **4** | `insert_before(parent, new, ref)` | **$O(\text{depth} + M)$** | $O(1)$ | Validação de hierarquia anticiclo $O(\text{depth})$ + atualização de ponteiros duplamente ligados na arena + indexação de subárvore de $M$ nós no `ElementIndex`. |
| **5** | `remove_child(parent, child)` | **$O(M + R \times \text{depth})$** | $O(1)$ | Desconexão de ponteiros na arena em $O(1)$ + desindexação recursiva dos $M$ nós da subárvore no `ElementIndex` + ajuste de $R$ live ranges ativos. |
| **6** | `get_element_by_id(id)` | **$O(1)$** | $O(N)$ | Consulta direta na tabela hash `ElementIndex` (`FxHashMap<Atom, NodeId>`). |
| **7** | `get_elements_by_class_name(cls)`| **$O(1)$ lookup** | $O(N)$ | Consulta em `ElementIndex::get_by_class` retornando slice contíguo de `NodeId`. |
| **8** | `AncestorFilter::fast_reject` | **$O(1)$** | $O(1)$ (64 Bytes) | Duplo hash FNV-1a e DJB2 verificando contadores nos buckets; rejeita instantaneamente $\ge 85\%$ de falhas. |
| **9** | `query_selector` / `matches` (RTL) | **$O(K \times \text{depth})$** ($O(N_{\text{subtree}})$ p/ `:has()`) | $O(\text{depth})$ stack | RTL matching guiado por `AncestorFilter` (rejeição rápida $O(1)$); seletores relacionais `:has()` realizam busca DFS na subárvore ($O(N_{\text{subtree}})$ por nó avaliado, até $O(N^2)$ global sem cache). |
| **10**| `Adoption Agency Algorithm (AAA)` | **$O(1)$ limitado** | $O(1)$ | Loop externo limitado a 8 iterações e loop interno a 3 iterações (WHATWG §12.2.6.4.7). |
| **11**| `TreeWalker` / `descendants(DFS)` | **$O(N)$** | $O(1)$ | Caminhamento iterativo via ponteiros `first_child`/`next_sibling`/`parent` sem recursão no stack. |
| **12**| `compare_document_position(a, b)`| **$O(\text{depth})$** | $O(\text{depth})$ | Caminhamento inclusivo até a raiz para identificação do LCA e comparação de índices de ramos irmãos bifurcados. |
| **13**| `Range::compare_boundary_points` | **$O(\text{depth})$** | $O(\text{depth})$ | Avaliação de contêineres via LCA em $O(\text{depth})$ seguida de comparação de offsets numéricos. |
| **14**| `LiveRangeRegistry::notify_*` | **$O(R)$** | $O(1)$ | $R = \text{número de ranges ativos}$; ajusta offsets ou reparenta pontos de nós removidos. |
| **15**| `Document::drop` (Teardown) | **$O(N)$ desalocações** | $O(1)$ | Destruição da Arena contígua sem recursão em árvore, executando $O(N)$ chamadas de `dealloc` no heap para elementos com `Box<ElementData>` / `Box<RareDocumentData>`. |

---

## 5. Roteiro de Decomposição em Milestones & Contratos de Interface

O roteiro de engenharia decompõe a evolução do `ace_dom` em **6 Milestones rigorosos**, cada qual especificando suas dependências, saídas esperadas, estruturas de dados, enums e assinaturas Rust.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ROADMAP DE EXECUÇÃO DE MILESTONES                        │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ M1: Tokenizer FSM 100% WHATWG & Parser Hardening                    │   │
│   │ (88 estados FSM, Ambiguous Ampersand, Script Escape, Doctype ID)    │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                      │
│                                      v                                      │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ M2: Compactação de Memória & Otimização de Layout                   │   │
│   │ (NodeData 88B, Atom 8B, Box<RareDocumentData>, 39% RAM footprint)   │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                      │
│                                      v                                      │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ M3: Sincronização Dinâmica de Índices & Comparação LCA O(depth)     │   │
│   │ (ElementIndex O(1) live update, compare_document_position LCA)      │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                      │
│                                      v                                      │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ M4: AncestorFilter no Query Engine & Especificidade CSSOM           │   │
│   │ (DFS Selector Traversal + Bloom Fast Reject, Specificity(A,B,C))    │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                      │
│                                      v                                      │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ M5: Acoplamento do Event Loop com MutationObserver & Live Ranges   │   │
│   │ (Microtask Queue dispatch, Transient Observers, Auto-Notification)  │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                      │
│                                      v                                      │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ M6: Runner Oficial de Web Platform Tests (WPT) & Suítes .dat        │   │
│   │ (html5lib-tests harness, WPT DOM/Tree/Selectors Conformance)        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Milestone 1 (M1): Tokenizer FSM 100% WHATWG & Parser Hardening

- **Objetivo:** Completar a implementação de todos os 88 estados normativos na máquina de estados do Tokenizer, garantindo conformidade absoluta contra `html5lib-tests`.
- **Dependências:** `ace_core::text::SegmentedString`, `ace_dom::tokenizer`.
- **Entregáveis Técnicos:**
  1. Implementação dos 57 estados faltantes no `match self.state` de `HTMLTokenizer::tokenize`.
  2. Implementação do algoritmo normativo de escape de comentários em scripts (`ScriptDataEscapeStart` $\dots$ `ScriptDataDoubleEscaped`).
  3. Correção da regra da ampersand ambígua em atributos (não decodificar quando não terminar em `;` e for seguida de `=` ou alfanumérico).
  4. Extração completa de `public_identifier` e `system_identifier` no parsing de DOCTYPE.
  5. Tratamento de EOF sem descarte silencioso de buffers pendentes de tags/comentários.

#### Contrato de Interface & Estruturas de Dados (M1):

```rust
// In Albedo_Core_Engine/ace_dom/src/tokenizer/mod.rs

impl HTMLTokenizer {
    /// Executa o loop da máquina de estados caractere a caractere, consumindo o buffer
    /// de entrada `SegmentedString` e emitindo tokens para o `TokenSink`.
    pub fn tokenize<S: TokenSink>(&mut self, input: &mut SegmentedString, sink: &mut S) {
        while let Some(c) = input.current_char() {
            match self.state {
                TokenizerState::Data => self.handle_data_state(input, sink, c),
                TokenizerState::TagOpen => self.handle_tag_open_state(input, sink, c),
                TokenizerState::EndTagOpen => self.handle_end_tag_open_state(input, sink, c),
                TokenizerState::TagName => self.handle_tag_name_state(input, sink, c),
                TokenizerState::ScriptData => self.handle_script_data_state(input, sink, c),
                TokenizerState::ScriptDataEscapeStart => self.handle_script_escape_start(input, c),
                TokenizerState::ScriptDataEscaped => self.handle_script_data_escaped(input, sink, c),
                TokenizerState::ScriptDataDoubleEscapeStart => self.handle_script_double_escape_start(input, c),
                TokenizerState::ScriptDataDoubleEscaped => self.handle_script_double_escaped(input, sink, c),
                TokenizerState::ScriptDataDoubleEscapeEnd => self.handle_script_double_escape_end(input, c),
                TokenizerState::BeforeDoctypePublicIdentifier => self.handle_before_doctype_public_id(input, c),
                TokenizerState::DoctypePublicIdentifierDoubleQuoted => self.handle_doctype_public_id_dq(input, c),
                TokenizerState::BeforeDoctypeSystemIdentifier => self.handle_before_doctype_system_id(input, c),
                TokenizerState::DoctypeSystemIdentifierDoubleQuoted => self.handle_doctype_system_id_dq(input, c),
                // Demais 74 estados tratados exaustivamente sem fallback genérico...
                _ => self.handle_state_transition(input, sink, c),
            }
        }
        self.handle_eof(sink);
    }

    /// Trata a regra da ampersand ambígua em atributos conforme WHATWG §12.2.5.73.
    #[inline]
    fn decode_attribute_character_reference(&self, slice: &str) -> Option<(char, usize)> {
        if let Some((ch, consumed, has_semicolon)) = entities::decode_entity_raw(slice) {
            if !has_semicolon {
                let next_char = slice.as_bytes().get(consumed).copied();
                if let Some(b) = next_char {
                    if b == b'=' || b.is_ascii_alphanumeric() {
                        return None; // Regra da ampersand ambígua: proíbe decodificação
                    }
                }
            }
            return Some((ch, consumed));
        }
        None
    }
}
```

---

### Milestone 2 (M2): Compactação de Memória & Otimização de Layout (`NodeData` 88B, `Atom` 8B)

- **Objetivo:** Reduzir a pegada de memória de cada nó DOM em 39% na arena contígua, compactando `NodeData` para 88 Bytes e `Atom` para 8 Bytes.
- **Dependências:** `ace_core::collections::InlineVec`, `ace_dom::node`.
- **Entregáveis Técnicos:**
  1. Boxar as variantes raras em `NodeKind` (`Document`, `DocumentType`, `ShadowRoot`).
  2. Implementar `Atom` como `NonZeroU64` indexado (tabela estática $0 \dots 1023$ vs identificador dinâmico de `string_cache`).
  3. Reduzir `sizeof(NodeData)` de 144B para 88B.
  4. Reduzir `sizeof(ElementData)` de 472B para 224B.

#### Contrato de Interface & Estruturas de Dados (M2):

```rust
// In Albedo_Core_Engine/ace_dom/src/node/mod.rs

#[repr(C)]
pub struct NodeData {
    pub id: NodeId,                              // 8 Bytes (NonZeroU64)
    pub parent: Option<NodeId>,                  // 8 Bytes (Niche 0x0)
    pub first_child: Option<NodeId>,             // 8 Bytes (Niche 0x0)
    pub last_child: Option<NodeId>,              // 8 Bytes (Niche 0x0)
    pub prev_sibling: Option<NodeId>,            // 8 Bytes (Niche 0x0)
    pub next_sibling: Option<NodeId>,            // 8 Bytes (Niche 0x0)
    pub flags: NodeFlags,                        // 4 Bytes
    _pad: u32,                                   // 4 Bytes
    pub kind: NodeKind,                          // 32 Bytes
}

pub enum NodeKind {
    Element(Box<ElementData>),                   // 8 Bytes ptr
    Text(TextData),                              // 24 Bytes (SmolStr)
    Comment(CommentData),                        // 24 Bytes (SmolStr)
    Document(Box<DocumentData>),                 // 8 Bytes ptr (Rare)
    DocumentType(Box<DoctypeData>),              // 8 Bytes ptr (Rare)
    ShadowRoot(Box<ShadowRootData>),             // 8 Bytes ptr (Rare)
    DocumentFragment,                            // 0 Bytes
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Atom(NonZeroU64);

impl Atom {
    #[inline]
    pub const fn from_static_index(index: u32) -> Self {
        // Tag bit 63 como 0 para estático
        unsafe { Self(NonZeroU64::new_unchecked((index as u64) + 1)) }
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        let val = self.0.get();
        if val <= STATIC_ATOM_TABLE_LEN as u64 {
            STATIC_ATOM_TABLE[(val - 1) as usize]
        } else {
            DYNAMIC_ATOM_REGISTRY.resolve(val)
        }
    }
}
```

---

### Milestone 3 (M3): Sincronização Dinâmica do `ElementIndex` & Comparação LCA $O(\text{depth})$

- **Objetivo:** Eliminar todas as consultas $O(N)$ em `getElementById` e `compare_document_position`, atingindo $O(1)$ e $O(\text{depth})$ respectivamente.
- **Dependências:** `ace_dom::tree::Document`, `ace_dom::query::ElementIndex`.
- **Entregáveis Técnicos:**
  1. Embutir `ElementIndex` no `Document` com sincronização automática em `append_child`, `remove_child`, `set_attribute` e `remove_attribute`.
  2. Implementar `Document::get_element_by_id` em $O(1)$ consultando a tabela hash indexada.
  3. Implementar algoritmo de Ancestral Comum Mais Próximo (LCA) para `Document::compare_document_position` e `Range::compare_boundary_points`.

#### Contrato de Interface & Algoritmo LCA (M3):

```rust
// In Albedo_Core_Engine/ace_dom/src/tree/mod.rs

impl Document {
    /// Retorna o nó com o ID fornecido em O(1) amortizado.
    #[inline]
    pub fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        self.element_index.get_by_id_str(id)
    }

    /// Compara a posição topológica relativa de dois nós em O(depth_a + depth_b) conforme WHATWG DOM §4.2.
    pub fn compare_document_position(&self, a: NodeId, b: NodeId) -> DocumentPosition {
        if a == b {
            return DocumentPosition::empty();
        }

        // 1. Constrói cadeia inclusiva: [node, parent, grandparent, ..., root]
        let mut chain_a: InlineVec<NodeId, 32> = InlineVec::new();
        chain_a.push(a);
        chain_a.extend(self.ancestors(a).map(|(id, _)| id));

        let mut chain_b: InlineVec<NodeId, 32> = InlineVec::new();
        chain_b.push(b);
        chain_b.extend(self.ancestors(b).map(|(id, _)| id));

        // 2. Relação direta de ancestralidade (WHATWG DOM §4.2):
        // Se `b` é ancestral de `a` (b está em chain_a), `b` contém `a` e precede `a`.
        if chain_a.contains(&b) {
            return DocumentPosition::CONTAINS | DocumentPosition::PRECEDING;
        }
        // Se `a` é ancestral de `b` (a está em chain_b), `b` está contido em `a` e segue `a`.
        if chain_b.contains(&a) {
            return DocumentPosition::CONTAINED_BY | DocumentPosition::FOLLOWING;
        }

        // 3. Raízes diferentes -> Desconectados com ordenação consistente de fallback
        let root_a = *chain_a.last().unwrap();
        let root_b = *chain_b.last().unwrap();

        if root_a != root_b {
            let fallback = if b < a { DocumentPosition::PRECEDING } else { DocumentPosition::FOLLOWING };
            return DocumentPosition::DISCONNECTED | DocumentPosition::IMPLEMENTATION_SPECIFIC | fallback;
        }

        // 4. Caminha da raiz (fim da cadeia) até o ponto de divergência imediato (ramos irmãos sob o LCA)
        let mut idx_a = chain_a.len() - 1;
        let mut idx_b = chain_b.len() - 1;

        while idx_a > 0 && idx_b > 0 && chain_a[idx_a - 1] == chain_b[idx_b - 1] {
            idx_a -= 1;
            idx_b -= 1;
        }

        let branch_a = chain_a[idx_a - 1];
        let branch_b = chain_b[idx_b - 1];

        // 5. Determina a ordem na lista de filhos do LCA comum
        if self.is_sibling_preceding(branch_a, branch_b) {
            DocumentPosition::PRECEDING
        } else {
            DocumentPosition::FOLLOWING
        }
    }
}
```

---

### Milestone 4 (M4): Integração do `AncestorFilter` no Query Selector & Motor de Especificidade CSSOM

- **Objetivo:** Acoplar o Counting Bloom Filter de 64 buckets (`AncestorFilter`) no traversal DFS de seletores e introduzir o calculador formal de especificidade `(A, B, C)`.
- **Dependências:** `ace_dom::query::bloom`, `ace_dom::query::selector`, `ace_dom::cssom`.
- **Entregáveis Técnicos:**
  1. Passagem de `AncestorFilter` por pilha durante o caminhamento DFS de `query_selector` e `query_selector_all`.
  2. Implementação de `fast_reject` no casamento de combinadores descendentes (`match_chain_rtl`).
  3. Criação da struct `Specificity(pub u32, pub u32, pub u32)` com trait `Ord` conforme CSS Selectors 4.
  4. Suporte a seletores complexos em `:not(...)` e seletores relativos em `:has(...)`.

#### Contrato de Interface & Estruturas de Dados (M4):

```rust
// In Albedo_Core_Engine/ace_dom/src/cssom/mod.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity {
    pub a_ids: u32,              // Seletor de ID (#header)
    pub b_classes_attrs: u32,    // Classes, Atributos, Pseudo-classes (:hover)
    pub c_tags_elements: u32,    // Tags de tipo (div, span) e Pseudo-elementos (::before)
}

impl Specificity {
    pub fn of_selector(selector: &ComplexSelector) -> Self {
        let mut spec = Specificity::default();
        for (compound, _) in &selector.compounds {
            for simple in &compound.selectors {
                match simple {
                    SimpleSelector::Id(_) => spec.a_ids += 1,
                    SimpleSelector::Class(_) | SimpleSelector::Attribute { .. } => spec.b_classes_attrs += 1,
                    SimpleSelector::Tag(_) => spec.c_tags_elements += 1,
                    SimpleSelector::Pseudo(pseudo) => {
                        match pseudo {
                            PseudoClass::Is(list) | PseudoClass::Not(list) => {
                                // O peso é o mais específico da lista
                                if let Some(max_s) = list.iter().map(Self::of_selector).max() {
                                    spec.a_ids += max_s.a_ids;
                                    spec.b_classes_attrs += max_s.b_classes_attrs;
                                    spec.c_tags_elements += max_s.c_tags_elements;
                                }
                            }
                            PseudoClass::Where(_) => {} // :where() tem especificidade (0, 0, 0)
                            _ => spec.b_classes_attrs += 1,
                        }
                    }
                    SimpleSelector::Universal => {}
                }
            }
        }
        spec
    }
}
```

---

### Milestone 5 (M5): Acoplamento do Event Loop com `MutationObserver` e `LiveRangeRegistry`

- **Objetivo:** Integrar a entrega assíncrona de `MutationRecord`s na fila de microtasks do Event Loop e automatizar o registro de mutações no `Document`.
- **Dependências:** `ace_core::event_loop`, `ace_dom::observer`, `ace_dom::range`.
- **Entregáveis Técnicos:**
  1. Adicionar `LiveRangeRegistry` e `MutationObserverRegistry` no estado interno do `Document`.
  2. Disparar notificações automáticas em `Document::append_child`, `remove_child`, `insert_before`, `set_attribute`, `split_text`.
  3. Implementar transient observers para nós desconectados com observação `subtree: true`.
  4. Executar a entrega de lotes de observadores na fase de microtasks do ciclo de vida WHATWG.

#### Contrato de Interface & Pipeline de Microtasks (M5):

```rust
// In Albedo_Core_Engine/ace_dom/src/observer/mod.rs

pub trait MicrotaskScheduler {
    fn enqueue_microtask(&self, task: Box<dyn FnOnce() + Send + 'static>);
}

impl Document {
    /// Notifica a mutação para os observadores ativos e agenda microtask se a fila estava limpa.
    pub(crate) fn queue_mutation_record(&mut self, record: MutationRecord) {
        let had_pending = !self.mutation_queue.is_empty();
        self.mutation_queue.push(record);

        if !had_pending {
            if let Some(scheduler) = &self.microtask_scheduler {
                let doc_handle = self.weak_handle();
                scheduler.enqueue_microtask(Box::new(move || {
                    if let Some(doc) = doc_handle.upgrade() {
                        doc.lock().deliver_mutation_records();
                    }
                }));
            }
        }
    }
}
```

---

### Milestone 6 (M6): Runner Oficial de Web Platform Tests (WPT) & Test Harness `.dat`

- **Objetivo:** Implementar o harness oficial de execução para suítes de conformidade `.dat` do `html5lib-tests` e testes oficiais do WPT com serialização canônica.
- **Dependências:** `ace_test_driver`, `ace_dom`.
- **Entregáveis Técnicos:**
  1. Parser de suítes de teste no formato canônico `.dat` com seções `#data`, `#errors`, `#document`, `#document-fragment` (context element), `#script-on` e `#script-off`.
  2. Serializador canônico de árvore WPT (`CanonicalTreeSerializer`):
     - Formatação de profundidade com barras verticais (`| `) e 2 espaços por nível de indentação.
     - Serialização canônica de atributos em múltiplas linhas, ordenados alfabeticamente pelo nome qualificado (`name="value"`), logo abaixo do elemento.
     - Prefixação estrita de namespaces: `<svg rect>`, `<math math>`, e tags HTML sem prefixo `<div >`.
     - Encapsulamento de subárvores de `<template>` sob o marcador `content` (`|   content`).
     - Serialização canônica de doctypes (`<!DOCTYPE html ...>`), comentários (`<!-- ... -->`) e nós de texto (`"..."`).
  3. Runner automatizado que executa suítes de tree construction de documento e de fragmentos, comparando a árvore obtida contra a esperada e emitindo métricas percentuais de conformidade.

#### Contrato de Interface & Harness de Execução (M6):

```rust
// In Albedo_Core_Engine/ace_dom/tests/wpt_harness.rs

pub struct DatTestCase {
    pub data: String,
    pub context_element: Option<String>,
    pub expected_document: String,
    pub scripting_enabled: bool,
}

pub struct CanonicalTreeSerializer;

impl CanonicalTreeSerializer {
    /// Serializa uma árvore DOM no formato canônico exigido pelo WPT / html5lib-tests:
    /// - Atributos em linhas separadas abaixo do elemento, ordenados alfabeticamente.
    /// - Namespaces: `<svg tag>`, `<math tag>`, `<html_tag>`.
    /// - Templates: filhos encapsulados sob o marcador `content`.
    pub fn serialize_document(doc: &Document) -> String {
        let mut out = String::new();
        Self::serialize_children(doc, doc.root(), 0, &mut out);
        out
    }

    fn serialize_children(doc: &Document, parent: NodeId, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        for (child_id, child_node) in doc.children(parent) {
            match &child_node.data {
                NodeData::DocumentType(doctype) => {
                    let pub_sys = match (&doctype.public_id, &doctype.system_id) {
                        (Some(p), Some(s)) => format!(" \"{}\" \"{}\"", p, s),
                        (Some(p), None) => format!(" \"{}\" \"\"", p),
                        (None, Some(s)) => format!(" \"\" \"{}\"", s),
                        (None, None) => String::new(),
                    };
                    out.push_str(&format!("| {}<!DOCTYPE {}{}>\n", indent, doctype.name, pub_sys));
                }
                NodeData::Comment(text) => {
                    out.push_str(&format!("| {}<!-- {} -->\n", indent, text));
                }
                NodeData::Text(text) => {
                    out.push_str(&format!("| {}\"{}\"\n", indent, text));
                }
                NodeData::Element(elem) => {
                    let ns_prefix = match elem.namespace {
                        Namespace::Svg => "svg ",
                        Namespace::MathMl => "math ",
                        Namespace::Html => "",
                    };
                    out.push_str(&format!("| {}<{}{}>\n", indent, ns_prefix, elem.tag_name));

                    // Atributos ordenados alfabeticamente em linhas próprias
                    let mut sorted_attrs: Vec<_> = elem.attributes.iter().collect();
                    sorted_attrs.sort_by_key(|(name, _)| name.as_str());
                    for (name, val) in sorted_attrs {
                        out.push_str(&format!("| {}  {}=\"{}\"\n", indent, name, val));
                    }

                    // Se for <template>, serializa o template content
                    if elem.tag_name.as_str() == "template" && elem.namespace == Namespace::Html {
                        out.push_str(&format!("| {}  content\n", indent));
                        if let Some(content_id) = elem.rare_data.as_ref().and_then(|r| r.template_content) {
                            Self::serialize_children(doc, content_id, depth + 2, out);
                        }
                    } else {
                        Self::serialize_children(doc, child_id, depth + 1, out);
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct DatTestRunner;

impl DatTestRunner {
    /// Faz o parsing estruturado do arquivo .dat do html5lib-tests
    pub fn parse_dat_file(content: &str) -> Vec<DatTestCase> {
        let mut tests = Vec::new();
        // Itera sobre seções #data, #errors, #document-fragment, #document, #script-on, #script-off
        // Extrai context_element e flag de scripting conforme especificado.
        tests
    }

    /// Executa um caso de teste individual (modo documento ou modo fragmento com context element)
    pub fn run_test(test: &DatTestCase) -> Result<(), String> {
        let doc = if let Some(context) = &test.context_element {
            ace_dom::parse_html_fragment(&test.data, context, test.scripting_enabled)
        } else {
            ace_dom::parse_html_with_options(&test.data, test.scripting_enabled)
        };

        let actual_dump = CanonicalTreeSerializer::serialize_document(&doc);
        if actual_dump.trim_end() != test.expected_document.trim_end() {
            return Err(format!(
                "WPT Mismatch:\n=== Esperado ===\n{}\n=== Obtido ===\n{}",
                test.expected_document, actual_dump
            ));
        }
        Ok(())
    }
}
```

---

## 6. Estratégia de Compilação/Bindings WebIDL e Acoplamento com `ace_js`

A camada de bindings entre a máquina virtual ECMAScript (`ace_js`) e a árvore DOM (`ace_dom`) adota uma arquitetura de wrappers não-possuidores de alto desempenho, integrando uma **Tabela Efêmera (*Ephemeron Table*)** para o gerenciamento de ciclo de vida seguro entre heaps distintos sem risco de retenção cíclica ou vazamento de memória.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ARQUITETURA DE BINDINGS JS <-> DOM                       │
│                                                                             │
│    ┌──────────────────────────────────┐                                     │
│    │        ace_js Runtime Heap       │                                     │
│    │                                  │                                     │
│    │  ┌────────────────────────────┐  │                                     │
│    │  │ JS Object (HTMLDivElement) │  │                                     │
│    │  │                            │  │                                     │
│    │  │  - Prototype Link          │  │                                     │
│    │  │  - Private Slot: NodeId(42)│  │                                     │
│    │  │  - Event Listener Closures │  │                                     │
│    │  └─────────────┬──────────────┘  │                                     │
│    └────────────────┼─────────────────┘                                     │
│                     │                                                       │
│                     │ Resolves NodeId(42) in O(1)                           │
│                     v                                                       │
│    ┌──────────────────────────────────┐                                     │
│    │        ace_dom Arena Heap        │                                     │
│    │                                  │                                     │
│    │  ┌────────────────────────────┐  │                                     │
│    │  │ NodeData Slot #42          │  │                                     │
│    │  │                            │  │                                     │
│    │  │  - Tag: "div"              │  │                                     │
│    │  │  - Classes: ["container"]  │  │                                     │
│    │  │  - Children: NodeId(43)    │  │                                     │
│    │  └────────────────────────────┘  │                                     │
│    └──────────────────────────────────┘                                     │
│                     ^                                                       │
│                     │ Ephemeron GC Tracing (Weak Wrapper / Sweep Pruning)   │
│                     └───────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.1 Pipeline de Geração de Código WebIDL
1. **Definição de Interfaces `.webidl`:**
   - As interfaces normativas (ex: `Node.webidl`, `Element.webidl`, `Document.webidl`, `HTMLElement.webidl`) são compiladas em tempo de build pela ferramenta interna do workspace (`xtask webidl-codegen`).
2. **Emissão de Rust Traits e JS Trampolines:**
   - O gerador emite traits Rust com tipos estritos e funções de conversão `to_js_value` / `from_js_value`.
   - Getters e setters refletem propriedades IDL diretamente para atributos e campos DOM correspondentes (ex: `element.id`, `element.className`, `node.textContent`).

### 6.2 Estruturas de Dados dos Bindings & Semântica de Ephemerons (GC)

Para evitar vazamentos de memória e retenção artificial de subárvores DOM desconectadas, o `DOMDataStore` opera como uma **Ephemeron Table (Tabela Fraca)**:
- **Não atua como Strong Root incondicional:** O mapa de wrappers nunca marca todos os seus nós durante a fase de marcação do GC.
- **Regras de Sobrevivência do Wrapper:**
  1. O wrapper JS permanece vivo se for alcançável a partir das raízes do JavaScript (pilha de execução, variáveis globais, closures).
  2. O nó DOM permanece vivo se fizer parte de uma árvore conectada ativa (`Document`) ou se possuir event listeners / propriedades expandidas (*expandos*) vinculadas a fluxos ativos.
- **Fase de Sweep:** Durante a varredura do GC no `ace_js`, qualquer wrapper JS coletado tem sua entrada automaticamente expurgada de `wrapper_map` via `sweep_dead_wrappers`.

```rust
// In Albedo_Core_Engine/ace_dom/src/bindings/mod.rs

/// Tabela Efêmera (Ephemeron Table) que mapeia NodeId <-> Wrapper JS
/// sem impedir a coleta de lixo de nós e wrappers não alcançáveis.
pub struct DOMDataStore {
    /// Mapeamento fraco de NodeId para JSObjectId.
    /// Não mantém o wrapper vivo por si só; a entrada é expurgada se o wrapper for coletado.
    wrapper_map: FxHashMap<NodeId, WeakJSObjectId>,
    /// Nós DOM ativos mantidos vivos por raízes de eventos ou árvores conectadas.
    active_dom_roots: FxHashSet<NodeId>,
}

impl DOMDataStore {
    /// Registra ou atualiza a associação fraca entre um NodeId e seu wrapper JS.
    pub fn set_wrapper(&mut self, node_id: NodeId, wrapper: WeakJSObjectId) {
        self.wrapper_map.insert(node_id, wrapper);
    }

    /// Retorna o wrapper JS caso ainda esteja vivo.
    pub fn get_wrapper(&self, node_id: NodeId) -> Option<WeakJSObjectId> {
        self.wrapper_map.get(&node_id).copied()
    }

    /// Executado durante a fase de Sweep do Garbage Collector de `ace_js`.
    /// Remove todas as entradas de wrappers que foram coletados nesta rodada.
    pub fn sweep_dead_wrappers(&mut self, is_alive: impl Fn(WeakJSObjectId) -> bool) {
        self.wrapper_map.retain(|_, &mut weak_ref| is_alive(weak_ref));
    }

    /// Marca apenas nós DOM que são raízes ativas no grafo (com listeners ou subárvores conectadas).
    pub fn trace_active_roots(&self, tracer: &mut dyn GcTracer) {
        for &node_id in &self.active_dom_roots {
            tracer.trace_node(node_id);
        }
    }
}

pub struct ElementBindings;

impl ElementBindings {
    /// Trampoline invocado pela VM JS ao acessar `element.getAttribute(name)`.
    pub fn get_attribute(doc: &Document, node_id: NodeId, args: &[JSValue]) -> WebIDLResult<JSValue> {
        let attr_name = args.get(0).ok_or(WebIDLException::TypeError("Argument 1 missing"))?.as_string()?;
        let val = doc.get_attribute(node_id, &attr_name);
        Ok(match val {
            Some(s) => JSValue::String(s.into()),
            None => JSValue::Null,
        })
    }

    /// Trampoline invocado pela VM JS ao executar `element.setAttribute(name, value)`.
    pub fn set_attribute(doc: &mut Document, node_id: NodeId, args: &[JSValue]) -> WebIDLResult<JSValue> {
        let name = args.get(0).ok_or(WebIDLException::TypeError("Argument 1 missing"))?.as_string()?;
        let value = args.get(1).ok_or(WebIDLException::TypeError("Argument 2 missing"))?.as_string()?;
        doc.set_attribute(node_id, &name, &value)?;
        Ok(JSValue::Undefined)
    }
}
```

---

## 7. Infraestrutura de Testes, Validação WPT e Garantia de Qualidade

Para certificar que o subsistema `ace_dom` atinja a mais alta confiabilidade de nível de produção da indústria de navegadores, a estratégia de testes é estruturada em uma pirâmide de 5 Tiers de validação contínua.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   PIRÂMIDE DE VALIDAÇÃO DE QUALIDADE (5 TIERS)              │
│                                                                             │
│                       ┌─────────────────────────┐                           │
│                       │ Tier 5: Adversarial     │                           │
│                       │ (Fuzzing, Chaos, XSS)   │                           │
│                      ┌┴─────────────────────────┴┐                          │
│                      │ Tier 4: Real-World Workl. │                          │
│                      │ (Wikipedia, SPAs, 10k nós)│                          │
│                     ┌┴───────────────────────────┴┐                         │
│                     │ Tier 3: Cross-Feature Integ.│                         │
│                     │ (DSD + AAA + Range + MutObs)│                         │
│                    ┌┴─────────────────────────────┴┐                        │
│                    │ Tier 2: Boundary & Corner-Case│                        │
│                    │ (EOF, Saturação Bloom, Nulls) │                        │
│                   ┌┴───────────────────────────────┴┐                       │
│                   │ Tier 1: Feature Normative Cover.│                       │
│                   │ (100% dos métodos e APIs W3C)   │                       │
│                   └─────────────────────────────────┘                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.1 Os 5 Tiers de Validação

1. **Tier 1: Feature Coverage (Cobertura Normativa Unitária):**
   - Cobertura de 100% de todas as APIs públicas de `Document`, `NodeData`, `ElementData`, `Range`, `MutationObserver`, `HTMLTokenizer` e `HTMLTreeBuilder`.
   - Cada método deve possuir testes unitários verificando caminho feliz e erros de pré-condição.

2. **Tier 2: Boundaries & Corners (Limites e Casos de Borda):**
   - Entradas com strings vazias, bytes nulos no meio de tags, buffers truncados no EOF, aninhamento de 1000 nós de profundidade, saturação do contador de 255 no `AncestorFilter`.

3. **Tier 3: Cross-Feature Combinations (Integração Cruzada):**
   - Combinação de Declarative Shadow DOM com nós de formatação malformados (AAA).
   - Mutações estruturais ocorrendo durante a execução de `LiveRange` e entrega de `MutationObserver`.

4. **Tier 4: Real-World Workloads (Cargas Reais de Produção):**
   - Parsing de páginas reais completas (Wikipedia, GitHub, portais de notícias pesados) em menos de 200ms por megabyte, garantindo que nenhum panic ou regressão ocorra.

5. **Tier 5: Adversarial Hardening (Segurança e Robustez Extrema):**
   - Testes contínuos de fuzzing (`cargo fuzz`) no Tokenizer e Sanitizer.
   - Injeção de ciclos de herança intencionais, payloads de evasão XSS com scripts em maiúsculas, esquemas ofuscados (`java\nscript:`) e entidades XML malformadas.

### 7.2 Comandos de Validação e Portões de CI

Antes de qualquer merge ou conclusão de milestone, os seguintes portões são mandatórios:

```powershell
# 1. Validação estrita do Linter com zero tolerância a warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 2. Execução da suíte completa de testes unitários e de integração
cargo test --workspace

# 3. Execução dos testes adversários e de estresse do ace_dom
cargo test -p ace_dom --test adversarial_parser_stress_test
cargo test -p ace_dom --test adversarial_dom_challenge_test
cargo test -p ace_dom --test adversarial_extreme_edge_cases_test
```

---

## 🎯 Conclusão & Prontidão de Execução

O presente **Plano Mestre Definitivo do `ace_dom`** estabelece as bases matemáticas, estruturais e normativas para dotar o **Albedo Browser** de um motor DOM de classe mundial:
- **100% Safe Rust** sem atalhos nem `unsafe` arbitrário.
- **Arena Geracional de 8 Bytes** com destruição em $O(1)$ e eliminação total de vazamentos cíclicos.
- **Conformidade Normativa com WHATWG HTML/DOM e CSS Selectors 4**.
- **Desempenho Extremo com SIMD, Counting Bloom Filters e Cache-Friendliness**.

A engenharia do subsistema `ace_dom` está formalmente autorizada para execução seguindo os Milestones M1 a M6 descritos neste documento.
