# 🌑 Albedo Browser
> "Velocidade da luz em hardware comum. Soberania tecnológica em cada linha de código."

[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**Albedo** é um navegador 100% Rust com filosofia de soberania tecnológica — zero dependências externas críticas.

---

## ⚠️ Estado Real do ACE-HTML

**O ACE-HTML NÃO está em nível, nem superior, ao Chrome/Blink.** Ainda estamos abaixo em conformidade, robustez e maturidade operacional.

### Lacunas Críticas de Conformidade

- **Estabilidade básica**: módulo teve quebra recente em `preload_scanner.rs`
- **WHATWG §13.2 incompleta**: lacunas reais em estados e modos especiais
- **html5lib não está 100% verde**: gaps críticos em:
  - Entidades históricas em atributos
  - Ambiguous ampersand
  - Adoption Agency Algorithm
  - Foster parenting
  - Foreign content SVG/MathML
  - Fragment parsing em contexto de tabela
  - Frameset insertion mode
- **Recuperação de HTML malformado**: ainda emite `UnexpectedToken`/`UnexpectedEof` onde engines maduros recuperam silenciosamente
- **Foreign content incompleto**: integração HTML/SVG/MathML sem precisão de Chrome/Firefox
- **`<template>` parcial**: falta comportamento completo do ecossistema template
- **Preload scanner imaturo**: não equivalente ao pipeline robusto de browsers grandes
- **Encoding detection básico**: não tem completude/compatibilidade histórica dos engines de referência

### Testes Pendentes (Prioridade Máxima)

- [ ] html5lib tree-construction amplo
- [ ] Tokenizer completo (100%)
- [ ] Fragment cases extensivos
- [ ] Regressões de páginas reais (top 1000 sites)
- [ ] Fuzzing pesado sem panic (10M+ inputs)
- [ ] Guard-rails de produção (CI bloqueia regressões)
- [ ] Performance de engine madura (documentos gigantes, streaming, inputs hostis)

Veja **[PLANO_REALISTA.md](./PLANO_REALISTA.md)** para roadmap de longo prazo (5-10 anos) e **[PLANO_DE_IMPLEMENTACAO.md](./PLANO_DE_IMPLEMENTACAO.md)** para plano técnico detalhado de implementação (52 semanas).

---

## 📁 Estrutura do Projeto

```
src/
├── ace/          # Parser HTML5 nativo, URL, JSON, Crypto
├── engine/       # DOM, CSS, Layout, Graphics, Text, Compositor
├── runtime/      # QuickJS integration (→ AlbedoJIT)
├── network/      # HTTP/1.1/2/3, QUIC, Cache, DNS
├── browser/      # Tabs, Events, Bridge
├── renderer.rs   # Pipeline visual
└── ui.rs         # Slint framework (→ ACE-UI)
```

## 📊 Progresso Real

| Módulo | Status | Notas |
|--------|--------|-------|
| Tokenizer | ~91% | 80/88 estados, mas gaps críticos |
| Error Handling | 100% | 58 codes implementados |
| Shadow DOM v1 | ✅ | Implementado |
| Custom Elements | ✅ | Implementado |
| Slots | ✅ | Implementado |
| SVG Elements | ✅ | 39 elementos |
| MathML Elements | ✅ | 25 elementos |
| Encoding | Parcial | 52 codificações, sem compatibilidade total |
| html5lib tree-construction | <50% | Centenas de casos falhando |
| html5lib fragment | <30% | Gaps críticos em tables/templates |

---

## 🚀 Rodar

```bash
cargo run --release
```

## 🎯 Próximos Passos Críticos

1. **Fechar html5lib tests** — prioridade máxima
2. **Estabilizar preload_scanner** — fixar bugs recentes
3. **Implementar AAA + foster parenting** — algoritmos críticos
4. **Melhorar error recovery** — parar de emitir erros desnecessários
5. **Completar foreign content** — integração precisa SVG/MathML
6. **Template ecosystem** — fragment parsing completo
7. **Fuzzing contínuo** — garantir zero panics
8. **Performance** — otimizar para documentos reais

---

## 🛠️ Tech Stack

- **Core:** Rust 🦀
- **Networking:** Tokio-based async loader
- **Scripting:** QuickJS (→ AlbedoJIT)
- **UI:** Slint GPU-Accelerated (→ ACE-UI)
- **Layout:** Taffy (→ ACE-Layout)

---

*Objetivo: rivalizar com Firefox/Chrome sendo minimalista e 100% próprio. Não há atalhos — conformidade exige trabalho duro e teste exaustivo.*
