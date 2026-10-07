# 🎯 Escopo Normativo do MVP (v1.0) — Albedo Browser & Engine

> **Documento Normativo de Escopo**  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.0](../../PLANO.md) — §1.4)  
> **Última Atualização:** 2026-10-07  
> **Propósito:** Delimitar taxativamente a fronteira de entrega da versão 1.0 desktop (Windows, Linux, macOS), eliminando ambiguidades e evitando expansão descontrolada de escopo (*scope creep*).

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.0.0** | 2026-10-07 | Auditoria Forense v7.0 | • Adoção irrestrita do Paradigma Pragmático (Fundações vs Alma).<br>• Confirmação do motor JS interpretador sem JIT (ADR-0019).<br>• Inclusão formal de HTTP/2 e HTTP/3 no MVP (implementados em `ace_net`).<br>• Isolamento de processos por site (ADR-0015). | • Reescrita completa eliminando afirmações de "Zero-Dependency" e "DNS próprio em sockets".<br>• Inclusão de APIs essenciais de SPA (Workers, History API). |
| **1.0.0** | 2026-08-16 | Definição Inicial | • Definição preliminar do MVP "Abismo Restrito". | • Documentação das primeiras restrições da versão 1.0. |

---

## 1. Princípio Norteador: O Paradigma Pragmático

O Albedo Browser estabelece uma fronteira clara e inegociável entre o que **orquestramos** e o que **forjamos**:
- **Fundações (Ecossistema Rust Auditado):** Transporte e rede assíncrona (`tokio`, `hyper`, `quinn`), TLS e criptografia (`rustls`), resolução DNS (`hickory-resolver`), tipos geométricos (`euclid`), janelas e contexto gráfico (`winit`, `wgpu`), dados Unicode (`icu4x`).
- **Alma (100% Forjado pelo ACE):** Tokenizador e construtor de árvore HTML5, modelo de memória do DOM, motor de cascata e especificidade de CSS, cálculo de layout (Box Model, Flexbox, Grid), display list, compositor, motor JavaScript próprio (`ace_js`), protocolo IPC e isolamento de processos por site.

---

## 2. O Que ENTRA no MVP (v1.0 Desktop)

Para que a versão 1.0 seja considerada concluída (DoD Geral cumprido), o navegador deve renderizar corretamente e permitir navegação fluida em páginas de documentos reais (Wikipedia, documentações técnicas, portais de notícias, blogs e SPAs institucionais):

### 2.1 Rede, Conectividade e Segurança de Transporte
- **Protocolos:** Conexões seguras HTTPS sobre HTTP/1.1, HTTP/2 e HTTP/3 (QUIC) via `ace_net`.
- **Resolução DNS:** Suporte a DNS-over-HTTPS (DoH) com algoritmo Happy Eyeballs v2 (RFC 8305).
- **Cache HTTP RFC 9111:** Cache em memória (L1 RAM) e persistência transacional em disco com WAL (L2), com suporte a validação condicional (`304 Not Modified`, `ETag`, `If-Modified-Since`).
- **Segurança Web:**
  - Same-Origin Policy (SOP), isolamento de origem RFC 6454.
  - Cross-Origin Resource Sharing (CORS) e Content Security Policy Level 3 (CSP).
  - HTTP Strict Transport Security (HSTS) e Private Network Access (PNA).
  - Cookie Jar particionado com suporte a CHIPS (*Cookies Having Independent Partitioned State*).
  - Particionamento triplo de estado de rede via `NetworkIsolationKey`.

### 2.2 Parsing Web e Árvore DOM
- **HTML5 Parser:** Tokenizer streaming aderente ao WHATWG HTML Living Standard §12 com otimizações SIMD e recuperação resiliente de erros.
- **Tree Builder:** Construção da árvore DOM com o Adoption Agency Algorithm (AAA) de 16 passos.
- **Estrutura DOM:** Modelo de nós imutável/geracional (`Arena<NodeData>`), suporte a nós de elemento, texto e comentários.
- **Componentes Modernos:** Declarative Shadow DOM (DSD), elementos `<template>`, `MutationObserver` e `Live Ranges`.
- **Sanitização:** Defesa anti-XSS integrada via W3C HTML Sanitizer API.

### 2.3 Estilo e Layout (Acelerado e Paralelo)
- **Subconjunto CSS MVP:** Conforme definido taxativamente em [`css_properties.md`](./css_properties.md):
  - Modelos de formatação: Bloco (BFC), Linha (IFC), Flexbox e CSS Grid básico.
  - Box Model completo: margin, padding, border, dimensionamento (`box-sizing`).
  - Posicionamento: `static`, `relative`, `absolute`, `fixed`, `sticky`.
  - Cascata: Especificidade, herança, variáveis CSS (`var()`) e suporte a `@layer` (CSS Cascade 5).
- **Tipografia e Texto:** Shaping de texto completo (incluindo scripts bidirecionais RTL e CJK) orquestrando `harfrust`/`rustybuzz` e descoberta de fontes do sistema via `fontdb`.
- **Imagens:** Decodificação de PNG, JPEG, GIF e WebP em streaming.

### 2.4 Motor JavaScript (`ace_js`) — Modo Jitless Permanente
- **Arquitetura da VM:** Motor JavaScript próprio baseado em **interpretador de bytecode** (sem JIT na v1.0).
- **Segurança por Construção:** O modo interpretado (*jitless*) permanece como postura defensiva permanente de segurança (reduzindo a superfície de ataques de corrupção de memória).
- **Linguagem ES6+:** Execução de scripts modernos essenciais (Promises, `async`/`await`, microtask queue checkpoint, arrow functions, classes, ES Modules estáticos).
- **Event Loop WHATWG:** Fila de tarefas por fonte (`TaskSource`), timers (`setTimeout`, `setInterval`), `requestAnimationFrame`.
- **APIs de SPA Essenciais:** Conforme catálogo em [`web_api.md`](./web_api.md):
  - Fetch API (`fetch()`), Streams, `AbortController`.
  - Web Workers (execução em threads dedicadas).
  - History API (`pushState`, `replaceState`, evento `popstate`).
  - `ResizeObserver`, `structuredClone`, `MessageChannel`.

### 2.5 Interface do Usuário e Plataforma (Desktop)
- **Shell do Navegador:** Janela nativa via `winit` com renderização GPU via `wgpu`.
- **Componentes do Chrome:** Abas navegáveis, omnibox com barra de endereços/busca, histórico básico e botões de navegação (voltar/avançar/recarregar).
- **Acessibilidade (a11y):** Árvore de acessibilidade própria mapeando ARIA e integrada ao `AccessKit` (expondo para MSAA/UIA no Windows, NSAccessibility no macOS, AT-SPI no Linux).
- **DevTools Mínimo:** Console de execução e inspetor DOM básico para depuração.

### 2.6 Arquitetura de Processos
- Conteúdo web executado em processos de **Renderer isolados por site** (*Site Isolation*), separados do processo do Browser (Fase 12).

---

## 3. O Que FICA FORA do MVP (Pós-MVP / 🔁)

Os seguintes recursos são explicitamente excluídos da versão 1.0 para manter a viabilidade técnica:

1. **Compilação JIT (Just-In-Time):** Compiladores Tier-1 (Baseline) e Tier-2 (Otimizante) para x86_64/ARM64 ficam para a Fase 14.
2. **WebAssembly (Wasm):** Runtimes Wasm não entram na v1.0.
3. **Aceleração 3D para Conteúdo Web:** APIs WebGL e WebGPU expostas para páginas web (a GPU do Albedo no MVP é exclusiva para o compositor do navegador).
4. **Comunicação em Tempo Real (WebRTC):** Pilha ICE/STUN/TURN, canais de dados e streams de mídia ao vivo.
5. **Mídia Proprietária e DRM:** Proteção Widevine / Encrypted Media Extensions (EME) dependente de acordos comerciais com a Google.
6. **Codecs de Mídia Patenteados:** Decodificação de H.264/AAC sujeitos a royalties.
7. **Ecossistema de Extensões:** Compatibilidade com WebExtensions (Chrome/Firefox extensions).
8. **Plataformas Mobile:** Versões Android/iOS.
9. **Impressão Nativa:** Pipeline de paginação para PDF/impressora física.
