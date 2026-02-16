# 🌑 Albedo Browser
> "Velocidade da luz em qualquer hardware."

[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust)](https://www.rust-lang.org)

**Albedo** é uma reação contra o inchaço da web moderna. Um navegador real desenhado para a filosofia *minimalista* e *brutalista*, focado em eficiência térmica e baixo uso de memória RAM. Ao contrário de outros navegadores "leves" que usam WebViews do sistema, o Albedo utiliza sua própria engine: a **ACE (Albedo Core Engine)**.

## ⚡ Por que Albedo?
A maioria dos navegadores hoje são Sistemas Operacionais disfarçados. Eles consomem 1GB de RAM apenas para exibir texto. O Albedo é diferente:
- **ACE Engine:** Uma engine de renderização customizada escrita do zero em Rust para máxima performance.
- **Networking Paralelo:** Carregamento assíncrono de recursos (HTML, CSS, imagens) para uma experiência de navegação sem travamentos.
- **QuickJS Integration:** Um interpretador de JavaScript leve e extremamente rápido integrado ao DOM.
- **Interface Neural:** GUI feita em **Slint**, leve como sistemas embarcados e acelerada por hardware.
- **Privacy First:** Sem telemetria, sem processos fantasmas e focado em total transparência.

## 🛠️ Tech Stack (Albedo Core Engine - ACE)
- **Linguagem:** Rust 🦀
- **Parsing HTML:** Kuchiki (HTML5ever)
- **Motor de Layout:** Taffy (Flexbox & Grid nativo)
- **Scripting:** QuickJS via `rquickjs`
- **Networking:** Async Resource Loader (Tokio + Reqwest)
- **UI Toolkit:** Slint (Acelerado por GPU)

## 🏗️ Arquitetura
O Albedo utiliza um modelo de **MPSC Channels** e **Shared State** para processar a web de forma assíncrona. Enquanto imagens e estilos são baixados em background, a UI permanece responsiva a 60 FPS.

## 🚀 Como Rodar (Desenvolvimento)
1. Certifique-se de ter o Rust instalado (`rustup`).
2. Clone o repositório.
3. Execute o comando:
```bash
cargo run
```

## 🗺️ Roadmap
- [x] Engine ACE básica (Layout + Render)
- [x] Integração JS-DOM (QuickJS)
- [x] Networking Assíncrono e Paralelo
- [ ] Suporte completo a CSS Grid/Flexbox avançado
- [ ] Sistema de Abas persistente
- [ ] API de Extensões via Rust/JS
