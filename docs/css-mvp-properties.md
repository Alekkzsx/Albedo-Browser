# Propriedades CSS Suportadas no MVP

Para evitar o escopo infinito do CSS3, o motor `ace_style` suportará inicialmente APENAS o subconjunto que garante a renderização estrutural das páginas web sem quebrar.

### Display & Positioning
- `display`: block, inline, inline-block, flex, grid, none
- `position`: static, relative, absolute, fixed, sticky

### Box Model
- `margin`, `padding` (todas as variantes bottom/top/left/right)
- `border` (width, style, color, radius)
- `width`, `height`, `min-width/height`, `max-width/height`
- `box-sizing`

### Tipografia e Cores
- `color`, `font-family`, `font-size`, `font-weight`, `text-align`, `line-height`
- `background-color` (somente cores sólidas hex, rgb, rgba)
- `opacity`

### Layout Engines Suportados
- **Flexbox:** `flex-direction`, `justify-content`, `align-items`, `flex-wrap`, `flex-grow`, `flex-shrink`.
- **CSS Grid (MVP):** `grid-template-columns/rows`, `gap`, funcionalidade com `minmax()`.

> [!WARNING]
> Qualquer outra propriedade CSS encontrada nos sites reais será ignorada no MVP de forma silenciosa para não quebrar o layout agressivamente. Animações e gradientes foram movidos para a v1.5.
