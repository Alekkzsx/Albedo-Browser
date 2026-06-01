# 📁 src/renderer/

Esta pasta contém o rasterizador CPU principal do **Albedo Browser**. Ele converte a lista de comandos visuais estruturados (Display List/Layer Tree) em uma grade de pixels na tela usando a biblioteca 2D **tiny-skia**.

## 🎯 Objetivo & Função
O módulo de renderização (`renderer`) representa a última etapa visível do pipeline visual (Paint). Ele recebe a lista de `DisplayItem`s (gerada pelo motor ACE após o cálculo do Layout e matching de estilos CSS) e desenha primitivas 2D (retângulos, bordas, textos, imagens) em um buffer de memória física (`tiny_skia::Pixmap`), que posteriormente é enviado para o Compositor GPU ou diretamente para a janela Slint.

## 📄 Arquivos e Suas Funções

| Arquivo | Função / Propósito |
| :--- | :--- |
| [mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/renderer/mod.rs) | Ponto de entrada do módulo. Expõe a função principal `paint_layout_tree`. |
| [paint.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/renderer/paint.rs) | Implementa a lógica detalhada de desenho de cada primitiva visual (backgrounds, border-radius, sombras, imagens, campos de formulário e renderização de fontes via `cosmic-text` e `swash`). |

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

### O que DEVE estar aqui:
- Lógica de desenho 2D pura baseada nas estruturas definidas na Display List/Layer Tree.
- Operações de clipping de coordenadas e interpolação de cores (gradientes).
- Rasterização e anti-aliasing de formas geométricas e renderização de glyphs de texto.
- Lógica para lidar com opacidade e filtros de mistura gráfica (Blend Modes).

### O que NÃO DEVE estar aqui:
- Lógica de parsing de HTML, CSS ou resolução de seletores de estilo (deve ocorrer em `src/ace/`).
- Cálculos de geometria ou distribuição de nós em Grid/Flexbox (deve ocorrer no motor de Layout).
- Chamadas diretas a APIs de janela do sistema operacional ou frameworks de UI (Slint). O renderer deve apenas preencher um Pixmap genérico na memória.
- Operações de rede para baixar imagens ou assets (deve ocorrer em `src/network/`).
