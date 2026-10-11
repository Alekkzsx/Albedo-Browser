# 🎨 Arquitetura do Motor de Estilos — `ace_style`

> **Documento de Arquitetura de Subsistema — Albedo Core Engine (ACE)**  
> **Crate:** `Albedo_Core_Engine/ace_style`  
> **Fase no PLANO:** Fase 6 (Concluída em 2026-10-11)  
> **Status:** Ativo & Auditado (100% Safe Rust com `#![forbid(unsafe_code)]`)  
> **Padrões Normativos:** W3C CSS Syntax L3, CSS Cascade L5/L6, CSS Selectors L4, CSS Values and Units L4, CSS Custom Properties L1, WHATWG HTML §15 (User-Agent Stylesheet).

---

## 📜 Histórico de Revisões e Governança

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.0.0** | 2026-10-11 | Implementação da Fase 6 | • Arquitetura modular em 6 frentes autônomas.<br>• Resolução de dívida DV-05 com migração e unificação do CSSOM.<br>• `#![forbid(unsafe_code)]` rigorosamente imposto no crate.<br>• Implementação de `RuleBucketMap` indexado por ID/Class/Tag.<br>• Algoritmo de resolução de variáveis `var()` com detecção de ciclo em grafo (DAG).<br>• Matriz de cascata com 8 origens normativas e `@layer`.<br>• `StyleSharingCache` para eliminação de redundância entre nós irmãos. | • 5.675 linhas de código limpo e seguro em 8 módulos.<br>• 18 testes automatizados aprovados sem falhas.<br>• 6 suítes de estresse pesado cobrindo DoS, concorrência, fuzzing e escala.<br>• 0 erros e 0 warnings no Clippy. |

---

## 1. Visão Geral e Papel no Pipeline

O subsistema `ace_style` é o terceiro estágio essencial do pipeline de renderização do Albedo Core Engine. Ele atua imediatamente após a construção da árvore DOM por `ace_dom` e antecede o cálculo geométrico do motor de layout (`ace_layout`):

```mermaid
flowchart TD
    subgraph Entrada ["Entradas Normativas"]
        DOM[Árvore DOM da Arena ace_dom]
        HTML_CSS[CSS Embutido style e link]
        UA_CSS[WHATWG §15 Default UA Stylesheet]
    end

    subgraph ace_style ["Motor ace_style (Fase 6)"]
        Parser[CSS Syntax L3 Tokenizer & Parser SIMD]
        RuleIndex[RuleBucketMap: ID, Class, Tag, Universal]
        Matcher[Rule Matcher + Ancestor Bloom Filter]
        Cascade[Cascade Sorter: 8 Origens + @layer + Especificidade]
        VarResolver[VariableResolver: DAG Anti-Cycle var()]
        SharingCache[StyleSharingCache: Cache de Irmãos]
    end

    subgraph Saida ["Saída Estrutural"]
        RenderTree[Render Tree: Nós Visíveis + ComputedStyle]
        Layout[ace_layout: Geometria & Fragment Tree]
    end

    HTML_CSS --> Parser
    UA_CSS --> Parser
    Parser --> RuleIndex
    DOM --> Matcher
    RuleIndex --> Matcher
    Matcher --> Cascade
    Cascade --> VarResolver
    VarResolver --> SharingCache
    SharingCache --> RenderTree
    RenderTree --> Layout
```

O objetivo primordial de `ace_style` é transformar regras textuais declarativas em um conjunto imutável de propriedades resolvidas ([`ComputedStyle`](file:///home/alekkzsx/Documents/Github/Albedo-Browser/Albedo_Core_Engine/ace_style/src/computed/mod.rs)) para cada nó visível da página, gerando a [`RenderTree`](file:///home/alekkzsx/Documents/Github/Albedo-Browser/Albedo_Core_Engine/ace_style/src/engine/mod.rs) com alto paralelismo e sem alocações desnecessárias.

---

## 2. As Seis Frentes Autônomas de Implementação

A arquitetura do `ace_style` foi projetada em 6 frentes desacopladas, espelhadas nos módulos do crate:

```
Albedo_Core_Engine/ace_style/src/
├── lib.rs              # Re-exports públicos e declaração de módulos
├── values/             # Frente 1: Tipos primitivos, comprimentos, cores e enums
│   └── mod.rs
├── parser/             # Frente 2: Tokenizer L3 streaming e parser de declarações
│   └── mod.rs
├── model/              # Frente 3: Modelo de regras (@media, @layer, StyleSheet)
│   └── mod.rs
├── user_agent/         # Frente 3: Folha de estilos padrão do navegador
│   └── mod.rs
├── rule_tree/          # Frente 4: Indexação por baldes e matching com Bloom filter
│   └── mod.rs
├── cascade/            # Frente 5: Cascata de 8 origens, camadas e especificidade
│   └── mod.rs
├── computed/           # Frente 6: ComputedStyle, herança e resolução de variáveis
│   └── mod.rs
└── engine/             # Frente 6: Orquestrador StyleEngine, Cache e RenderTree
    └── mod.rs
```

---

### Frente 1: Valores, Unidades e Tipos Primitivos (`values/`)

Define as estruturas de dados fundamentais do CSS com foco em consumo de memória compacto e precisão:

- **`Length`:** Representação de comprimentos absolutos (`Px(f32)`, `Pt(f32)`, `In(f32)`, `Mm(f32)`), relativos (`Em(f32)`, `Rem(f32)`, `Vw(f32)`, `Vh(f32)`, `Percent(f32)`), automáticos (`Auto`) e expressões avaliadas dinamicamente (`Calc(CalcExpr)`).
- **`CalcExpr`:** AST de avaliação para `calc()`, suportando soma, subtração, multiplicação e divisão com resolução contextual.
- **`CssColor` / `Color`:** Suporte a formato RGBA de 32 bits, hexadecimais de 3, 4, 6 e 8 dígitos, funções funcionais `rgb()`, `rgba()`, `hsl()`, `hsla()` e integração com o espaço perceptualmente uniforme do CSS Color 4 (`oklab()`, `oklch()`).
- **Enums Estruturais:** `Display` (`None`, `Block`, `Inline`, `InlineBlock`, `Flex`, `Grid`), `Position` (`Static`, `Relative`, `Absolute`, `Fixed`, `Sticky`), `BoxSizing`, `Visibility`, `FlexDirection`, `FlexWrap`, `JustifyContent`, `AlignItems`, etc.

---

### Frente 2: Tokenizador e Parser CSS Syntax L3 (`parser/`)

Implementa a gramática formal do **W3C CSS Syntax Module Level 3**:

- **Varredura Otimizada:** Tokenizador de streaming que opera sobre slices UTF-8, emitindo tokens tipados (`Ident`, `Function`, `AtKeyword`, `Hash`, `String`, `Url`, `Delim`, `Number`, `Percentage`, `Dimension`, `Whitespace`, `CDO`, `CDC`, `Colon`, `Semicolon`, `Comma`, parênteses e chaves).
- **Preservação de Escapes e Caracteres Especiais:** Suporte a caracteres escapados em seletores (`\.`, `\#`, `\ `).
- **Parser Resiliente:** Tratamento gracioso de erros de sintaxe (descarte determinístico de tokens malformados até o próximo delimitador de bloco ou ponto-e-vírgula, prevenindo travamento do pipeline).
- **Tratamento de Modificadores:** Detecção nativa do modificador `!important`.

---

### Frente 3: Modelo de Regras e Folha User-Agent (`model/` e `user_agent/`)

Organiza as declarações CSS em estruturas manipuláveis e fornece os estilos padrão de renderização:

- **`StyleSheet` & `CSSRule`:** Representação de regras com suporte a regras de estilo regulares (`StyleRule`), blocos condicionais `@media` (`MediaRule`) e blocos de camadas `@layer` (`LayerBlockRule`).
- **`LayerRegistry`:** Mapeamento determinístico de ordem de precedência de camadas conforme declaradas no documento.
- **Folha User-Agent Padrão (WHATWG §15):** Regras canônicas para elementos HTML fundamentais:
  - Estilização de blocos (`<div>`, `<p>`, `<h1>` a `<h6>`, `<header>`, `<footer>`, `<section>`, `<article>`, `<nav>`, `<aside>`).
  - Listas (`<ul>`, `<ol>`, `<li>`, `<dl>`, `<dt>`, `<dd>`).
  - Tabelas (`<table>`, `<tr>`, `<td>`, `<th>`).
  - Formulários (`<input>`, `<textarea>`, `<select>`, `<button>`).
  - Ocultação nativa de elementos de metadados (`display: none` para `<head>`, `<meta>`, `<title>`, `<style>`, `<script>`, `<link>`, `[hidden]`).

---

### Frente 4: Indexação por Baldes e Seletores RTL (`rule_tree/`)

Garante complexidade sublinear no casamento de seletores CSS contra o DOM:

- **`RuleBucketMap`:** Particiona todas as regras ativas em quatro baldes indexados pela chave direita (*key selector*):
  1. **ID Bucket:** Regras com `#id` como seletor chave.
  2. **Class Bucket:** Regras com `.class` como seletor chave.
  3. **Tag Bucket:** Regras com `tag` como seletor chave.
  4. **Universal Bucket:** Regras curinga (`*`) ou seletores genéricos.
- **Matching Right-to-Left (RTL):** O motor valida primeiro o elemento alvo antes de percorrer a cadeia de ancestrais.
- **Integração com `AncestorFilter` (Bloom Filter):** Reutiliza o Bloom filter de 64 buckets do `ace_dom`. Se a tag, classe ou ID do ancestral não estiver presente no filtro de bits do nó, toda a cadeia de seletores descendentes é descartada instantaneamente em tempo \(O(1)\).

---

### Frente 5: Cascata Normativa e Especificidade (`cascade/`)

Implementa a matriz formal de ordenação do **CSS Cascading and Inheritance Level 5/6**:

1. **8 Origens Normativas:** Ordenadas estritamente por ordem de precedência:
   1. User-Agent Normal
   2. User Normal
   3. Author Normal (Layers explícitas em ordem de declaração)
   4. Author Normal (Unlayered — vence layers declaradas)
   5. CSS Animations
   6. Author `!important` (Inversão: unlayered perde para layered)
   7. User `!important`
   8. User-Agent `!important`
2. **Especificidade Tripla `(a, b, c)`:**
   - `a`: Número de seletores de ID.
   - `b`: Número de seletores de classe, pseudo-classes e atributos.
   - `c`: Número de seletores de tipo (tags) e pseudo-elementos.
3. **Desempate por Ordem de Origem:** Em caso de igualdade de especificidade, vence a regra declarada por último no código-fonte.

---

### Frente 6: Computação, Herança, Variáveis e Cache (`computed/` e `engine/`)

O estágio de finalização gera os estilos definitivos e prepara a árvore de renderização:

- **`ComputedStyle`:** Estrutura imutável e compacta contendo todas as propriedades resolvidas para um elemento específico.
- **Herança Automática:** Propriedades herdadas (`color`, `font-family`, `font-size`, `line-height`, `visibility`) propagam-se do nó pai para os filhos; propriedades não-herdadas (`margin`, `padding`, `display`, `width`) retornam ao valor inicial.
- **`VariableResolver` (Resolução de `var()`):**
  - Implementa um Grafo Acíclico Dirigido (DAG) para resolver referências entre variáveis CSS customizadas (`--name`).
  - **Mitigação DoS / Ciclos:** Detecção de ciclos em tempo linear com pilha de nós em visitação. Se um ciclo for detectado (ex: `--a: var(--b); --b: var(--a)`), a variável é invalidada imediatamente para `unset`, evitando estouro de pilha (*stack overflow*) e ataques de expansão exponencial (*Billion Laughs CSS*).
- **`StyleSharingCache`:**
  - Armazena em cache o estilo computado de nós irmãos com os mesmos seletores, classes e atributos.
  - Elimina até 90% dos cálculos repetidos de estilo em listas e tabelas densas.
- **`StyleEngine` e `RenderTree`:**
  - Orquestra a travessia da árvore DOM gerando a `RenderTree`.
  - Elementos com `display: none` são omitidos da `RenderTree`, poupando processamento de layout e pintura.

---

## 3. Segurança e Invariantes de Memória

- **100% Safe Rust:** O crate `ace_style` possui a anotação `#![forbid(unsafe_code)]` ativada na raiz de sua biblioteca.
- **Zero Alocações Cíclicas:** A resolução de propriedades é estritamente imutável (`Arc<ComputedStyle>`), não permitindo retenção de ponteiros cíclicos ou vazamentos de memória.
- **Proteção contra DoS e Fuzzing:** O parser de declarações e o avaliador de expressões `calc()` possuem limites estritos de profundidade recursiva, impedindo estouros de pilha (*stack overflow*) por aninhamento adversarial de blocos de chaves.

---

## 4. Métricas e Bateria de Testes

O `ace_style` conta com **18 testes automatizados aprovados (0 falhas)** em 7 binários dedicados, combinando testes unitários e testes de estresse de nível industrial:

| Binário de Teste | Tipo | Cenário Coberto |
| :--- | :---: | :--- |
| `unit_values_and_parser_test` | Unitário | Conversões de comprimento, CSS Colors, expressões `calc()`, tokenização normativa e expansão de shorthands. |
| `stress_cascade_layer_matrix` | Estresse | Resolução da matriz completa de 8 origens, inversão de precedência em `!important` e blocos aninhados de `@layer`. |
| `stress_concurrent_style_engine` | Concorrência | Processamento massivo concorrente do motor de estilos simulando múltiplas threads via `rayon`. |
| `stress_parser_fuzz_adversarial` | Fuzzing / DoS | Injeção de payloads maliciosos, milhares de chaves abertas recursivas e cargas extremas de declarações. |
| `stress_rule_matching_scale` | Escala | Casamento de seletores com mais de 10.000 regras simultâneas acelerado pelo Bloom filter. |
| `stress_style_sharing_cache` | Cache / Heap | Reutilização de estilos computados em 10.000 nós irmãos idênticos, garantindo taxa de reaproveitamento máxima. |
| `stress_variables_cycle_explosion` | Segurança | Prevenção de ataques *Billion Laughs*, detecção de ciclos diretos/mútuos, e resolução de anéis de 500 nós em tempo linear. |

---

## 5. Como Executar e Validar

```bash
# Executar toda a suíte de testes do ace_style
cargo test -p ace_style

# Executar a verificação estrita do Clippy
cargo clippy -p ace_style --all-targets --all-features -- -D warnings

# Validar formatação do código
cargo fmt --check -p ace_style
```
