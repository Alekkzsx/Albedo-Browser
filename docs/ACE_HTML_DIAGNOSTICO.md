# Diagnóstico: Estado Atual do ACE-HTML

## TL;DR

Há **inconsistência nos dados** entre relatórios diferentes. O benchmark mais recente mostra **100% pass rate**, mas o relatório de comparação mostra **1.34%**. A causa é a **diferença no formato de comparação** (normalização).

---

## Arquivos de Evidência

| Arquivo | Data | Pass Rate | Formato |
|--------|------|----------|--------|
| `ace_html_benchmark_results.json` | 29/04/2026 | 100% | Rust com normalização |
| `ace_html_super_benchmark_results.json` | 29/04/2026 | 100% | Rust com normalização |
| `browser_html_benchmark_results.json` | 26/04/2026 | 99% | **sem** normalização |
| `ACE_HTML_BROWSER_COMPARISON.md` | 29/04/2026 | 1.34% | Desconhecido |

---

## O Problema: Normalização

### O que é a normalização?

A normalização é uma transformação aplicada antes de comparar árvores:

```javascript
// Funcão de normalização (Rust e Linux)
// normalize_tree_template_markers / normalizeTreeTemplateMarkers

// Transforma:
// | <template-content>  →  | content
// | content          →  | content
```

### Onde é usada

| Componente | Usa Normalização | Pass Rate |
|-----------|-----------------|----------|
| Rust `ace_html_super_benchmark.rs` | ✅ Sim | 100% |
| Linux `linux_browser_benchmark.js` | ✅ Sim | 99% |
| Windows `browser_html_benchmark.js` | ❌ Não | 99% |

### Por que importa

- **Com normalização**: `| <template-content>` == `| content` (passa)
- **Sem normalização**: `| <template-content>` != `| content` (falha)

---

## Resultados por Fonte

### Benchmark Rust (mais recente)

```json
{
  "ace_html": {
    "html5libTreeFull": {
      "passed": 1498,
      "total": 1498,
      "passRate": 100
    },
    "requiredTree": {
      "passed": 14,
      "total": 14,
      "passRate": 100
    }
  }
}
```

### Browser Benchmark (Windows)

```json
{
  "chrome": {
    "requiredTree": { "passRate": 100 },
    "html5libTreeSample": { "passed": 99, "total": 100, "passRate": 99 }
  },
  "firefox": {
    "requiredTree": { "passRate": 100 },
    "html5libTreeSample": { "passed": 100, "total": 100, "passRate": 100 }
  }
}
```

### Relatório de Comparação (antigo?)

```
| Engine | Passou | Total | Taxa |
|--------|-------|-------|------|
| ACE-HTML | 20 | 1498 | 1.34% |
| Chrome | 1328 | 1498 | 88.65% |
| Firefox | 1307 | 1498 | 87.25% |
```

**Este relatório mostra 1.34% - formato desconhecido ou dados desatualizados.**

---

## O que está implementado

### Biblioteca base: html5ever 0.39

O ACE-HTML usa `html5ever` como parser base, que **implementa**:

- ✅ Adoption Agency Algorithm
- ✅ Foster Parenting
- ✅Todas as insertion modes
- ✅ Fragment parsing
- ✅ Namespace SVG/MathML Templates
- ✅ Frameset handling

###Arquitetura do Código

```
src/ace/html/
├── mod.rs                    # Entry points, fast path, streaming
├── html5ever_parser.rs       # Parser principal (1059 linhas)
├── tree_builder.rs          # Versão rudimentar (não usada)
├── tokenizer_v2.rs          # Tokenizer
└── entities.json            # Entidades HTML
```

### Conversão de Árvore

O `AceSinkNode` em `html5ever_parser.rs` faz a conversão do formato interno do html5ever para o formato do Albedo.

---

## Possíveis Causas de Divergência

Mesmo usando html5ever (que implementa tudo), ainda pode haver gaps por:

| # | Causa | Evidência |
|---|------|----------|
| 1 | **Serializer diferente** | `AceSinkNode` pode serializar diferente do browser |
| 2 | **Erros removidos** | `strip_generic_tree_builder_errors` remove erros úteis |
| 3 | **Fixups agressivos** | `apply_html5lib_compat_fixups` altera comportamento |
| 4 | **Dados desatualizados** | Relatório pode ser de versão antiga |

### Código que remove erros

```rust
// html5ever_parser.rs:451

fn strip_generic_tree_builder_errors(result: &mut ParseResult) {
    result.errors.retain(|error| {
        !(error.source == ParseErrorSource::TreeBuilder && error.kind == ParseErrorKind::HtmlSyntax)
    });
    result.parse_errors.retain(|error| {
        !(error.source == ParseErrorSource::TreeBuilder && error.kind == ParseErrorKind::HtmlSyntax)
    });
}
```

Este código **remove erros importantes** que podem indicar problemas reais!

---

## Testes de Conformidade

### Testes Existentes

| Teste | Escopo | Status |
|-------|-------|--------|
| `ace_html_tree_construction_smoke` | html5lib tree-construction | ⏳ |
| `ace_html_tree_construction_full_report` |html5lib completo | ⏳ |
| `ace_html_required_conformance_passes_100` | required.json | ⏳ |
| `ace_html_hard_cases_guard` | casos difíceis | ⏳ |

### Casos de Teste

- `tests/ace_html_conformance/required.json` - 28 casos
- `tests/html5lib/tree-construction/*.dat` - 1498 casos
- `tests/ace_html_hard_cases.json` - casos difíceis

---

## Conclusão

### Estado Atual

1. **Benchmark recente**: 100% pass rate (Rust), 99% (browsers)
2. **html5ever**: Implementa todos os algoritmos necessários
3. **Código**: Estruturado, usa bibliotecas estabelecidas

### O que precisa verificar

1. **Confirmar se os testes passam** - executar `cargo test`
2. **Entender a origem do 1.34%** - relatório pode ser antigo
3. **Decidir sobre normalização** - se deve manter ou não
4. **Verificar erros removidos** - analisar impacto

### Recomendação

1. Executar testes localmente para confirmar estado
2. Manter normalização (browsers também usam em produção)
3. Investigar relatório antigo de 1.34%
4. Avaliar se `strip_generic_tree_builder_errors` é necessário

---

## Comandos Úteis

```bash
# Executar todos os testes de tree construction
cargo test --test html5lib_tree_harness ace_html_tree_construction_full_report -- --nocapture

# Filtrar por arquivo específico
ACE_HTML_TREE_FILE_FILTER=adoption cargo test --test html5lib_tree_harness -- --nocapture

# Executar benchmark de performance
cargo run --bin ace_html_super_benchmark

# Executar teste de conformance Required
cargo test --test ace_html_equivalence -- --nocapture
```

---

*Documento gerado em 29/04/2026*
*Última atualização: perlu verificar resultados reais executando os testes*