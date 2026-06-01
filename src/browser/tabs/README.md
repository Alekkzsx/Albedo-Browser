# 📁 src/browser/tabs/

Esta pasta gerencia o ciclo de vida e o estado das abas do **Albedo Browser**.

## 🎯 Objetivo & Função
Este módulo implementa a abstração e gerenciamento de múltiplas instâncias de navegação. Cada aba no navegador hospeda sua própria instância da engine de renderização (`AceEngine`), seu próprio canal assíncrono de comunicação com o gestor de recursos de rede (`resource_rx`) e informações como estado de carregamento, favicons e URL ativa.

## 📄 Arquivos e Suas Funções

| Arquivo | Função / Propósito |
| :--- | :--- |
| [mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/mod.rs) | Módulo raiz que expõe a coleção e o gerenciador de abas para a aplicação. |
| [tab.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/tab.rs) | Contém a estrutura de dados `Tab`, que encapsula a engine de renderização associada a ela e os canais de recebimento de recursos assíncronos. |
| [collection.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/collection.rs) | Define a estrutura `TabCollection`, que armazena a lista física de abas ativas, rastreia qual aba está no foco visual do usuário e gerencia a inserção e fechamento de abas. |
| [manager.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/manager.rs) | Implementa o `TabManager`, o orquestrador principal das abas. Ele recebe os eventos físicos da janela Slint (cliques, movimentos, teclas, rolagem), identifica qual elemento na aba ativa foi clicado, e despacha esses eventos de input para o runtime JS da aba ou aciona comportamentos nativos (como toggle de `<details>` e fechamento de `<dialog>`). |

## 🛠️ Regras de Design (O que DEVE e NÃO DEVE estar aqui)

### O que DEVE estar aqui:
- Gerenciamento de vetores de abas, índices ativos e estado de progresso de carregamento de páginas.
- Tradução de coordenadas físicas do mouse (X, Y) do Slint para a aba ativa a fim de disparar detecção de elementos (Hit Testing).
- Orquestração e descarte de recursos de canal `UnboundedReceiver` quando uma aba é destruída.

### O que NÃO DEVE estar aqui:
- Lógica de desenho de interface da barra de abas Slint (deve estar em `src/ui/components/tab_bar.slint`).
- Lógica interna de layout ou parsing de páginas (deve estar na engine ACE em `src/ace/`).
- Protocolos de transporte de rede (devem estar em `src/network/`).
