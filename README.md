# 🌑 Albedo Browser
> "Velocidade da luz em hardware comum. Soberania tecnológica em cada linha de código."

[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**Albedo** não é apenas um navegador; é um manifesto técnico contra a obesidade da web moderna. O Albedo rejeita a hegemonia do motor Blink/Chromium para construir seu próprio caminho: a **ACE (Albedo Core Engine)**.

## ⚡ A Filosofia Albedo
Hoje, um navegador consome 1GB de RAM para exibir uma página de texto. O Albedo quebra essa norma através de:
- **ACE Engine:** Motor de renderização próprio, orquestrado em Rust, usando as **melhores crates do ecossistema** como blocos de construção — assim como o Chrome usa Skia, zlib, BoringSSL e ICU internamente.
- **Engenharia Pragmática:** Usamos as melhores ferramentas disponíveis (`html5ever`, `cssparser`, `taffy`, `tiny-skia`, `wgpu`) e focamos nossa energia na **arquitetura, pipeline e integração** — que é onde o diferencial real mora.
- **Zero Bloat:** Sem telemetria, sem processos fantasmas e sem o lixo de rastreamento da web comercial.
- **Estética Brutalista:** Interface neural feita em **Slint**, acelerada por hardware e leve o suficiente para sistemas embarcados.

## 🏗️ Arquitetura ACE (Albedo Core Engine)
A engine ACE é composta por subsistemas modulares projetados para alta concorrência:
- **ACE-HTML (Parser):** Parser HTML5 com tree builder aderente ao spec WHATWG, powered by `html5ever` + otimizações de arena allocation e SIMD scanning.
- **ACE-Net:** Camada de networking assíncrona ultra-rápida baseada em Tokio, com HTTP/1.1, HTTP/2 e HTTP/3 (QUIC).
- **AlbedoJIT (Em desenvolvimento):** Motor JavaScript/Jasm nativo para processamento de scripts de alta performance.
- **DOM Bridge:** Integração direta e sem overhead entre o motor de script e a árvore DOM do Albedo.

## 🛠️ Tech Stack
- **Core:** Rust 🦀 (The Only Choice)
- **HTML Parsing:** html5ever + html5gum (WHATWG compliant)
- **CSS:** cssparser + selectors (spec compliant)
- **Layout:** Taffy (Flexbox/Grid)
- **Rasterização:** tiny-skia (2D) + wgpu (GPU compositor)
- **Text:** cosmic-text (shaping + rendering)
- **Networking:** reqwest + quinn/h3 (HTTP/3 QUIC) + rustls (TLS 1.3)
- **Scripting:** QuickJS via rquickjs (preparando AlbedoJIT)
- **UI:** Slint (GPU-Accelerated Framework)

## 🚀 Como Rodar
```bash
# Clone o repositório
git clone https://github.com/Alekkzsx/Albedo-Browser

# Execute o motor
cargo run --release
```

## 🗺️ O Caminho para o 1.0
- [x] **ACE-HTML:** Parser HTML5 funcional com tree building.
- [x] **DOM Core:** Árvore de nós com Shadow DOM, Custom Elements, a11y.
- [x] **JS Integration:** Suporte funcional a scripts via QuickJS.
- [x] **ACE-Net:** HTTP/1.1, HTTP/2, HTTP/3 QUIC, TLS 1.3, cache em disco.
- [ ] **Conformidade WHATWG:** html5lib tree construction ≥90%.
- [ ] **Performance:** Throughput de parsing ≥150 MB/s (superando Chrome/Firefox).
- [ ] **AlbedoJIT:** Motor JavaScript JIT próprio (quando benchmarks justificarem).
- [ ] **Web Completa:** Video/Audio, WebGL, DevTools.

---
*"O Albedo é a nossa resposta técnica à pergunta: por que a internet ficou tão pesada?"*
