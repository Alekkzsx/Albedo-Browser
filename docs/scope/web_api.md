# 🌐 Catálogo Normativo de Web APIs no MVP (v1.0)

> **Documento Normativo de Escopo de APIs Web**  
> **Subsistemas Alvo:** `Albedo_Core_Engine/ace_dom` e `Albedo_Core_Engine/ace_js`  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.1](../../PLANO.md) — §1.4, Fase 5 Concluída e Fase 13)  
> **Última Atualização:** 2026-10-12  
> **Padrões Normativos:** WHATWG DOM Living Standard, WHATWG HTML Living Standard, WHATWG Fetch, W3C WebIDL.

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.0.0** | 2026-10-12 | Alinhamento com Capacidades Reais do `ace_dom` | • Registro formal dos bindings WebIDL funcionais no `ace_dom` (`DocumentBindings`, `ElementBindings`, `NodeBindings`).<br>• Incorporação da infraestrutura de GC Tracing via `DOMDataStore` para integração com o futuro `ace_js` (ADR-0004 e ADR-0007).<br>• Registro do motor de seletores CSS4 acelerado por Ancestor Bloom Filter de 64 buckets. | • Escopo de Web APIs 100% ancorado nas estruturas reais do motor DOM.<br>• Suporte a Declarative Shadow DOM e streaming consolidado. |
| **1.0.0** | 2026-10-07 | Catalogação Normativa v7.0 | • Definição do conjunto mínimo de APIs para execução fluida de SPAs modernas (React, Vue, Svelte). | • Catálogo inicial de APIs de DOM, eventos, timers e rede. |

---

## 1. Diretriz de Implementação

Para suportar Single Page Applications (SPAs) modernas e frameworks web populares (como React, Vue e Svelte em modo cliente), o Albedo Browser disponibiliza uma camada estruturada de bindings WebIDL em `ace_dom` que conecta a árvore de nós em Arena geracional à máquina virtual do `ace_js`.

> [!NOTE]
> **Gestão de Memória e GC Tracing:** O gerenciamento do ciclo de vida dos nós conta com a infraestrutura do `DOMDataStore` em `ace_dom`, provendo rastreamento de liveness e pontos de controle para o futuro coletor de lixo (GC Bridge) do `ace_js` (ADR-0004 e ADR-0007), impedindo referências pendentes (*dangling references*).

---

## 2. Catálogo de APIs Disponíveis no MVP

### 2.1 DOM Core & Manipulação de Árvore
- **Document & Node:**
  - `document.getElementById()`, `document.getElementsByTagName()`, `document.getElementsByClassName()`
  - `document.querySelector()`, `document.querySelectorAll()` (com suporte a seletores CSS4 `:is()`, `:where()`, `:has()` e aceleração por Ancestor Bloom Filter)
  - `document.createElement()`, `document.createElementNS()`, `document.createTextNode()`, `document.createComment()`
  - `Node.appendChild()`, `Node.removeChild()`, `Node.replaceChild()`, `Node.insertBefore()`
  - `Node.cloneNode()`, `Node.contains()`, `Node.compareDocumentPosition()`
  - `Node.parentNode`, `Node.parentElement`, `Node.childNodes`, `Node.firstChild`, `Node.lastChild`, `Node.nextSibling`, `Node.previousSibling`
- **Element & Attributes:**
  - `Element.getAttribute()`, `Element.setAttribute()`, `Element.removeAttribute()`, `Element.hasAttribute()`
  - `Element.id`, `Element.className`, `Element.classList` (`add`, `remove`, `toggle`, `contains`)
  - `Element.innerHTML`, `Element.outerHTML`, `Element.textContent`
  - `HTMLElement.style` (leitura e mutação de propriedades CSS inline em tempo de execução via `StyleEngine`)
  - Medição de geometria: `Element.getBoundingClientRect()`, `Element.clientWidth`, `Element.clientHeight`, `Element.scrollWidth`, `Element.scrollHeight`, `Element.scrollTop`, `Element.scrollLeft` (integrado aos tipos `euclid`/`LayoutUnit` da Fase 7)

### 2.2 Sistema de Eventos (EventTarget)
- **Despacho e Escuta:**
  - `addEventListener(type, listener, options)`, `removeEventListener(type, listener, options)`, `dispatchEvent(event)`
  - Fases de propagação: Captura (`capture`), Alvo (`target`) e Borbulhamento (`bubble`).
  - Métodos de controle: `event.stopPropagation()`, `event.stopImmediatePropagation()`, `event.preventDefault()`.
- **Eventos Fundamentais:**
  - Teclado: `keydown`, `keyup`, `keypress`
  - Ponteiro/Mouse: `click`, `dblclick`, `mousedown`, `mouseup`, `mousemove`, `mouseenter`, `mouseleave`
  - Foco e Formulários: `focus`, `blur`, `input`, `change`, `submit`, `reset`
  - Ciclo de Vida do Documento: `DOMContentLoaded`, `load`, `unload`, `beforeunload`

### 2.3 Temporizadores e Agendamento no Event Loop
- `setTimeout()`, `clearTimeout()`
- `setInterval()`, `clearInterval()`
- `requestAnimationFrame()`, `cancelAnimationFrame()`
- `queueMicrotask()` (inserção direta na fila de microtarefas do Event Loop WHATWG de `ace_core`)

### 2.4 Rede e Comunicação Assíncrona
- **Fetch API (`ace_net`):**
  - `fetch(input, init)` suportando requisições assíncronas GET, POST, PUT, DELETE.
  - Objetos `Request`, `Response`, `Headers`.
  - Suporte a `AbortController` e `AbortSignal` para cancelamento de requisições em voo.
- **Streams:**
  - `ReadableStream` básico para consumo de respostas sob backpressure e integração ao `StreamingHtmlParser`.
- **Dados e URLs:**
  - `URL` e `URLSearchParams` (parse e mutação normativa de query strings).
  - `FormData` para submissão de formulários multipart.

### 2.5 APIs Críticas para Single Page Applications (SPAs)
- **History API & Navegação:**
  - `history.pushState()`, `history.replaceState()`, `history.back()`, `history.forward()`, `history.go()`
  - Evento `popstate` e `hashchange` no objeto `window`.
  - Propriedades de leitura: `window.location` (`href`, `origin`, `protocol`, `host`, `pathname`, `search`, `hash`).
- **Observabilidade de Árvore e Layout:**
  - `MutationObserver` (notificação de mutações estruturais, adição/remoção de nós e alteração de atributos integrada ao `RestyleDriver`).
  - `Live Range` (fronteiras dinâmicas de seleção com ajuste automático em mutações).
  - `ResizeObserver` (notificação de alterações dimensionais de caixas).
- **Web Workers:**
  - `new Worker(url)` executando scripts isolados em thread própria via `ace_js`.
  - Troca de mensagens via `postMessage()` e evento `onmessage`.
  - `MessageChannel` e `MessagePort` para canais bidirecionais.
  - Algoritmo de clonagem estruturada (`structuredClone()`).

### 2.6 Persistência no Navegador (Fase 11)
- `localStorage` e `sessionStorage` (APIs de armazenamento síncrono particionadas por origem).
- `document.cookie` (leitura e escrita com enforcement de políticas de segurança e particionamento CHIPS via `ace_net`).

### 2.7 Objetos Globais e Utilitários
- `window` e `document` como objetos de escopo global.
- `console` (`log`, `warn`, `error`, `info`, `trace`, `assert`, `time`, `timeEnd`).
- `atob()`, `btoa()` (codificação e decodificação base64).
- `crypto.getRandomValues()`, `crypto.randomUUID()` integrados a geradores criptográficos seguros (`CSPRNG`).

---

## 3. APIs Explicitamente Desativadas no MVP
Qualquer tentativa de acesso a estas APIs emitirá um aviso no console e lançará `NotSupportedError`:
- WebGL 1.0/2.0 e WebGPU para scripts da página.
- WebAudio API e MediaStream.
- WebRTC (`RTCPeerConnection`).
- Web Bluetooth, Web USB, Web Serial e sensores nativos de hardware.

