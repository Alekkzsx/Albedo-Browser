# 📁 scripts/

Esta pasta contém scripts utilitários em **PowerShell** (`.ps1`) para automatizar processos de build, execução e benchmarking do **Albedo Browser**.

## 🎯 Objetivo & Função
Simplificar o fluxo de trabalho do desenvolvedor no ambiente Windows, provendo atalhos rápidos para compilação em múltiplos perfis (debug/release), inicialização rápida do navegador e disparo de testes de benchmarks de desempenho comparativos.

## 📄 Arquivos e Suas Funções

| Arquivo | Função / Propósito |
| :--- | :--- |
| [build.ps1](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/scripts/build.ps1) | Executa o build da aplicação em modo release (`cargo build --release`) garantindo que as dependências do Slint sejam inicializadas. |
| [run.ps1](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/scripts/run.ps1) | Atalho rápido para compilar e iniciar o browser em modo release (`cargo run --release`). |
| [benchmark_ace.ps1](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/scripts/benchmark_ace.ps1) | Roda o motor em modo perfilador de desempenho para colher estatísticas de throughput da engine de layout e JIT. |

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

### O que DEVE estar aqui:
- Scripts de automação de console (PowerShell, Bash, Batch) para tarefas de compilação, testes, empacotamento ou deploy.
- Scripts de auxílio à instrumentação de código em tempo de profiling (ex: coletores de logs do JIT).

### O que NÃO DEVE estar aqui:
- Código-fonte em Rust do navegador (deve estar em `src/` ou `albedo-jit/`).
- Primitivas visuais ou assets estáticos de páginas internas.
