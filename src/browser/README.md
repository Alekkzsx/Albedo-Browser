# 📁 browser

Esta pasta funciona como o **Shell do Navegador** (Browser Shell), atuando como a camada de orquestração central que gerencia o estado geral da aplicação (janelas, coleções de abas, navegação), despacha eventos do sistema e do usuário para o motor de renderização (Engine) e faz a ponte (bridge) com a interface de usuário construída em Slint.

---

## 🎯 Objetivo & Função

Na arquitetura do Albedo-Browser, o módulo `browser` é o "cérebro" executivo da interface e do ciclo de vida das abas. Ele não renderiza pixels diretamente (responsabilidade do `renderer`) nem interpreta o HTML/CSS/JS (responsabilidade da `ace` engine), mas gerencia a comunicação entre todos esses subsistemas:

1. **Ciclo de Vida das Abas**: Criação, alternância, carregamento e encerramento de abas via `TabManager` e `TabCollection`.
2. **Orquestração de Eventos**: Captura ações do usuário na interface (cliques, movimentos do mouse, teclas pressionadas, rolagem) e as encaminha para a instância da Engine associada à aba ativa.
3. **Ponte de Renderização (UI Bridge)**: Sincroniza a árvore de layout produzida pela Engine com o framebuffer exibido na interface Slint (`AppWindow`), gerenciando invalidações parciais (dirty rects) e selecionando entre compositor GPU (WGPU) ou rasterizador CPU (Tiny-Skia).
4. **Ciclo de Loop ("Pulse")**: Executa tarefas recorrentes a cada 16ms (60 FPS) para atualizar animações, processar microtarefas do runtime JavaScript, lidar com requisições de rede assíncronas e recalcular layouts devido a mutações no DOM.

---

## 📄 Arquivos e Suas Funções

Abaixo estão descritos os arquivos contidos neste diretório e a finalidade de cada um na arquitetura:

| Arquivo/Caminho | Função Principal | Descrição Detalhada |
| :--- | :--- | :--- |
| [mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/mod.rs) | Ponto de entrada do módulo | Expõe os submódulos públicos `tabs`, `ui` e `events`, além de reexportar funções essenciais de configuração e ponte. |
| [events.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/events.rs) | Despachante e manipulador de eventos | Converte interações do Slint (ex: navegação, cliques, foco, rolagem) em comandos para a Engine. Contém também o `handle_pulse` que sincroniza o estado a cada frame e monitora os recursos do sistema. |
| [tabs/mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/mod.rs) | Entrada do submódulo de abas | Organiza e expõe a coleção, o gerenciador e o modelo individual de aba. |
| [tabs/tab.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/tab.rs) | Modelo de dados de uma Aba (`Tab`) | Define a estrutura `Tab`, que armazena seu ID único, título, URL atual, estado de carregamento, ícone (favicon), canal de recebimento de rede (`resource_rx`) e sua própria instância isolada da `AceEngine`. |
| [tabs/collection.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/collection.rs) | Coleção de abas (`TabCollection`) | Gerencia a lista ordenada de abas ativas, controlando a inclusão, remoção e a aba que detém o foco no momento. |
| [tabs/manager.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/tabs/manager.rs) | Gerenciador de Abas (`TabManager`) | Coordena as operações de alto nível sobre as abas. Fornece comportamentos nativos do navegador (como processamento de formulários dialog e toggle de `<details>`), além de delegar eventos de teclado, scroll, clique e hover para a aba ativa. |
| [ui/mod.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/ui/mod.rs) | Entrada do submódulo de UI | Expõe as utilidades de inicialização da janela e a ponte com a interface. |
| [ui/bridge.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/ui/bridge.rs) | Ponte de Renderização Slint/Engine | Traduz os comandos de renderização gerados pela Engine para a imagem Slint. Coordena o pipeline assíncrono que tenta usar o compositor GPU (WGPU) e faz fallback para CPU (Tiny-Skia), aplicando otimizações de invalidadores parciais. |
| [ui/setup.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/browser/ui/setup.rs) | Inicialização da Interface | Cria a instância do `AppWindow` do Slint e registra o gancho global de pânico (panic hook) para tratamento de erros graves. |

---

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

Para manter a separação de conceitos limpa (Separation of Concerns) e garantir que o navegador continue modular e testável, siga as regras de design abaixo:

### 🟢 O que DEVE estar aqui
* **Orquestração de ciclo de vida das abas**: Qualquer código que envolva abrir, fechar ou alternar abas, ou associar canais de rede às abas.
* **Mapeamento de eventos Slint $\rightarrow$ Engine**: Despacho de eventos como cliques em posições de tela `(x, y)` mapeadas para nós da árvore DOM da Engine.
* **Sincronização de propriedades Slint**: Atualização de propriedades do `AppWindow` (ex: URL atual, progresso de carregamento, dados de imagem renderizada).
* **Tratamento de fallback de renderização**: Código que decide e executa a rasterização da página e atualiza o buffer do componente Slint.
* **Comportamentos nativos simples do navegador**: Regras de navegação padrão (ex: se o texto digitado não é uma URL, transformar em busca no Google) e interações nativas sem JavaScript (ex: comportamento padrão das tags `<details>`/`<summary>`).

### 🔴 O que NÃO DEVE estar aqui
* **Lógica interna de renderização e desenho (Painting/Rasterização)**: O desenho real de formas geométricas, textos e bordas deve viver exclusivamente em `src/renderer/`. Este módulo apenas chama os renderizadores apropriados.
* **Parsing de HTML/CSS ou interpretação de JS**: Toda a lógica de parseamento, construção de DOM/CSSOM e execução de scripts pertence estritamente ao diretório `src/ace/`.
* **Requisições de Rede Diretas (Networking)**: Este shell não deve fazer chamadas HTTP brutas. Ele deve delegar a obtenção de recursos para o `src/network/` através do `ResourceManager`.
* **Componentes de UI Slint Hardcoded**: O layout visual e declarações de estilo da interface do usuário (ex: barra de ferramentas, botões de fechar) devem viver nos arquivos `.slint` correspondentes (em `ui/`) e não serem embutidos em código Rust dentro deste diretório.

---

> [!IMPORTANT]
> A sincronização de desenho executada no `ui/bridge.rs` roda de forma assíncrona usando `tokio::spawn` para não bloquear a thread principal da interface de usuário do Slint. É crucial garantir que dados compartilhados sejam protegidos por travas de exclusão mútua (`Arc`, `Mutex` ou `lock().unwrap()`) adequadas e que o acesso seja feito de forma rápida para evitar gargalos de frame (Frame Drops).

> [!TIP]
> Ao adicionar novos comportamentos de atalho de teclado globais, certifique-se de adicioná-los no `events.rs` através de `handle_key_down` ou integrando com o callback do Slint em `main.rs`, evitando embutir lógica de manipulação direta de teclas dentro do motor `AceEngine`.
