# 🌑 Albedo Browser
> "Velocidade da luz em hardware comum. Soberania tecnológica em cada linha de código."

[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**Albedo** não é apenas um navegador; é um manifesto técnico contra a obesidade da web moderna. Desenhado sob a filosofia *minimalista* e *independente*, o Albedo rejeita a hegemonia do motor Blink/Chromium para construir seu próprio caminho: a **ACE (Albedo Core Engine)**.

## ⚡ A Filosofia Albedo
Hoje, um navegador consome 1GB de RAM para exibir uma página de texto. O Albedo quebra essa norma através de:
- **ACE Engine:** Motor de renderização **100% nativo** escrito em Rust, focado em performance pura.
- **Independência Total:** Estamos eliminando dependências externas críticas (como `html5ever` e `kuchiki`) em favor de implementações proprietárias e otimizadas.
- **Zero Bloat:** Sem telemetria, sem processos fantasmas e sem o lixo de rastreamento da web comercial.
- **Estética Brutalista:** Interface neural feita em **Slint**, acelerada por hardware e leve o suficiente para sistemas embarcados.

## 🏗️ Arquitetura ACE (Albedo Core Engine)
A engine ACE é composta por subsistemas modulares projetados para alta concorrência:
- **ACE-HTML (Parser):** Motor de parsing HTML5 proprietário, 100% aderente ao spec WHATWG, com suporte nativo a AAA (Adoption Agency Algorithm).
- **ACE-Net:** Camada de networking assíncrona ultra-rápida baseada em Tokio, otimizada para carregamento paralelo de recursos.
- **AlbedoJIT (Em desenvolvimento):** Motor JavaScript/Jasm nativo para processamento de scripts de alta performance.
- **DOM Bridge:** Integração direta e sem overhead entre o motor de script e a árvore DOM do Albedo.

## 🛠️ Tech Stack Atual
- **Core:** Rust 🦀 (The Only Choice)
- **Networking:** Custom Async Resource Loader (Tokio-based)
- **Scripting:** QuickJS Integration (preparando migração para AlbedoJIT)
- **UI:** Slint GPU-Accelerated Framework
- **Layout:** Taffy (preparando migração para ACE-Layout nativo)

## 🚀 Como Rodar
```bash
# Clone o repositório
git clone https://github.com/Alekkzsx/Albedo-Browser

# Execute o motor
cargo run --release
```

## 🗺️ O Caminho para o 1.0 (Zero-Departure Roadmap)
- [x] **ACE-HTML:** Parser HTML5 nativo e independente.
- [x] **DOM Core:** Árvore de nós proprietária e manuseável.
- [x] **JS Integration:** Suporte funcional a scripts via QuickJS.
- [ ] **ACE-Layout:** Motor de layout (Flexbox/Grid) 100% Albedo.
- [ ] **AlbedoJIT:** Motor JavaScript de produção próprio.
- [ ] **ACE-UI:** Interface final sem dependências de frameworks externos.

---
*"O Albedo é a nossa resposta técnica à pergunta: por que a internet ficou tão pesada?"*
