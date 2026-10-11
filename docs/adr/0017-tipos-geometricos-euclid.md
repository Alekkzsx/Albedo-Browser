# ADR-0017: Tipos Geométricos 2D/3D via `euclid`

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_core`, `ace_layout`, `ace_gfx`, `ace_style`

---

## 1. Contexto e Problema

Motores de layout e renderização manipulam coordenadas em múltiplos espaços dimensionais distintos:
- **Espaço de Layout CSS (CSS Pixels):** Coordenadas lógicas definidas pelas propriedades CSS.
- **Espaço do Dispositivo (Device Pixels):** Pixels físicos da tela após aplicação do fator de escala HiDPI / Retina (*Device Pixel Ratio*).
- **Espaço da Janela / Viewport:** Coordenadas relativas à área cliente da aplicação.
- **Espaço da Tela do SO:** Coordenadas absolutas do desktop.

Em implementações ingênuas baseadas em primitivas não tipadas como tuplas `(f32, f32)` ou structs genéricas `Point { x: f32, y: f32 }`, erros graves de escala ocorrem com facilidade (por exemplo, somar pixels CSS diretamente com device pixels), produzindo renderizações borradas, tamanhos incorretos de fontes e cliques descalibrados.

## 2. Decisão

Adotamos a crate de geometria fortemente tipada [`euclid`](https://docs.rs/euclid) como padrão fundacional em `ace_core::geometry`:

1. **Tipagem por Tag de Unidade (Phantom Types):**
   - Todas as estruturas geométricas (`Point2D<T, U>`, `Rect<T, U>`, `Size2D<T, U>`, `Vector2D<T, U>`, `Transform3D<T, Src, Dst>`) são parametrizadas por uma unidade de marcação (`U`).
   - Declaramos marcadores estruturais explícitos em `ace_core`: `CssPixel`, `DevicePixel`, `LayoutPixel`.
2. **Conversões Explícitas Obrigatórias:** A transformação de um ponto de `CssPixel` para `DevicePixel` exige uma operação matemática explícita multiplicando pela taxa de escala (`Scale<f32, CssPixel, DevicePixel>`), impedindo que o compilador Rust admita operações aritméticas entre espaços de coordenadas incompatíveis.

## 3. Consequências

- **Positivas:**
  - Eliminação total em tempo de compilação de bugs de mistura de espaços de coordenadas (CSS vs Device vs Window).
  - Suporte de primeira classe a transformações afins e matrizes de projeção 3D para o pipeline gráfico.
  - Zero custo de abstração em runtime (as tags de unidade são tipos fantasma de tamanho zero).
- **Negativas / Riscos:**
  - Assinaturas de funções geométricas mais verbosas exigindo a parametrização do espaço de unidades.
