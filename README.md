<div align="center">
  <img src="https://raw.githubusercontent.com/Alekkzsx/Albedo-Browser/main/.github/assets/logo.png"
       alt="Albedo Browser Logo" width="200" height="200"
       onerror="this.style.display='none'">

  <h1>Albedo Browser & Engine</h1>

  <p><b>Um motor de navegador de nova geração, escrito em Rust, do zero — a alma é nossa, a fundação é compartilhada.</b></p>

  <img src="https://img.shields.io/badge/Rust-1.85%2B_(Edition_2024)-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/CI-Enterprise_Grade-success.svg" alt="CI Enterprise Grade">
  <img src="https://img.shields.io/badge/Status-Fase_2_(Core_Engine)-yellow.svg" alt="Status">
  <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License MIT">
</div>

<br>

---

## 📜 Manifesto de Engenharia

O ecossistema web atual é dominado por monopólios arquiteturais. Engines gigantescas como Chromium (Blink) e Firefox (Gecko) carregam décadas de código legado, decisões arquiteturais antigas (single-process loops adaptados) e milhões de linhas de C++ que dificultam inovações radicais em segurança e performance.

O resultado? Navegadores que consomem gigabytes de RAM apenas para renderizar texto e caixas coloridas.

O **Albedo Browser** não é um fork, nem um wrapper, nem um embed. Não há Chromium, Blink, WebKit, Gecko, Servo, V8, CEF, Electron ou WebView2 sob o capô. O **Albedo Engine (ACE)** é a nossa própria fundação — a árvore DOM/CSSOM, a cascata de estilos, os algoritmos de layout (Flex/Grid), o pipeline de pintura e o motor JavaScript são escritos por nós.

Nosso objetivo não é ser mais um navegador de nicho, mas construir um motor com capacidade técnica, segurança e performance para **rivalizar diretamente com os monopólios**, devolvendo diversidade tecnológica à web.

Este projeto responde a uma pergunta simples:

> **Como seria um navegador web se ele fosse projetado hoje, do zero, em Rust, orquestrando as melhores crates do ecossistema e focando estritamente em processamento assíncrono e paralelismo massivo?**

---

## 🧭 O que o Albedo é — e o que ele nunca será

| ✅ O que o Albedo **É** | ❌ O que o Albedo **NUNCA será** |
| --- | --- |
| Um motor de renderização escrito por nós, em Rust | Um fork de Blink, WebKit ou Gecko |
| Um motor JavaScript próprio (`ACE JS`) | Um embed de V8, SpiderMonkey, JavaScriptCore, QuickJS ou Boa |
| Uma arquitetura multi-processo nativa | Um wrapper de CEF, Electron ou WebView2 |
| Um compositor paralelo com foco em GPU | Uma webview de sistema (WKWebView, Wry/Tauri) |
| Um orquestrador pragmático de crates maduras | Uma engine pronta "bala de prata" (ex: Servo original) |

> **Não sofremos de "Not Invented Here".** Focamos a genialidade do time naquilo que define um navegador — renderização, layout e UI — e não em reescrever criptografia e event loops já perfeitamente resolvidos pela comunidade.

---

## ⚖️ A Fronteira Pragmática

O Albedo estabelece uma fronteira clara e inegociável entre o que **orquestramos** e o que **forjamos**. Esta é a alma do Paradigma Pragmático.

### 🧱 As Fundações — crates maduras e auditadas (usamos com orgulho)

Infraestrutura "chata, perigosa ou resolvida" é delegada ao melhor do ecossistema Rust. Herdar essa segurança é **obrigatório**.

| Domínio | Crates |
| --- | --- |
| **Async & Concorrência** | `tokio`, `rayon`, `crossbeam`, `mio` |
| **Rede & Segurança** | `rustls`, `hyper`, `reqwest`, `url` |
| **Tipografia & Unicode** | `icu4x`, `harfbuzz`, `rustybuzz` |
| **Gráficos & Mídia** | `wgpu` (Vulkan / Metal / D3D12), decodificadores de imagem da comunidade |
| **Estruturas de Dados** | HashMaps seguros e estruturas lock-free validadas com `loom` |

### 🔨 A Alma — 100% construída do zero por nós

É aqui que vive o diferencial competitivo do ACE. Nada disso é terceirizado.

- **DOM & CSSOM** — Tokenizer HTML5 (spec WHATWG) e parser CSS próprios, com árvore imutável e segura.
- **Motor de Estilos** — Cascata, especificidade, valores computados, media/feature queries, custom properties e `@layer`.
- **Geometry Engine (Layout)** — Box Model, BFC/IFC, margin collapsing, Flexbox e CSS Grid, massivamente paralelos.
- **Pipeline de Pintura** — Display list, rasterização e compositing orientados a GPU via `wgpu`.
- **ACE JS** — Motor JavaScript próprio: lexer, parser, bytecode, VM, GC geracional e JIT em tiers.
- **Multi-processo & Sandbox** — Isolamento por processo, IPC binário próprio e políticas de segurança nativas.
- **Browser Chrome** — Janela nativa, abas, omnibox e DevTools construídos sobre o próprio motor (*dogfooding*).

### 🚫 Estritamente Proibido ("Mata-Projetos")

- **Motores de renderização inteiros:** Blink, WebKit, Gecko, Servo.
- **Motores JavaScript prontos:** V8, SpiderMonkey, JavaScriptCore, QuickJS, Boa.
- **Encapsuladores / webviews:** CEF, Electron, WebView2, WKWebView, Wry, Tauri.
- **Engines opinativas de layout/DOM:** qualquer crate que resolva a cascata CSS, o layout web ou a árvore DOM completa por nós. Nós implementamos a lógica W3C.

---

## 🏛️ Arquitetura — O Ecossistema ACE

Todas as crates internas vivem sob o guarda-chuva **ACE (Albedo Core Engine)**, prefixadas com `ace_`, organizadas em uma pirâmide de camadas onde cada camada depende apenas das imediatamente inferiores — baixo acoplamento, alta coesão.

| Camada | Crate | Responsabilidade |
| --- | --- | --- |
| Foundation | `ace_core` | Tipos fundamentais (`AceError`, IDs), matemática 2D/3D, Event Loop, Thread Pool, logging |
| Communication | `ace_ipc` | Protocolo binário próprio, serialização, canais |
| Networking | `ace_net` | DNS, HTTP, pool de conexões, cache, resource fetcher |
| Security | `ace_core` + `ace_net` | URL parser, SOP, CORS, CSP, cookie jar |
| Parsing | `ace_dom`, `ace_style` | Tokenizers e parsers HTML5 / CSS3 |
| Styling | `ace_style` | Cascata, especificidade, valores computados |
| Layout | `ace_layout` | Box model, BFC/IFC, Flexbox, Grid, positioning |
| Media | `ace_media` | Decodificação de imagens |
| Rendering | `ace_render` | Display list, rasterização, tipografia, compositing |
| JavaScript | `ace_js` | Lexer, parser, bytecode, VM, GC, JIT |
| Persistence | `ace_storage` | Cookies, LocalStorage, KV store, histórico |
| Application | `ace_browser` | Binário final: janela, chrome, abas, DevTools |

A comunicação entre camadas acontece por interfaces públicas bem definidas e um tipo de erro unificado (`AceError`), sem dependências cruzadas indevidas.

---

## 🧠 Metodologia de Engenharia Tridimensional (3D)

Cada decisão, tarefa e validação é mapeada em três eixos ortogonais:

- **Eixo X — Integração Horizontal:** pureza das fronteiras entre crates (ex: `ace_layout` nunca chama `ace_net` diretamente).
- **Eixo Y — Profundidade Vertical:** conformidade matemática com as especificações (WHATWG, W3C, ECMA).
- **Eixo Z — Horizonte Temporal:** as 14 fases de entrega, cada uma atravessando X e Y com valor funcional.

O roadmap técnico exaustivo — sub-milestones, performance budgets, riscos e DoD de cada fase — vive em [`PLANO.md`](./PLANO.md).

---

## 🗺️ Roadmap (visão macro)

| Fase | Escopo | Status |
| --- | --- | --- |
| 1 | Fundação & Governança | ✅ Concluída |
| 2 | Core Engine, Infraestrutura & Matemática | 🚧 Em andamento |
| 3 | Motor de Rede & TLS | ⏳ Planejada |
| 4 | Sandboxing Nativo & Políticas Web | ⏳ Planejada |
| 5 | Parsing (DOM & CSSOM) | ⏳ Planejada |
| 6 | Render Tree & Motor de Estilos | ⏳ Planejada |
| 7 | Geometry Engine (Layout) | ⏳ Planejada |
| 8 | Pintura, Rasterização & Tipografia | ⏳ Planejada |
| 9 | Janela Nativa & Browser Chrome | ⏳ Planejada |
| 10 | Motor JavaScript (`ACE JS`) | ⏳ Planejada |
| 11 | Armazenamento & Persistência | ⏳ Planejada |
| 12 | Multi-Processo & Isolamento | ⏳ Planejada |
| 13 | Web APIs Modernas & SPAs | ⏳ Planejada |
| 14 | Otimizações, Conformidade & Evolução | ⏳ Planejada |

---

## 🎯 Princípios de Engenharia

- **Memory Safety First** — Rust mitiga as categorias clássicas de vulnerabilidades da web (buffer overflow, use-after-free).
- **Modularidade via Crates** — cada componente é isolado em bibliotecas bem delimitadas, garantindo reuso e manutenção desacoplada.
- **Zero-Cost Abstractions** — a ponte entre DOM e renderizador evita alocações desnecessárias usando lifetimes e borrowing estrito.
- **Fearless Concurrency** — layout e pintura tiram proveito do modelo de concorrência do Rust para paralelismo massivo.
- **Conformidade como Teste** — TDD guiado pela suíte *Web Platform Tests* (WPT) desde o dia 1 de parsing.
- **Governança Contínua** — toda decisão arquitetural relevante vira um ADR versionado em `docs/adr/`.

---

## 🚀 Quick Start

> O projeto está na **Fase 2 (Core Engine)**. Instruções completas de build, teste e debug chegam com o amadurecimento do workspace — veja `CONTRIBUTING.md`.

```bash
# Clone o repositório
git clone https://github.com/Alekkzsx/Albedo-Browser.git
cd Albedo-Browser

# Instale as ferramentas de desenvolvimento
cargo xtask setup

# Build do workspace
cargo build --workspace

# Testes (gate obrigatório de CI)
cargo test --workspace

# Lint sem exceções
cargo clippy -- -D warnings