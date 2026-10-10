<div align="center">
  <img src="https://raw.githubusercontent.com/Alekkzsx/Albedo-Browser/main/.github/assets/logo.png"
       alt="Albedo Browser Logo" width="180" height="180"
       onerror="this.style.display='none'">

  <h1>Albedo Browser & Engine (ACE)</h1>

  <p><b>Um motor de navegador de nova geração, escrito em Rust, do zero — a alma é nossa, a fundação é compartilhada.</b></p>

  <p>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.80%2B_(Edition_2021)-orange.svg" alt="Rust 1.80+"></a>
    <img src="https://img.shields.io/badge/CI-Enterprise_Grade-success.svg" alt="CI Enterprise Grade">
    <img src="https://img.shields.io/badge/Memory_Safety-Miri_Audited-blue.svg" alt="Safe Rust">
    <a href="./LICENSE"><img src="https://img.shields.io/badge/License-Proprietary_(All_Rights_Reserved)-red.svg" alt="License Proprietary"></a>
  </p>
</div>

<br>

---

## 📜 Manifesto de Engenharia

O ecossistema web atual é dominado por monopólios arquiteturais. Engines gigantescas como Chromium (Blink) e Firefox (Gecko) carregam décadas de código legado em C++, loops de thread única adaptados e milhões de linhas que dificultam inovações radicais em segurança, escalabilidade e eficiência de memória. O resultado são navegadores que consomem gigabytes de RAM apenas para exibir conteúdo básico.

O **Albedo Browser** não é um fork, não é um wrapper e não é uma casca webview:
- **Zero Chromium / Blink / WebKit / Gecko** sob o capô.
- **Zero V8, SpiderMonkey, JavaScriptCore, QuickJS ou Boa** — o motor JavaScript (`ACE JS`) é forjado por nós.
- **Zero CEF, Electron, Tauri, Wry ou WebView2**.

O **Albedo Core Engine (ACE)** é construído para responder a uma pergunta de engenharia fundamental:

> **Como seria um navegador web se ele fosse projetado hoje, do zero, em Rust, aproveitando o poder de paralelismo massivo, garantias de memory safety e orquestrando as melhores primitivas assíncronas do ecossistema?**

---

## 🧭 O Que o Albedo É — e O Que Nunca Será

| ✅ O Que o Albedo **É** | ❌ O Que o Albedo **NUNCA Será** |
| :--- | :--- |
| Um motor de renderização moderno escrito em Rust | Um fork de Blink, WebKit, Gecko ou Servo |
| Um motor JavaScript próprio com VM de Bytecode e GC (`ACE JS`) | Uma incorporação de V8, SpiderMonkey ou engines prontas |
| Uma árvore DOM imutável em Arena Slotmap geracional (`ace_dom`) | Um wrapper de CEF, Electron, WebView2 ou Tauri |
| Um subsistema de rede assíncrono de alto desempenho (`ace_net`) | Uma webview de sistema (WKWebView, Wry) |
| Um compositor orientado a GPU sobre `wgpu` (`ace_render`) | Uma engine baseada em parsers "bala de prata" opinativos |

---

## ⚖️ O Paradigma Pragmático (A Regra de Ouro)

O projeto Albedo estabelece uma fronteira clara e rigorosa entre o que **orquestramos** e o que **forjamos**:

### 🧱 1. As Fundações (Crates consolidadas e auditadas do ecossistema)
Infraestrutura básica e transporte são delegados a bibliotecas maduras, testadas em batalha e auditadas:
- **Async & Concorrência:** `tokio`, `rayon`, `crossbeam`, `loom`
- **Rede & Transporte:** `hyper`, `rustls` (WebPKI), `hickory-resolver` (DNS), `url`
- **Tipografia & Unicode:** `icu4x`, `harfbuzz`, `rustybuzz`, `ttf-parser`
- **Gráficos & Janelas:** `wgpu` (Vulkan / Metal / D3D12), `winit`
- **Compressão & Hashing:** `zstd`, `brotli`, `flate2`, `sha2`

### 🔨 2. A Alma (100% forjado do zero pela engenharia do ACE)
O diferencial competitivo do navegador vive aqui e é inteiramente nosso:
- **DOM & CSSOM:** Tokenizer HTML5 (WHATWG §12) streaming com SIMD, Tree Builder com 16-step Adoption Agency Algorithm, CSSOM e seletor CSS4 com Ancestor Bloom Filter.
- **Motor de Estilos:** Cascata, especificidade, valores computados, herança, `@layer` e custom properties.
- **Geometry Engine (Layout):** Box Model, BFC/IFC, Flexbox e CSS Grid massivamente paralelos.
- **Pipeline de Pintura:** Display list, rasterização por CPU/GPU e compositing com camadas.
- **ACE JS:** Motor JavaScript próprio: lexer, parser, gerador de bytecode, VM registradores/pilha, GC geracional e JIT em tiers.
- **Segurança & Sandboxing:** Particionamento triplo de cache (`NetworkIsolationKey`), isolamento de processos, cookies CHIPS e SOP/CORS/CSP nativos.
- **Browser Chrome:** Janela nativa, abas, omnibox e DevTools (*dogfooding* sobre a própria engine).

> [!NOTE]
> Essa fronteira é auditada automaticamente no CI via [`deny.toml`](./deny.toml), que proíbe qualquer dependência não autorizada na árvore de pacotes.

---

## 🏛️ Arquitetura — O Ecossistema ACE

Todas as crates do motor residem no diretório [`Albedo_Core_Engine/`](./Albedo_Core_Engine) sob uma pirâmide de camadas rigorosa:

| Camada | Crate | Responsabilidade | Status |
| :--- | :--- | :--- | :---: |
| **Foundation** | [`ace_core`](./Albedo_Core_Engine/ace_core) | Tipos fundamentais (`AceError`), matemática 2D/3D, Event Loop WHATWG, Arena, Tri-color GC, buffers lock-free | 🟡 Em Andamento |
| **Parsing** | [`ace_dom`](./Albedo_Core_Engine/ace_dom) | Tokenizer HTML5 SIMD, Tree Builder (AAA), DOM imutável, seletores CSS4, MutationObserver, Live Ranges | 🟡 Em Andamento |
| **Networking** | [`ace_net`](./Albedo_Core_Engine/ace_net) | ResourceFetcher, Cache RFC 9111 (L1 RAM + L2 WAL), DoH Happy Eyeballs v2, Early Hints 103, Scheduler RFC 9218 | 🚧 Fechamento |
| **Security** | `ace_core` + `ace_net` | Origin, SOP, CORS, CSP, HSTS, PNA, Cookie Jar com suporte a CHIPS e isolamento triplo | 🟡 Em Andamento |
| **Styling** | `ace_style` | Cascata CSS3, especificidade, valores computados, herança e matching | ⏳ Planejado |
| **Layout** | `ace_layout` | Box Model, BFC/IFC, Flexbox, CSS Grid, margin collapsing e posicionamento | ⏳ Planejado |
| **Rendering** | `ace_render` | Display list, text layout com HarfBuzz/ICU4X, rasterização e compositing GPU via `wgpu` | ⏳ Planejado |
| **JavaScript** | `ace_js` | Motor JS nativo: Lexer, Parser, Bytecode VM, GC Geracional e JIT | ⏳ Planejado |
| **Persistence** | `ace_storage` | LocalStorage, IndexedDB / KV Store transacional, histórico e cookies persistentes | ⏳ Planejado |
| **Communication** | `ace_ipc` | Protocolo binário multi-processo de alta velocidade e canais tipados | ⏳ Planejado |
| **Application** | `ace_browser` | Binário executável: Janela Winit, UI do navegador, abas, omnibox e DevTools | ⏳ Planejado |

---

## 🗺️ Roadmap de Engenharia (Visão Macro)

O desenvolvimento segue as 14 fases mapeadas na **Metodologia de Engenharia Tridimensional (3D)**:

| Fase | Escopo | Status |
| :---: | :--- | :---: |
| **1** | Fundação, Governança & CI Enterprise | 🟡 Em Andamento |
| **2** | Core Engine, Infraestrutura & Fundação Matemática (`ace_core`) | 🟡 Em Andamento |
| **3** | Motor de Rede, TLS & Cache HTTP RFC 9111 (`ace_net`) | 🚧 Em Fechamento |
| **4** | Políticas Web e Segurança em Rede | 🟡 Em Andamento |
| **5** | Parsing Web, Tokenizador HTML5 & DOM Tree (`ace_dom`) | 🟡 Em Andamento |
| **6** | Render Tree & Motor de Estilos (`ace_style`) | ⏳ Planejada |
| **7** | Geometry Engine / Layout: Flexbox & Grid (`ace_layout`) | ⏳ Planejada |
| **8** | Pintura, Rasterização & Tipografia GPU (`ace_render`) | ⏳ Planejada |
| **9** | Janela Nativa & Browser Chrome (`ace_browser`) | ⏳ Planejada |
| **10** | Motor JavaScript Próprio (`ace_js`) | ⏳ Planejada |
| **11** | Armazenamento & Persistência Local (`ace_storage`) | ⏳ Planejada |
| **12** | Arquitetura Multi-Processo & IPC Binário (`ace_ipc`) | ⏳ Planejada |
| **13** | Web APIs Modernas & SPAs | ⏳ Planejada |
| **14** | Hardening de Conformidade WPT & Otimizações | ⏳ Planejada |

O roteiro técnico detalhado com cada milestone, estruturas de dados e performance budgets encontra-se no [**PLANO.md**](./PLANO.md).

---

## 📚 Base de Documentação

Toda a documentação técnica, especificações de subsistemas e relatórios estão centralizados em [`docs/`](./docs):

- 🗺️ [**PLANO.md**](./PLANO.md) — O Plano Mestre de Engenharia do navegador (Fonte da Verdade).
- 📑 [**Central de Documentação (`docs/README.md`)**](./docs/README.md) — Guia de navegação completo.
  - 🌐 [Arquitetura de Rede (`docs/architecture/ace_net.md`)](./docs/architecture/ace_net.md)
  - 🌳 [Arquitetura do DOM & Parsing (`docs/architecture/ace_dom.md`)](./docs/architecture/ace_dom.md)
  - 🧪 [Infraestrutura de Testes 4-Tier (`docs/architecture/test_infra.md`)](./docs/architecture/test_infra.md)
  - 🎯 [Escopo Normativo do MVP (`docs/scope/mvp_scope.md`)](./docs/scope/mvp_scope.md)
  - 🎨 [Propriedades CSS Suportadas (`docs/scope/css_properties.md`)](./docs/scope/css_properties.md)
  - 📊 [Relatórios de Testes e Auditorias (`docs/reports/`)](./docs/reports/)
  - 🏛️ [Architecture Decision Records (`docs/adr/`)](./docs/adr/)

---

## 🚀 Como Executar e Contribuir

### Pré-requisitos
- **Rust 1.80+ (Edition 2021)** via [rustup](https://rustup.rs/)

### Build e Testes

```bash
# Clone o repositório
git clone https://github.com/Alekkzsx/Albedo-Browser.git
cd Albedo-Browser

# Compilar todo o workspace
cargo build --workspace

# Executar a suíte de testes de todos os componentes
cargo test --workspace

# Validação rigorosa de linter (zero warnings tolerados)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## 📄 Licença

Este projeto é **Proprietário (Todos os Direitos Reservados)**. É estritamente proibida a cópia, modificação, distribuição ou comercialização de qualquer parte deste código sem autorização expressa. O uso não autorizado constitui plágio e violação de direitos autorais, passível de ação judicial. Veja o arquivo [LICENSE](./LICENSE) para os termos legais.