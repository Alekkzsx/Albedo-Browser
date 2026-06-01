# 📁 Runtime JavaScript & Bindings (`src/ace/runtime/`)

Este diretório contém a infraestrutura de execução de JavaScript e bindings de APIs Web para o Albedo Browser. Ele funciona como a ponte de comunicação entre o interpretador JavaScript e o motor de renderização/layout nativo em Rust.

A engine de JavaScript é baseada no **QuickJS**, utilizando a biblioteca [rquickjs](https://github.com/delan/rquickjs) para fornecer um interpretador leve, de inicialização rápida e altamente integrável com Rust, estendido por um compilador JIT customizado (**AlbedoJIT**).

---

## 🎯 Objetivo & Função

O principal objetivo deste módulo é gerenciar o ciclo de vida da execução de scripts e módulos ES (ECMAScript) nas páginas web e Service Workers. Ele atua nas seguintes frentes:
1. **Isolamento de Contexto (Sandboxing)**: Garante que cada aba ou frame execute em seu próprio contexto JavaScript isolado com regras de segurança rígidas.
2. **Ciclo de Eventos (Event Loop)**: Orquestra tarefas assíncronas, microtasks, timers, animações (`requestAnimationFrame`) e tarefas em segundo plano de forma cooperativa.
3. **Bindings da Web API & DOM**: Expõe objetos nativos do Rust (como nós da árvore DOM, consoles e recursos de rede) de maneira segura e ergonômica para o JavaScript.
4. **Intercepção JIT & OSR (Bridge)**: Intercepta a interpretação padrão do QuickJS para migrar a execução de loops quentes para código de máquina nativo e otimizado.

---

## 📂 Organização da Pasta

A arquitetura do runtime é dividida em três subdiretórios principais:

*   **[core/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core)**: O motor de execução principal do JavaScript, contendo o wrapper do interpretador, o carregador de módulos ES, o loop de eventos e o despachante (executor) de tarefas.
*   **[bridge/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/bridge)**: A camada de instrumentação que conecta o interpretador QuickJS ao compilador JIT e gerencia a transição dinâmica (OSR) para execução nativa.
*   **[bindings/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/bindings)**: Os mapeamentos de APIs do navegador para o ambiente JS. Subdividido em:
    *   `html/`: Manipulação e observação de elementos DOM (MutationObserver, ResizeObserver, etc.).
    *   `webapi/`: APIs do navegador como fetch, IndexedDB, localStorage/sessionStorage e Service Workers.
    *   `utils/`: Auxiliares de console, shims e polyfills.

---

## 📄 Arquivos e Suas Funções

Abaixo estão descritos os principais arquivos deste diretório e suas responsabilidades:

### Núcleo de Execução (`core/`)

| Arquivo | Função |
| :--- | :--- |
| [runtime.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/runtime.rs) | Wrapper de segurança sobre o `rquickjs::Runtime` e `Context`. Controla o estado global do runtime de uma aba, histórico de navegação, despacho de eventos DOM/ponteiro e integração com o JIT Profiler. |
| [event_loop.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/event_loop.rs) | Mantém as filas de macro-tarefas, timers (`setTimeout`/`setInterval`), callbacks de animação (`requestAnimationFrame`), tarefas em segundo plano (`requestIdleCallback`) e canais de sincronização assíncronos. |
| [executor.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/executor.rs) | Implementa o motor de pulso (`run_pending`) que processa mensagens postMessage, executa microtasks/promessas, processa eventos do IndexedDB/Service Worker, executa timers expirados e despacha notificações de Resize/Intersection Observers. |
| [module_loader.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/module_loader.rs) | Implementa a resolução de caminhos (`Resolver`) e download/cacheamento (`Loader`) de módulos ES. Também oferece suporte completo a Import Maps (`<script type="importmap">`). |
| [init.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/init.rs) | Cria e configura um novo `JsRuntime` com todas as Web APIs registradas no escopo global (`window`/`self`), configura o localStorage persistente e inicializa os protótipos de eventos do DOM. |
| [eval.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/eval.rs) | Compila e avalia scripts normais e módulos ES. Contém os hooks para interceptação JIT nativa e tratamento/log detalhado de exceções do JavaScript com stack traces. |
| [sandbox.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/sandbox.rs) | Contém as flags de Sandbox para frames (ex: `allow-scripts`, `allow-same-origin`) e o analisador correspondente. |
| [registry.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/registry.rs) | Mantém um catálogo estático e global de todos os runtimes ativos, permitindo a comunicação entre frames (via `postMessage`). |
| [service_worker.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/service_worker.rs) | Gerenciador de ciclo de vida e roteamento de Service Workers ativos e enfileiramento de sincronização em segundo plano. |
| [sw_db.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/sw_db.rs) | Banco de dados SQLite persistente para armazenar metadados dos Service Workers registrados. |
| [intersection_registry.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/intersection_registry.rs) | Registro auxiliar para rastrear observers de interseção. |

### Ponte JIT (`bridge/`)

| Arquivo | Função |
| :--- | :--- |
| [quickjs_intercept.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/bridge/quickjs_intercept.rs) | Intercepta o interpretador QuickJS em ticks de execução periódicos (através de interrupt handler). Detecta loops quentes ("hot") e gerencia a migração OSR para o código JIT nativo. |

---

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

Para manter a separação de conceitos limpa e garantir a segurança do navegador, as seguintes diretrizes de design devem ser estritamente seguidas:

### O que DEVE estar aqui
*   **Códigos de bindings diretos com QuickJS**: Qualquer tipo Rust exposto para JS usando as macros `#[rquickjs::class]` ou `#[rquickjs::methods]`.
*   **Lógicas de ciclo de vida de scripts**: Inicialização de escopo global do JS, resolução e download síncrono/assíncrono de módulos JS, execução de micro e macro-tarefas da especificação HTML.
*   **Abstrações seguras do DOM para JS**: Estruturas de dados leves que guardam referências seguras à árvore DOM nativa (como `Element` contendo um índice numérico e um `Arc<Mutex<AceDOM>>`).
*   **Interceptores de máquina virtual**: Telemetria, profilação e hooks de interrupção que interagem com o interpretador JavaScript.

### O que NÃO DEVE estar aqui
*   **Lógica de renderização direta ou UI**: Não deve haver chamadas diretas para bibliotecas de UI (como Slint) ou manipulação de janelas. O runtime se comunica apenas com o `AceDOM` ou envia eventos por canais de mensagens.
*   **Manipulação de rede de baixo nível**: O download de recursos HTTP para páginas web deve ser delegado ao `ResourceManager` do módulo de rede, nunca instanciando clientes de rede de baixo nível diretamente nos bindings (com exceção do download de fontes de módulos síncronos no loader).
*   **Ponteiros brutos diretos compartilhados com o JS**: Nunca expor ponteiros de memória do Rust diretamente ao ecossistema JS para evitar vulnerabilidades de use-after-free. Em vez disso, use IDs ou registros indexados.
*   **Locks aninhados que bloqueiam o interpretador**: Evite manter travas de escrita ou leitura no DOM ou no Event Loop enquanto executa código JS (`ctx.eval` ou callbacks). O código JS pode durar muito ou disparar novos eventos, causando deadlocks.

---

## 🔒 Isolamento e Diretrizes de FFI

> [!IMPORTANT]
> A segurança e a estabilidade do Albedo dependem do isolamento estrito entre a engine JavaScript e o sistema operacional.

### 1. Isolamento de Origem (Same-Origin Policy)
- Antes de permitir chamadas cross-context (como envio de mensagens via `postMessage` ou leitura de propriedades de outros frames/janelas), o método `check_same_origin` da struct [runtime.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/runtime.rs) DEVE ser validado.
- Os dados do `localStorage` e `sessionStorage` são estritamente isolados por arquivos sanitizados com base na origem em [init.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/core/init.rs).

### 2. Gestão Segura de Memória e Referências DOM
- Para evitar que o Garbage Collector (GC) do QuickJS remova objetos que ainda são referenciados pelo Rust, use `rquickjs::Persistent` para guardar callbacks JS de longa duração (como em timers ou EventListeners).
- O Rust garante a segurança do DOM usando um modelo de **índices de nós** (`node_index`). As structs expostas como classes JavaScript (ex: `Element` e `Document`) nunca guardam referências `&mut` diretas a nós do DOM. Elas operam via IPC interno e manipulam a árvore clonando referências seguras `Arc<Mutex<AceDOM>>`.

### 3. FFI com QuickJS e JIT
- A integração de telemetria e OSR em [quickjs_intercept.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime/bridge/quickjs_intercept.rs) adota uma abordagem estritamente segura para evitar ler internals não mapeados da struct `JSContext` do QuickJS, que variam entre plataformas.
- Os registradores da máquina virtual no momento do OSR são alimentados com valores neutros (zeros ou indefinidos), assumindo a entrada limpa no cabeçalho do loop.
- O JIT Bridge garante que qualquer código de máquina gerado e executado dinamicamente (`unsafe` transmute para ponteiro de função) seja validado pelo `BytecodeRegistry` do AlbedoJIT antes da execução nativa.
