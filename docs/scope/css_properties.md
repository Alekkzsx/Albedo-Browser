# 🎨 Subconjunto de Propriedades CSS no MVP (v1.0)

> **Documento Normativo de Escopo CSS**  
> **Subsistema Alvo:** `Albedo_Core_Engine/ace_style`  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.1](../../PLANO.md) — §1.4 e Fase 6 Concluída)  
> **Última Atualização:** 2026-10-12  
> **Padrões Normativos:** W3C CSS Syntax L3, CSS Cascade L5/L6, CSS Box Model L3, CSS Flexible Box Layout L1, CSS Grid Layout L1, CSS Values and Units L4.

---

## 1. Diretriz de Implementação

Para garantir fidelidade visual sem complexidade desnecessária, o motor `ace_style` concentra-se na renderização determinística da **estrutura geométrica, tipográfica e reativa** dos layouts web modernos.

> [!NOTE]
> **Status Arquitetural (DV-05 Resolvido):** O CSSOM e o cálculo de cascata foram integralmente implementados dentro do crate dedicado `ace_style` na **Fase 6** (conforme aprovado no [**ADR-0012**](../adr/0012-migracao-cssom-ace-style.md)), eliminando a dívida técnica DV-05 com 100% Safe Rust (`#![forbid(unsafe_code)]`).

Propriedades CSS não reconhecidas são parseadas e descartadas de forma segura e silenciosa, sem interromper a árvore DOM ou a compilação da `RenderTree`.

---

## 2. Catálogo Taxativo de Propriedades e Recursos Suportados

### 2.1 Modelo de Caixa e Geometria (Box Model)
- `width`, `height`
- `min-width`, `max-width`, `min-height`, `max-height`
- `margin`, `margin-top`, `margin-right`, `margin-bottom`, `margin-left` (suporte a `auto` e resolução de margens colapsantes)
- `padding`, `padding-top`, `padding-right`, `padding-bottom`, `padding-left`
- `border-width`, `border-style`, `border-color` (shorthand `border` e variantes por lado)
- `border-radius` (curvas elípticas básicas conforme CSS Backgrounds & Borders L3 §5.5)
- `box-sizing`: `content-box`, `border-box`
- `overflow`: `visible`, `hidden`, `scroll`, `auto`
- Propriedades lógicas dimensionais: `inline-size`, `block-size`, `min-inline-size`, `max-inline-size`, `min-block-size`, `max-block-size`
- Propriedades lógicas direcionais: `margin-inline-start`, `margin-inline-end`, `margin-block-start`, `margin-block-end`, `padding-inline-start`, `padding-inline-end`, `padding-block-start`, `padding-block-end`

### 2.2 Formatação, Posicionamento e Visibilidade
- `display`: `block`, `inline`, `inline-block`, `flex`, `grid`, `none`
- `position`: `static`, `relative`, `absolute`, `fixed`, `sticky`
- `top`, `right`, `bottom`, `left`
- `z-index`: `auto`, `<integer>` (formação de contextos de empilhamento)
- `visibility`: `visible`, `hidden`, `collapse`
- `opacity`: `<number>` (0.0 a 1.0)
- Pseudo-elementos: `::before`, `::after` (injeção automática de caixas anônimas na `RenderTree`)

### 2.3 Layout Flexível (Flexbox)
- `flex-direction`: `row`, `row-reverse`, `column`, `column-reverse`
- `flex-wrap`: `nowrap`, `wrap`, `wrap-reverse`
- `flex-flow`: shorthand de `flex-direction` e `flex-wrap`
- `justify-content`: `flex-start`, `flex-end`, `center`, `space-between`, `space-around`, `space-evenly`
- `align-items`, `align-self`: `stretch`, `flex-start`, `flex-end`, `center`, `baseline`
- `align-content`: `stretch`, `flex-start`, `flex-end`, `center`, `space-between`, `space-around`
- `flex-grow`, `flex-shrink`, `flex-basis` (e shorthand `flex`)
- `order`: `<integer>`
- `gap`, `row-gap`, `column-gap`

### 2.4 Layout em Grade (CSS Grid — Subconjunto Estrutural)
- `grid-template-columns`, `grid-template-rows` (suporte a unidades `px`, `%`, `fr`, `auto` e função `minmax()`)
- `grid-column-start`, `grid-column-end`, `grid-row-start`, `grid-row-end` (e shorthands `grid-column`, `grid-row`)
- `grid-auto-flow`: `row`, `column`, `dense`
- `grid-auto-columns`, `grid-auto-rows`
- `gap`, `row-gap`, `column-gap`

### 2.5 Tipografia, Cores e Modos de Escrita
- `color`: Hexadecimal (`#rgb`, `#rrggbb`, `#rrggbbaa`), `rgb()`, `rgba()`, `hsl()`, `hsla()`, e espaços CSS Color 4 (`oklab()`, `oklch()`).
- `background-color`: Cores sólidas e transparência (`transparent`).
- `font-family`: Nomes genéricos (`serif`, `sans-serif`, `monospace`) e fontes locais mapeadas via `fontdb`.
- `font-size`: Unidades absolutas (`px`, `pt`) e relativas (`em`, `rem`, `%`).
- `font-weight`: `normal`, `bold`, valores numéricos de `100` a `900`.
- `font-style`: `normal`, `italic`.
- `text-align`: `left`, `right`, `center`, `justify`, `start`, `end`.
- `line-height`: `normal`, `<number>`, `<length>`.
- `text-decoration`: `none`, `underline`, `line-through`.
- `white-space`: `normal`, `nowrap`, `pre`, `pre-wrap`, `pre-line`.
- `word-break`, `overflow-wrap`: `normal`, `break-word`, `anywhere`.
- `direction`: `ltr`, `rtl` (com suporte a matriz combinatória bidi testada no motor).

### 2.6 Expressões Matemáticas e Funções de Valor (CSS Values L4)
- `calc()`: Avaliação de expressões com adição, subtração, multiplicação e divisão de unidades dimensionais mistas.
- `min()`, `max()`, `clamp()`: Funções matemáticas de limite dimensional.
- `round()`, `abs()`, `sign()`: Funções numéricas resolvidas no compilador de estilo.

### 2.7 Cascata, Variáveis e Resolução de Valores
- **Custom Properties (Variáveis CSS):** Declaração (`--custom-name`) e consumo via `var(--custom-name, fallback)` com resolução DAG acíclica e proteção contra ataques *Billion Laughs CSS*.
- **Camadas de Cascata (`@layer`):** Ordenação explícita de regras conforme CSS Cascade 5, incluindo inversão de precedência para declarações `!important`.
- **Origens Normativas da Cascata:** 8 origens suportadas (`UserAgent`, `UserNormal`, `AuthorNormal`, `AuthorLayers`, `Animation`, `AuthorImportant`, `UserImportant`, `UserAgentImportant`).
- **Palavras-chave universais:** `inherit`, `initial`, `unset`, `revert`.

### 2.8 Contrato com a Fase 7 (`ace_layout`)
O estilo resolvido classifica cada nó da árvore em uma categoria formal do enum `BoxGenerationKind`:
- `Normal(Block)` / `Normal(Inline)` / `Normal(InlineBlock)`: Caixas normais de fluxo.
- `Normal(Flex)` / `Normal(Grid)`: Contêineres de formatação independente.
- `Anonymous`: Caixas anônimas sintetizadas para pseudo-elementos ou normalização de blocos.
- `TextRun`: Segmentos de texto sem estilos de contêiner.
- `None`: Nós suprimidos da árvore de renderização (`display: none`).

---

## 3. Pós-MVP (v1.5+)
- Animações e Transições (`transition`, `@keyframes`, `animation`).
- Transformações 3D e filtros avançados (`filter: blur()`, `backdrop-filter`).
- Consultas de contêiner (*Container Queries* `@container`).
- Efeitos complexos de máscara e mesclagem (`mix-blend-mode`).

