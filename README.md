# 🌑 Albedo Browser
> "Velocidade da luz em qualquer hardware."

[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust)](https://www.rust-lang.org)

**Albedo** é uma reação contra o inchaço da web moderna. Um navegador desenhado para a filosofia *minimalista* e *brutalista*, focado em eficiência térmica e baixo uso de memória RAM.

## ⚡ Por que Albedo?
A maioria dos navegadores hoje são Sistemas Operacionais disfarçados. Eles consomem 1GB de RAM apenas para exibir texto. O Albedo é diferente:
- **Core em Rust:** Segurança de memória sem Garbage Collector.
- **Engine Híbrida:** Utiliza bibliotecas nativas do sistema (WRY/Tao) para manter o binário minúsculo (~15MB).
- **Interface Neural:** GUI feita em **Slint** (leve como sistemas embarcados), totalmente controlável por teclado/Lua.
- **Privacy First:** Sem telemetria, sem "sincronização na nuvem", sem processos fantasmas.

## 🛠️ Tech Stack
- **Linguagem:** Rust 🦀
- **UI Toolkit:** Slint (Nativo e acelerado por GPU mínima)
- **Webview:** WRY (Cross-platform WebView rendering library)
- **Scripting:** LuaJIT (Futuro suporte para plugins)

## 🚀 Como Rodar (Desenvolvimento)
Você precisa ter o Rust instalado (`rustup`).
