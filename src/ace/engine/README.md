# 📁 src/ace/engine/

Este diretório contém o núcleo de processamento, layout e renderização visual do navegador **Albedo-Browser** (codinome **Ace**). A engine é responsável por transformar documentos HTML e folhas de estilo CSS em uma representação gráfica interativa na tela, integrando todas as fases tradicionais de um motor de renderização web (DOM, CSSOM, Style Matching, Layout e Compositing/Graphics).

---

## 🎯 Objetivo & Função

A engine **Ace** atua como a ponte entre o código da web (HTML, CSS, JS) e a exibição final ao usuário. Na arquitetura do sistema, o fluxo de execução segue estas etapas principais:

1. **Construção do DOM (Document Object Model):** Armazena a estrutura lógica hierárquica do documento em uma árvore de nós (`AceDOM`).
2. **Resolução e Matching de Estilos (CSS Style Matching):** Associa regras CSS (tanto User-Agent quanto fornecidas pelo autor) aos nós correspondentes do DOM com base em seletores, tratando herança, estados interativos (`:hover`, `:active`, `:focus`) e processamento de animações.
3. **Cálculo de Layout:** Determina o tamanho e a posição exata de cada nó na tela. Combina o poder da biblioteca **Taffy** (para Grid e Flexbox) com algoritmos específicos da engine para layout de texto inline, posicionamento flutuante (`float`) e regras de desvio (`clear`).
4. **Composição e Gráficos (Graphics/Compositor):** Gera uma árvore de camadas (`LayerTree`) a partir de primitivas de desenho (`DisplayItem`), aplicando transformações, filtros e transparências. A renderização final suporta tanto aceleração por hardware (via **WGPU** e shaders WGSL) quanto rasterização em CPU (via **Tiny-Skia**).

> [!NOTE]
> A engine foi projetada para funcionar de maneira reativa e incremental. Mutações e animações marcam partes da árvore como sujas (*dirty flags*), disparando apenas as etapas de recalculo necessárias para manter o desempenho em 60 FPS.

---

## 📄 Arquivos e Suas Funções

Abaixo estão descritos os principais subdiretórios e arquivos que compõem o motor **Ace**:

| Arquivo / Subpasta | Tipo | Descrição & Função |
| :--- | :--- | :--- |
| [dom/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine/dom/) | Pasta | Módulo responsável pela estrutura de árvore de nós do documento (DOM). Gerencia Shadow DOM, custom elements (Web Components), listas de nós dinâmicas (`live_nodelist`) e APIs de seleção e ranges. |
| [style/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine/style/) | Pasta | Módulo de parseamento e cálculo de estilos CSS. Implementa a cascata CSS, cálculo de herança de valores, seletores de estilo, media queries e o gerenciador de animações (`AnimationManager`). |
| [layout/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine/layout/) | Pasta | Módulo responsável pelo cálculo geométrico e posicionamento dos elementos. Utiliza a biblioteca Taffy para Grid/Flexbox e implementa algoritmos de inline layout e fluxo de floats. |
| [graphics/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine/graphics/) | Pasta | Módulo de renderização visual. Contém o compositor GPU (utilizando WGPU/WGSL), rasterizador CPU (Tiny-Skia) para Canvas 2D/SVG, sistema de fontes (cosmic-text) e a árvore de camadas de renderização. |
| [mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine/mod.rs) | Arquivo | O orquestrador central (`AceEngine`). Coordena o ciclo de vida completo da engine de navegação: carrega URLs, aplica CSS de forma reativa, dispara o cálculo de layout e composição de camadas, e gerencia estados (hover, active, focus). |
| [layout_types.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine/layout_types.rs) | Arquivo | Define tipos e estruturas compartilhadas para posicionamento e desenho, como `DisplayItem` (primitiva de desenho de caixa/texto enviada ao compositor), `FloatRect` e `FloatContext` para layouts flutuantes. |

---

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

Para manter a codebase limpa, modular e escalável, siga rigorosamente as seguintes diretrizes de design ao estender ou modificar esta pasta:

### 🟢 O que DEVE estar aqui:
- **Estruturas de dados do documento:** Qualquer representação interna de elementos HTML, atributos e estilos CSS computados.
- **Algoritmos de layout puramente geométricos:** Cálculos de caixas, medições de texto, resolução de fluxo CSS (como floats, grid, flexbox).
- **Lógica de composição e desenho:** Shaders de composição de camadas, rasterização de primitivas geométricas básicas (bordas, fundos, imagens) e texto.
- **Orquestração de ciclos de renderização:** Controle de *dirty flags* de estilo e layout, e sincronização de atualizações de tela.

### 🔴 O que NÃO DEVE estar aqui:
- **Lógica de Interface do Usuário (UI):** Não insira elementos gráficos ou lógica do painel do navegador (como botões de voltar/avançar, barra de endereços, abas ou menus). Isso pertence à camada de aplicação principal (Ex: Slint UI).
- **Detalhes da Pilha de Rede:** A engine consome recursos e inicia downloads através de abstrações, mas a implementação de baixo nível de requisições HTTP, TLS ou gerenciamento de conexões TCP deve residir no módulo de rede dedicado (`crate::network`).
- **Implementação do interpretador JavaScript (Runtime JS):** A engine apenas orquestra bindings e eventos que ativam ações do JS, mas o motor JavaScript e a execução de scripts em si devem residir de forma isolada em `crate::ace::runtime`.
- **Preferências persistentes e histórico:** Dados como histórico de navegação, cookies persistentes e gerenciamento de sessões de usuário devem ser tratados fora da engine.
