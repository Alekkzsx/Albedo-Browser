# 📁 src/

Esta pasta é o diretório de código-fonte principal do **Albedo Browser** (excluindo a subcrate isolada do compilador JIT `albedo-jit`).

## 🎯 Objetivo & Função
Ela organiza os componentes fundamentais do navegador, separando as responsabilidades de análise sintática e engine (ACE), protocolo de rede (ACE-Net), interface gráfica declarativa (Slint UI), rasterizador visual (Renderer) e o próprio shell do browser que orquestra e gerencia as sessões e interações de abas.

## 📄 Estrutura de Diretórios e Submódulos

| Diretório / Arquivo | Função / Propósito |
| :--- | :--- |
| [main.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/main.rs) | O ponto de entrada (*entry point*) do executável. Inicializa o runtime assíncrono Tokio, instancia a janela Slint principal, gerencia o `TabManager`, configura timers do sistema e de pulso gráfico e inicia o loop de eventos da UI. |
| [lib.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/lib.rs) | Declara os submódulos da biblioteca compartilhada. |
| [ace/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace) | **Albedo Core Engine**: O coração técnico. Contém o analisador de HTML (WHATWG compliant), árvore DOM, CSS engine (cascade e matching), motor de Layout (Taffy), e o runtime/bindings de Javascript. |
| [browser/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser) | **Browser Shell**: Camada de orquestração do navegador. Contém a gerência do ciclo de vida das abas (`TabManager`), gerenciador de eventos de input (clique, hover, teclado) e sincronia de frames (`ui/bridge.rs`). |
| [network/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network) | **ACE-Net**: Camada de networking de alto desempenho. Implementa requisições HTTP/3 (QUIC), pools de sockets HTTP/2/1.1, validações CORS/SOP e cache local criptografado e compactado. |
| [renderer/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/renderer) | **Rasterizador 2D**: Lógica gráfica de baixo nível. Desenha a Display List gerada pela engine ACE em Pixmaps de pixels brutos utilizando `tiny-skia` e formatação de texto com `cosmic-text`. |
| [ui/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui) | **Slint UI Layout**: Arquivos de interface do usuário (`.slint`) que descrevem visualmente o frame externo do browser, barra de endereços, controle de abas e página inicial. |

## 🛠️ Regras de Design Geral do Projeto (Design Constraints)
- **Separação de Preocupações (Separation of Concerns)**: Cada submódulo tem uma responsabilidade estrita. O motor visual `renderer` não sabe nada de rede; o módulo de rede `network` não sabe nada de DOM; e a UI declarativa não manipula ou executa estados (apenas delega comandos para o shell `browser`).
- **Tokio & Threads**: A UI (Slint) roda na thread principal do sistema operacional devido a requisitos gráficos. Qualquer operação de I/O de rede (`network`) ou decodificação deve ser despachada para threads assíncronas do pool do Tokio, retornando as respostas por canais `mpsc::channel` sem bloquear os frames da interface.
- **Estruturas de Dados Internas**: Use string interning (`lasso`) e arena allocation (`bumpalo`) na árvore DOM e no parser para reduzir o gargalo de alocações na heap.
