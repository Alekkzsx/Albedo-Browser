# 📁 scripts/

Esta pasta contém scripts utilitários para automatizar processos de build, execução, linting e benchmarking do **Albedo Browser**.

## 🎯 Objetivo & Função
Simplificar o fluxo de trabalho do desenvolvedor, provendo atalhos rápidos para compilação, inicialização, linting e testes de desempenho.

## 📄 Arquivos e Suas Funções

| Arquivo | Função / Propósito |
| :--- | :--- |
| `build.ps1` | Executa o build em modo release (`cargo build --release`). |
| `run.ps1` | Compila e inicia o browser em modo release (`cargo run --release`). |
| `benchmark_ace.ps1` | Roda benchmarks de desempenho da engine. |
| `lint.sh` | **Linter customizado** — 12 verificações de código (Bash/Linux/macOS). |
| `lint.ps1` | **Linter customizado** — 12 verificações de código (PowerShell/Windows). |

## 🔍 Linter

O linter verifica **12 regras** de código:

| # | Check | O que detecta |
|---|-------|---------------|
| 1 | allow attributes | Qualquer `#[allow]` → ERROR |
| 2 | println/eprintln | Print como logging → ERROR |
| 3 | dbg!() | Zero tolerância → ERROR |
| 4 | unwrap() | Em não-teste → ERROR |
| 5 | unsafe blocks | Sem // SAFETY: → ERROR |
| 6 | Função sem comment | Sem doc comment → ERROR |
| 7 | Função longa | > 150 linhas → ERROR |
| 8 | Arquivo longo | > 250 linhas → ERROR |
| 9 | Module boundaries | network/→ace/ etc → ERROR |
| 10 | Imports circulares | A→B→A → ERROR |
| 11 | Cyclic deps | albedo-jit↔albedo → ERROR |
| 12 | Nomes abreviados | x, y, tmp etc → ERROR |

### Uso

```bash
# Linux/macOS
./scripts/lint.sh           # Para no 1º erro
./scripts/lint.sh --all     # Mostra todos os erros
./scripts/lint.sh --ci      # Output GitHub Actions

# Windows (PowerShell)
.\scripts\lint.ps1          # Para no 1º erro
.\scripts\lint.ps1 -All     # Mostra todos os erros
```

### Formato de Saída

```
src/network/resources.rs:3:1: error[CHECK9]: network/ imports from ace/
  suggested: use crate::network::contracts interface instead
```

## 🛠️ Regras de Design

### O que DEVE estar aqui:
- Scripts de automação (PowerShell, Bash) para build, testes, lint, deploy.
- Scripts de profiling e instrumentação.

### O que NÃO DEVE estar aqui:
- Código-fonte Rust (deve estar em `src/` ou `albedo-jit/`).
- Assets visuais ou páginas internas.
