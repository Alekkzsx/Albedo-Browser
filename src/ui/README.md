# 📁 src/ui/

Esta pasta contém as definições visuais e de interface do usuário do **Albedo Browser** escritas em **Slint** (um framework declarativo acelerado por hardware).

## 🎯 Objetivo & Função
Este módulo define a moldura externa do navegador (o chamado *Browser Chrome*), incluindo a barra de abas, barra de endereços (URLs), botões de navegação, página inicial e o componente que hospeda o buffer gráfico da página renderizada. Ele serve como a camada de apresentação que recebe as interações do usuário (clique, teclado, scroll) e as despacha via callbacks para a lógica Rust em `src/browser/`.

## 📄 Arquivos e Suas Funções

| Arquivo/Pasta | Função / Propósito |
| :--- | :--- |
| [mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/mod.rs) | Macro que importa e compila as classes Slint geradas durante o build (`slint::include_modules!()`). |
| [appwindow.slint](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/appwindow.slint) | Arquivo principal da janela. Organiza o layout geral (TabBar -> NavBar -> StartPage/BrowserView) e expõe as propriedades de estado do navegador. |
| [components/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/components) | Pasta contendo os subcomponentes de UI isolados (botões, barra de abas, barra de navegação, página inicial). |

### 🧩 Componentes Individuais (`components/`)
*   [start_page.slint](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/components/start_page.slint): Página inicial (`albedo://start`) com barra de buscas integrada, atalhos rápidos e monitoramento em tempo real do sistema (RAM/CPU).
*   [browser_view.slint](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/components/browser_view.slint): Componente contendo a área Flickable que projeta o buffer gráfico da página da web e despacha eventos de input como cliques de mouse, movimentos e rolagem.
*   [tab_bar.slint](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/components/tab_bar.slint): Componente de abas dinâmicas, permitindo adicionar, fechar e alternar entre abas ativas.
*   [nav_bar.slint](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/components/nav_bar.slint): Barra de navegação que hospeda botões (Voltar, Avançar, Recarregar, Início) e a entrada de endereços com medidor de progresso.
*   [icon_button.slint](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ui/components/icon_button.slint): Widget reutilizável para ícones interativos.

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

### O que DEVE estar aqui:
- Código declarativo de estilo, layout, animações, estados visuais e posicionamento de elementos de UI.
- Mock de dados simples para visualização na IDE do Slint.
- Declaração de callbacks (`callback request_new_tab()`, etc.) para expor ações do usuário para o Rust.

### O que NÃO DEVE estar aqui:
- Lógica complexa de negócios ou decisões de estado (ex: validar regras CORS, resolver domínios de DNS, decidir qual aba fechar). A UI Slint apenas aciona o callback e o Rust decide o comportamento.
- Código de rede ou chamadas ao sistema operacional diretamente (devem ocorrer no backend Rust).
- Renderização direta de HTML ou processamento do DOM (a UI apenas recebe o buffer pronto como um objeto `image` do Rust).
