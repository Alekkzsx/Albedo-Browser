# 🎨 Subconjunto de Propriedades CSS no MVP (v1.0)

> **Documento Normativo de Escopo CSS**  
> **Subsistema Alvo:** `Albedo_Core_Engine/ace_style`  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.0](../../PLANO.md) — §1.4 e Fase 6)  
> **Padrões Normativos:** W3C CSS Syntax L3, CSS Cascade L5/L6, CSS Box Model L3, CSS Flexible Box Layout L1, CSS Grid Layout L1.

---

## 1. Diretriz de Implementação

Para evitar a complexidade infinita das centenas de especificações CSS, o motor `ace_style` foca na renderização correta da **estrutura geométrica e tipográfica** dos sites modernos.

> [!NOTE]
> **Situação Arquitetural Transitória (DV-05):** O CSSOM e o cálculo inicial de cascata residem provisoriamente em `ace_dom/src/cssom`. Conforme aprovado no **ADR-0012**, eles serão migrados e unificados com o parser de Syntax L3 dentro do crate dedicado `ace_style` durante a execução da **Fase 6**.

Propriedades CSS não listadas neste catálogo serão parseadas e descartadas silenciosamente sem interromper o fluxo da árvore de renderização.

---

## 2. Catálogo Taxativo de Propriedades Suportadas

### 2.1 Modelo de Caixa e Geometria (Box Model)
- `width`, `height`
- `min-width`, `max-width`, `min-height`, `max-height`
- `margin`, `margin-top`, `margin-right`, `margin-bottom`, `margin-left` (e suporte a `auto`)
- `padding`, `padding-top`, `padding-right`, `padding-bottom`, `padding-left`
- `border-width`, `border-style`, `border-color` (shorthand `border` e variantes por lado)
- `border-radius` (curvas elípticas básicas conforme CSS Backgrounds & Borders L3 §5.5)
- `box-sizing`: `content-box`, `border-box`
- `overflow`: `visible`, `hidden`, `scroll`, `auto`
- Propriedades lógicas de dimensão: `inline-size`, `block-size`

### 2.2 Formatação, Posicionamento e Visibilidade
- `display`: `block`, `inline`, `inline-block`, `flex`, `grid`, `none`
- `position`: `static`, `relative`, `absolute`, `fixed`, `sticky`
- `top`, `right`, `bottom`, `left`
- `z-index`: `auto`, `<integer>` (formação de contextos de empilhamento básicos)
- `visibility`: `visible`, `hidden`, `collapse`
- `opacity`: `<number>` (0.0 a 1.0)

### 2.3 Layout Flexível (Flexbox)
- `flex-direction`: `row`, `row-reverse`, `column`, `column-reverse`
- `flex-wrap`: `nowrap`, `wrap`, `wrap-reverse`
- `flex-flow`: shorthand de direction e wrap
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

### 2.5 Tipografia e Cores
- `color`: Hexadecimal (`#rgb`, `#rrggbb`, `#rrggbbaa`), `rgb()`, `rgba()`, `hsl()`, `hsla()`, e cores do CSS Color 4 (`oklab()`, `oklch()`).
- `background-color`: Cores sólidas e transparência.
- `font-family`: Nomes genéricos (`serif`, `sans-serif`, `monospace`) e fontes locais mapeadas via `fontdb`.
- `font-size`: Unidades absolutas (`px`, `pt`) e relativas (`em`, `rem`, `%`).
- `font-weight`: `normal`, `bold`, valores numéricos de `100` a `900`.
- `font-style`: `normal`, `italic`.
- `text-align`: `left`, `right`, `center`, `justify`, `start`, `end`.
- `line-height`: `normal`, `<number>`, `<length>`.
- `text-decoration`: `none`, `underline`, `line-through`.
- `white-space`: `normal`, `nowrap`, `pre`, `pre-wrap`, `pre-line`.
- `word-break`, `overflow-wrap`: `normal`, `break-word`, `anywhere`.

### 2.6 Cascata e Resolução de Valores
- **Custom Properties (Variáveis CSS):** Declaração (`--custom-name`) e consumo via `var(--custom-name, fallback)` com resolução de ciclos de dependência.
- **Camadas de Cascata (`@layer`):** Suporte à ordenação explícita de regras conforme CSS Cascade 5.
- **Palavras-chave universais:** `inherit`, `initial`, `unset`, `revert`.

---

## 3. Pós-MVP (v1.5+)
- Animações e Transições (`transition`, `@keyframes`, `animation`).
- Transformações 3D e filtros avançados (`filter: blur()`, `backdrop-filter`).
- Consultas de contêiner (*Container Queries* `@container`).
- Efeitos complexos de máscara e mesclagem (`mix-blend-mode`).
