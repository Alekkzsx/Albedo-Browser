# ACE-HTML Chrome-Level Performance & Conformance

## 1. Visão Geral

### 1.1 Objetivo
Elevar o parser HTML ACE-HTML ao nível ou superior ao Chrome/Blink em:
- **Performance**: Throughput ≥ 500 MB/s (Chrome: ~400-600 MB/s)
- **Conformidade**: 100% WHATWG HTML Living Standard
- **Latência**: Parsing incremental < 1ms por chunk
- **Memória**: Footprint ≤ 50% do Chrome para documentos equivalentes

### 1.2 Motivação
O ACE-HTML atual (~5% de completude segundo PLAN.txt) tem:
- ✅ Parser HTML5 funcional com tokenizer/tree builder
- ✅ SIMD básico (SSE2/AVX2) para whitespace e entity lookup
- ✅ Streaming parser com métricas
- ❌ Performance ~10-20x mais lenta que Chrome
- ❌ Conformidade WHATWG incompleta (faltam edge cases)
- ❌ Sem optimizações críticas (speculative parsing, preload scanner robusto)
- ❌ Sem paralelização (Chrome usa threads para tokenização)

### 1.3 Escopo
**In-Scope**:
- Otimizações de performance (SIMD avançado, paralelização, zero-copy)
- Conformidade 100% WHATWG (todos os estados do lexer, AAA completo)
- Streaming incremental otimizado
- Preload scanner especulativo robusto
- Benchmarking contra Chrome/Firefox

**Out-of-Scope**:
- Integração com DOM (já existe em engine/dom/)
- CSS parsing (será tratado em ACE-CSS)
- JavaScript execution (será tratado em AlbedoJIT)

### 1.4 Filosofia Zero-Dependências 🎯
**CRÍTICO**: Este projeto segue a filosofia **100% PRÓPRIA DO ALBEDO**.

**Permitido**:
- ✅ Rust `std` (standard library)
- ✅ Rust `core` (no_std compatible)
- ✅ Datasets de teste (arquivos JSON/HTML do html5lib-tests)
- ✅ Documentos de especificação (WHATWG, W3C)

**PROIBIDO**:
- ❌ Crates externos (html5ever, scraper, select, etc.)
- ❌ Bibliotecas de parsing (nom, pest, lalrpop, etc.)
- ❌ Frameworks de benchmark (criterion, divan, etc.)
- ❌ Qualquer dependência que não seja `std`

**Implementação Própria Obrigatória**:
- Parser HTML5 completo (lexer + tokenizer + tree builder)
- Arena allocator
- String interner
- SIMD optimizations (SSE2/AVX2/AVX-512)
- Benchmark framework
- Test harness para html5lib-tests
- Profiling e métricas

**Justificativa**:
> "Cada linha de código que roda no Albedo é do Albedo. Nenhum crate de terceiro em subsistema crítico."  
> — PLAN.txt, Filosofia do Projeto

Isso garante:
- **Soberania tecnológica**: Controle total sobre o código
- **Performance otimizada**: Sem overhead de abstrações genéricas
- **Manutenibilidade**: Sem breaking changes de dependências
- **Segurança**: Sem supply chain attacks
- **Tamanho binário**: Sem bloat de dependências não usadas

---

## 2. Análise de Gaps (ACE-HTML vs Chrome)

### 2.1 Performance Gaps

| Métrica | ACE-HTML Atual | Chrome/Blink | Gap | Prioridade |
|---------|----------------|--------------|-----|------------|
| Throughput (MB/s) | ~30-50 | 400-600 | **12-20x** | 🔴 Crítico |
| Latência incremental | ~5-10ms | <1ms | **5-10x** | 🔴 Crítico |
| Memória (10MB doc) | ~80MB | ~40MB | **2x** | 🟡 Alto |
| Startup (first token) | ~2ms | <0.5ms | **4x** | 🟢 Médio |
| SIMD coverage | ~15% | ~60% | **4x** | 🔴 Crítico |
| Paralelização | 0 threads | 2-4 threads | **∞** | 🔴 Crítico |

**Análise**:
- Chrome usa **speculative parsing** (tokeniza em thread separada)
- Chrome tem **preload scanner** que extrai `<link>`, `<script>`, `<img>` antes do tree builder
- Chrome usa **SIMD extensivo** (AVX-512 em CPUs modernos)
- Chrome tem **zero-copy** para strings (arena allocator + string interning)

### 2.2 Conformance Gaps

#### 2.2.1 Lexer States (WHATWG §13.2.5)
**Status Atual**: ~70/80 estados implementados

**Faltando**:
```
❌ Script data escaped states (completo mas não testado)
❌ CDATA section states (stub básico)
❌ Character reference ambiguous ampersand
❌ Comment less-than sign states (parcial)
```

#### 2.2.2 Tree Builder (WHATWG §13.2.6)
**Status Atual**: ~18/24 insertion modes implementados

**Faltando**:
```
❌ InTemplate mode (stub básico)
❌ InFrameset/AfterFrameset (parcial)
❌ AfterAfterBody/AfterAfterFrameset (stub)
```

#### 2.2.3 Adoption Agency Algorithm (WHATWG §13.2.6.4.7)
**Status Atual**: Versão simplificada implementada

**Gaps**:
```
❌ Outer loop limit (8 iterations) não enforçado
❌ Inner loop limit (3 iterations) não enforçado
❌ Bookmark tracking incompleto
❌ Formatting element cloning edge cases
```

#### 2.2.4 Foster Parenting (WHATWG §13.2.6.1)
**Status Atual**: Implementado mas não testado extensivamente

**Gaps**:
```
⚠️ Inserção antes de table vs tbody/thead/tfoot
⚠️ Template elements como foster parent
```

### 2.3 Feature Gaps

| Feature | ACE-HTML | Chrome | Impacto |
|---------|----------|--------|---------|
| Speculative parsing | ❌ | ✅ | Alto (2-3x speedup) |
| Preload scanner | 🟡 Básico | ✅ Robusto | Alto (network latency) |
| Parallel tokenization | ❌ | ✅ | Alto (2x speedup) |
| SIMD entity decode | 🟡 Parcial | ✅ Completo | Médio (5-10% speedup) |
| Zero-copy strings | 🟡 Interner | ✅ Arena | Médio (30% memory) |
| Incremental layout | ❌ | ✅ | Alto (UX) |
| Error recovery | 🟡 Básico | ✅ Robusto | Médio (conformance) |

---

## 3. Requisitos Funcionais

### 3.1 Conformidade WHATWG 100%

#### RF-001: Lexer States Completos
**Prioridade**: 🔴 Crítica  
**Descrição**: Implementar todos os 80 estados do lexer conforme WHATWG §13.2.5

**Critérios de Aceite**:
- [ ] Todos os 80 estados implementados e testados
- [ ] html5lib-tests: 100% pass rate (tokenizer suite)
- [ ] Character references: todas as 2.231 entidades nomeadas
- [ ] Numeric references: suporte completo (decimal, hex, surrogate pairs)
- [ ] Error recovery: todos os 52 parse errors do spec

**Testes**:
```rust
#[test]
fn test_all_lexer_states_coverage() {
    // Verificar que todos os 80 estados são alcançáveis
    let states = collect_reachable_states();
    assert_eq!(states.len(), 80);
}

#[test]
fn test_html5lib_tokenizer_suite() {
    // ~2.800 test cases do html5lib
    let pass_rate = run_html5lib_tests("tokenizer");
    assert_eq!(pass_rate, 1.0); // 100%
}
```

#### RF-002: Tree Builder Insertion Modes Completos
**Prioridade**: 🔴 Crítica  
**Descrição**: Implementar todos os 24 insertion modes conforme WHATWG §13.2.6

**Critérios de Aceite**:
- [ ] Todos os 24 insertion modes implementados
- [ ] html5lib-tests: 100% pass rate (tree construction suite)
- [ ] Template elements: suporte completo com stack de insertion modes
- [ ] Frameset: suporte completo (frameset-ok flag)
- [ ] Foreign content: SVG/MathML namespace handling

**Testes**:
```rust
#[test]
fn test_html5lib_tree_construction_suite() {
    // ~2.800 test cases do html5lib
    let pass_rate = run_html5lib_tests("tree-construction");
    assert_eq!(pass_rate, 1.0); // 100%
}
```

#### RF-003: Adoption Agency Algorithm Completo
**Prioridade**: 🟡 Alta  
**Descrição**: Implementar AAA conforme WHATWG §13.2.6.4.7 com todos os limites e edge cases

**Critérios de Aceite**:
- [ ] Outer loop: máximo 8 iterações
- [ ] Inner loop: máximo 3 iterações
- [ ] Bookmark tracking correto
- [ ] Formatting element cloning com atributos
- [ ] Marker handling (scope boundaries)

**Testes**:
```rust
#[test]
fn test_adoption_agency_nested_formatting() {
    let html = "<b><i>1<b>2</i>3</b>4";
    let doc = parse_document(html);
    // Verificar estrutura correta após AAA
    assert_tree_structure(doc, expected_tree);
}
```

#### RF-004: Foster Parenting Robusto
**Prioridade**: 🟡 Alta  
**Descrição**: Implementar foster parenting conforme WHATWG §13.2.6.1

**Critérios de Aceite**:
- [ ] Inserção antes de table
- [ ] Inserção antes de tbody/thead/tfoot
- [ ] Template elements como foster parent
- [ ] Text nodes consolidation

**Testes**:
```rust
#[test]
fn test_foster_parenting_before_table() {
    let html = "<table><div>text</div><tr><td>cell</td></tr></table>";
    let doc = parse_document(html);
    // <div> deve ser foster parented antes de <table>
    assert_foster_parent_position(doc);
}
```

### 3.2 Performance Otimizada

#### RF-005: Throughput ≥ 500 MB/s
**Prioridade**: 🔴 Crítica  
**Descrição**: Atingir throughput de parsing ≥ 500 MB/s em CPU moderna (Zen 4 / Raptor Lake)

**Critérios de Aceite**:
- [ ] Benchmark: Wikipedia homepage (1.2 MB) em < 2.5ms
- [ ] Benchmark: Large document (10 MB) em < 20ms
- [ ] Benchmark: Synthetic stress test (100 MB) em < 200ms
- [ ] Throughput médio ≥ 500 MB/s

**Testes**:
```rust
#[bench]
fn bench_wikipedia_homepage(b: &mut Bencher) {
    let html = load_wikipedia_homepage(); // ~1.2 MB
    b.iter(|| {
        let start = Instant::now();
        parse_document(&html);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_micros(2500));
    });
}
```

#### RF-006: Latência Incremental < 1ms
**Prioridade**: 🔴 Crítica  
**Descrição**: Parsing incremental com latência < 1ms por chunk (16KB)

**Critérios de Aceite**:
- [ ] Chunk 16KB: latência p50 < 0.5ms, p99 < 1ms
- [ ] Chunk 64KB: latência p50 < 2ms, p99 < 5ms
- [ ] Sem alocações por chunk (arena pre-allocated)
- [ ] Backpressure handling para streaming

**Testes**:
```rust
#[test]
fn test_streaming_latency() {
    let mut parser = StreamingHtmlParser::new();
    let chunks = split_into_chunks(large_html, 16384);
    
    for chunk in chunks {
        let start = Instant::now();
        parser.feed(chunk);
        let latency = start.elapsed();
        assert!(latency < Duration::from_millis(1));
    }
}
```

#### RF-007: Memória ≤ 50% do Chrome
**Prioridade**: 🟡 Alta  
**Descrição**: Footprint de memória ≤ 50% do Chrome para documentos equivalentes

**Critérios de Aceite**:
- [ ] 10 MB document: ≤ 40 MB peak memory (Chrome: ~80 MB)
- [ ] 100 MB document: ≤ 200 MB peak memory (Chrome: ~400 MB)
- [ ] Arena allocator: < 10% overhead
- [ ] String interning: > 70% hit rate

**Testes**:
```rust
#[test]
fn test_memory_footprint() {
    let html = generate_large_document(10_000_000); // 10 MB
    let before = get_memory_usage();
    let doc = parse_document(&html);
    let after = get_memory_usage();
    let peak = after - before;
    assert!(peak < 40_000_000); // < 40 MB
}
```

### 3.3 Features Avançadas

#### RF-008: Speculative Parsing
**Prioridade**: 🔴 Crítica  
**Descrição**: Tokenização especulativa em thread separada (como Chrome)

**Critérios de Aceite**:
- [ ] Tokenizer roda em thread separada
- [ ] Tree builder consome tokens via channel
- [ ] Speedup 2-3x em documentos grandes (> 1 MB)
- [ ] Fallback para single-thread se necessário

**Implementação Própria**:
- Thread pool próprio (sem rayon, tokio, etc.)
- Channel próprio (std::sync::mpsc ou implementação custom)
- Work stealing scheduler (se necessário)

**Testes**:
```rust
#[test]
fn test_speculative_parsing_speedup() {
    let html = load_large_document(); // 10 MB
    
    let single_thread_time = bench_single_thread(&html);
    let speculative_time = bench_speculative(&html);
    
    let speedup = single_thread_time / speculative_time;
    assert!(speedup >= 2.0); // Mínimo 2x speedup
}
```

#### RF-009: Preload Scanner Robusto
**Prioridade**: 🔴 Crítica  
**Descrição**: Preload scanner que extrai recursos antes do tree builder

**Critérios de Aceite**:
- [ ] Detecta: `<link rel="stylesheet">`, `<script src>`, `<img src>`, `<link rel="preload">`
- [ ] Extrai: URL, tipo, atributos (async, defer, crossorigin)
- [ ] Roda em paralelo com tokenizer
- [ ] Latência < 0.1ms para scan de 1 MB

**Implementação Própria**:
- Scanner próprio (não usar regex crates)
- State machine otimizada para extração rápida
- Zero allocations (arena-based)

**Testes**:
```rust
#[test]
fn test_preload_scanner_extraction() {
    let html = r#"
        <link rel="stylesheet" href="app.css">
        <script src="app.js" async></script>
        <img src="hero.jpg" loading="lazy">
    "#;
    
    let preloads = preload_scan(html);
    assert_eq!(preloads.len(), 3);
    assert_eq!(preloads[0].url, "app.css");
    assert_eq!(preloads[1].attributes.get("async"), Some(&""));
}
```

#### RF-010: SIMD Avançado (AVX-512)
**Prioridade**: 🟡 Alta  
**Descrição**: Otimizações SIMD avançadas para CPUs modernas

**Critérios de Aceite**:
- [ ] AVX-512: 64-byte parallel whitespace detection
- [ ] AVX-512: 64-byte parallel entity lookup
- [ ] AVX-512: 64-byte parallel tag name scanning
- [ ] Runtime detection: SSE2 → AVX2 → AVX-512
- [ ] Speedup 10-15% vs AVX2

**Implementação Própria**:
- Intrinsics diretos (std::arch::x86_64)
- Sem dependências de SIMD crates
- Runtime CPU detection próprio

**Testes**:
```rust
#[test]
fn test_simd_avx512_speedup() {
    if !is_x86_feature_detected!("avx512f") {
        return; // Skip se CPU não suporta
    }
    
    let html = generate_whitespace_heavy_document();
    let avx2_time = bench_with_avx2(&html);
    let avx512_time = bench_with_avx512(&html);
    
    let speedup = avx2_time / avx512_time;
    assert!(speedup >= 1.10); // Mínimo 10% speedup
}
```

#### RF-011: Zero-Copy String Handling
**Prioridade**: 🟡 Alta  
**Descrição**: Strings sem cópia usando arena allocator + string interning

**Critérios de Aceite**:
- [ ] Tag names: 100% interned (zero allocations após warmup)
- [ ] Attribute names: > 95% interned
- [ ] Attribute values: arena-allocated (single allocation)
- [ ] Text nodes: arena-allocated com deduplication

**Implementação Própria**:
- Arena allocator próprio (sem bumpalo, typed-arena, etc.)
- String interner próprio (HashMap-based ou perfect hashing)
- Memory pool management próprio

**Testes**:
```rust
#[test]
fn test_zero_copy_strings() {
    let html = "<div class='hero'><p>text</p></div>".repeat(1000);
    let mut interner = StringInterner::new();
    
    let doc = parse_with_interner(&html, &mut interner);
    let stats = interner.stats();
    
    assert!(stats.hit_rate > 0.95); // > 95% hit rate
    assert_eq!(stats.unique_strings, 3); // div, p, class
}
```

#### RF-012: Benchmark Framework Próprio
**Prioridade**: 🟡 Alta  
**Descrição**: Framework de benchmarking 100% Albedo (sem Criterion)

**Critérios de Aceite**:
- [ ] Micro benchmarks (< 1ms)
- [ ] Macro benchmarks (> 1ms)
- [ ] Statistical analysis (mean, median, p95, p99)
- [ ] Warmup iterations
- [ ] Outlier detection
- [ ] Comparison com baseline
- [ ] HTML report generation

**Implementação Própria**:
```rust
// src/ace/html/bench/mod.rs
pub struct BenchmarkRunner {
    warmup_iterations: usize,
    measurement_iterations: usize,
    outlier_threshold: f64,
}

impl BenchmarkRunner {
    pub fn bench<F>(&mut self, name: &str, f: F) -> BenchmarkResult
    where F: Fn() {
        // Implementação própria
    }
}
```

**Testes**:
```rust
#[test]
fn test_benchmark_framework() {
    let mut runner = BenchmarkRunner::new();
    let result = runner.bench("parse_simple", || {
        parse_document("<div>test</div>");
    });
    
    assert!(result.mean < Duration::from_micros(100));
    assert!(result.outliers < 5); // < 5% outliers
}
```

#### RF-013: html5lib Test Harness Próprio
**Prioridade**: 🔴 Crítica  
**Descrição**: Test harness para rodar html5lib-tests sem dependências

**Critérios de Aceite**:
- [ ] Parser de JSON próprio (sem serde_json)
- [ ] Test runner próprio (sem test frameworks externos)
- [ ] Diff engine para comparar árvores
- [ ] Report generation (HTML/JSON)
- [ ] Parallel test execution

**Implementação Própria**:
```rust
// src/ace/html/tests/html5lib_harness.rs
pub struct Html5libTestRunner {
    test_dir: PathBuf,
    parallel: bool,
}

impl Html5libTestRunner {
    pub fn run_tokenizer_tests(&self) -> TestResults {
        // Parser JSON próprio
        // Executar testes
        // Gerar relatório
    }
    
    pub fn run_tree_construction_tests(&self) -> TestResults {
        // Parser JSON próprio
        // Executar testes
        // Comparar árvores
    }
}
```

**Testes**:
```rust
#[test]
fn test_html5lib_harness() {
    let runner = Html5libTestRunner::new("tests/html5lib-tests");
    let results = runner.run_tokenizer_tests();
    
    assert_eq!(results.pass_rate(), 1.0); // 100%
    assert_eq!(results.total, 2800); // ~2800 test cases
}
```

---

## 4. Requisitos Não-Funcionais

### 4.1 Performance

#### RNF-001: Throughput Targets
- **Mínimo**: 300 MB/s (baseline)
- **Alvo**: 500 MB/s (Chrome-level)
- **Stretch**: 800 MB/s (superior ao Chrome)

#### RNF-002: Latência Targets
- **Chunk 16KB**: p50 < 0.5ms, p99 < 1ms
- **Chunk 64KB**: p50 < 2ms, p99 < 5ms
- **First token**: < 0.5ms (startup)

#### RNF-003: Memória Targets
- **10 MB doc**: ≤ 40 MB peak
- **100 MB doc**: ≤ 200 MB peak
- **Arena overhead**: < 10%

### 4.2 Conformidade

#### RNF-004: WHATWG Conformance
- **html5lib-tests**: 100% pass rate (tokenizer + tree construction)
- **WPT (Web Platform Tests)**: > 99% pass rate (HTML parsing subset)
- **Error recovery**: todos os 52 parse errors do spec

#### RNF-005: Compatibilidade
- **Chrome**: comportamento idêntico em 99.9% dos casos
- **Firefox**: comportamento idêntico em 99.5% dos casos
- **Safari**: comportamento idêntico em 99% dos casos

### 4.3 Manutenibilidade

#### RNF-006: Código
- **Cobertura de testes**: > 95%
- **Documentação**: 100% das APIs públicas
- **Benchmarks**: suite completa (micro + macro)

#### RNF-007: Monitoramento
- **Métricas**: throughput, latência, memória, SIMD coverage
- **Profiling**: flamegraphs, perf counters
- **Regression detection**: CI com benchmarks

---

## 5. Casos de Uso

### 5.1 UC-001: Parsing de Página Real (Wikipedia)
**Ator**: Albedo Browser  
**Pré-condições**: Página Wikipedia carregada (1.2 MB HTML)  
**Fluxo**:
1. Browser recebe HTML via network
2. ACE-HTML inicia parsing incremental
3. Preload scanner extrai recursos (CSS, JS, images)
4. Tokenizer roda em thread separada (speculative)
5. Tree builder constrói DOM incrementalmente
6. Layout engine recebe DOM parcial e inicia rendering

**Pós-condições**:
- Parsing completo em < 3ms
- Preloads extraídos em < 0.5ms
- First paint em < 50ms (incluindo layout)

### 5.2 UC-002: Streaming de Documento Grande
**Ator**: Albedo Browser  
**Pré-condições**: Documento 10 MB sendo baixado via HTTP/2  
**Fluxo**:
1. Browser recebe chunks de 16KB via network
2. ACE-HTML processa cada chunk incrementalmente
3. DOM é construída progressivamente
4. Layout engine renderiza conteúdo visível primeiro
5. Scroll funciona antes do parsing completo

**Pós-condições**:
- Latência por chunk < 1ms
- First paint em < 100ms
- Interatividade antes do parsing completo

### 5.3 UC-003: Parsing de HTML Malformado
**Ator**: Albedo Browser  
**Pré-condições**: HTML com erros de sintaxe  
**Fluxo**:
1. ACE-HTML detecta erros durante parsing
2. Error recovery conforme WHATWG spec
3. Parse errors são logados (dev tools)
4. DOM é construída com correções

**Pós-condições**:
- Parsing não falha
- DOM é válida e renderizável
- Comportamento idêntico ao Chrome

---

## 6. Benchmarks & Métricas

### 6.1 Benchmark Suite

#### Micro Benchmarks
```rust
// Tokenizer
bench_tokenizer_simple_tags()      // <div><p>text</p></div>
bench_tokenizer_attributes()       // <div class="x" id="y">
bench_tokenizer_entities()         // &nbsp; &lt; &gt; &amp;
bench_tokenizer_script_data()     // <script>...</script>
bench_tokenizer_comments()         // <!-- comment -->

// Tree Builder
bench_tree_builder_flat()          // Flat structure
bench_tree_builder_nested()        // Deep nesting
bench_tree_builder_tables()        // Complex tables
bench_tree_builder_foster()        // Foster parenting

// SIMD
bench_simd_whitespace_sse2()
bench_simd_whitespace_avx2()
bench_simd_whitespace_avx512()
bench_simd_entity_lookup()
bench_simd_tag_scan()

// Memory
bench_arena_allocator()
bench_string_interner()
bench_zero_copy_strings()
```

#### Macro Benchmarks
```rust
// Real-world documents
bench_wikipedia_homepage()         // 1.2 MB
bench_github_readme()              // 500 KB
bench_twitter_timeline()           // 2 MB
bench_amazon_product_page()        // 800 KB
bench_youtube_watch_page()         // 1.5 MB

// Synthetic stress tests
bench_large_table()                // 10K rows
bench_deep_nesting()               // 1K levels
bench_many_attributes()            // 100 attrs per element
bench_entity_heavy()               // 50% entities
```

### 6.2 Métricas de Sucesso

| Métrica | Baseline | Alvo | Stretch |
|---------|----------|------|---------|
| Throughput | 50 MB/s | 500 MB/s | 800 MB/s |
| Latência (16KB) | 5ms | 0.5ms | 0.3ms |
| Memória (10MB) | 80MB | 40MB | 30MB |
| html5lib pass rate | 95% | 100% | 100% |
| SIMD coverage | 15% | 60% | 80% |

---

## 7. Dependências & Riscos

### 7.1 Dependências
- **Rust std apenas**: Sem crates externos (exceto std)
- **CPU com AVX2**: Para otimizações SIMD (fallback para SSE2)
- **html5lib-tests dataset**: Apenas os arquivos JSON de teste (não o crate)
- **Benchmarking próprio**: Framework de benchmark 100% Albedo (sem Criterion)

### 7.2 Riscos

| Risco | Probabilidade | Impacto | Mitigação |
|-------|---------------|---------|-----------|
| Speculative parsing complexo | Alta | Alto | Implementar incremental, fallback single-thread |
| AVX-512 não disponível | Média | Baixo | Runtime detection, fallback AVX2/SSE2 |
| html5lib 100% difícil | Alta | Médio | Priorizar casos comuns, edge cases depois |
| Regressões de performance | Média | Alto | CI com benchmarks, regression detection |
| Bugs de conformidade | Alta | Alto | Testes extensivos, fuzzing |

---

## 8. Cronograma Estimado

### Fase 1: Conformidade (4-6 semanas)
- Lexer states completos
- Tree builder insertion modes completos
- AAA completo
- Foster parenting robusto
- html5lib 100% pass rate

### Fase 2: Performance Core (6-8 semanas)
- SIMD avançado (AVX-512)
- Zero-copy strings
- Arena allocator otimizado
- Throughput 300+ MB/s

### Fase 3: Features Avançadas (8-10 semanas)
- Speculative parsing
- Preload scanner robusto
- Streaming incremental otimizado
- Throughput 500+ MB/s

### Fase 4: Polish & Benchmarking (2-4 semanas)
- Benchmarks completos
- Profiling e otimizações finais
- Documentação
- Comparação com Chrome/Firefox

**Total**: 20-28 semanas (~5-7 meses)

---

## 9. Critérios de Aceite Finais

### 9.1 Performance
- [ ] Throughput ≥ 500 MB/s (Wikipedia homepage)
- [ ] Latência < 1ms por chunk (16KB)
- [ ] Memória ≤ 50% do Chrome (10 MB doc)
- [ ] Speedup 2-3x com speculative parsing

### 9.2 Conformidade
- [ ] html5lib-tests: 100% pass rate
- [ ] WPT: > 99% pass rate (HTML parsing)
- [ ] Comportamento idêntico ao Chrome em 99.9% dos casos

### 9.3 Features
- [ ] Speculative parsing funcional
- [ ] Preload scanner robusto
- [ ] SIMD AVX-512 funcional
- [ ] Zero-copy strings funcional

### 9.4 Qualidade
- [ ] Cobertura de testes > 95%
- [ ] Documentação 100% APIs públicas
- [ ] Benchmarks completos (micro + macro)
- [ ] CI com regression detection

---

## 10. Referências

### 10.1 Especificações
- [WHATWG HTML Living Standard](https://html.spec.whatwg.org/)
- [html5lib-tests dataset](https://github.com/html5lib/html5lib-tests) - apenas arquivos JSON
- [Web Platform Tests](https://github.com/web-platform-tests/wpt) - apenas casos de teste

**IMPORTANTE**: Usaremos apenas os **datasets de teste** (arquivos JSON/HTML), NÃO as bibliotecas. Todo o código de parsing, validação e benchmarking será **100% próprio do Albedo**.

### 10.2 Implementações de Referência
- [Chromium/Blink HTMLTokenizer](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/html/parser/)
- [Firefox/Gecko nsHtml5Tokenizer](https://searchfox.org/mozilla-central/source/parser/html)
- [WebKit HTMLTokenizer](https://github.com/WebKit/WebKit/tree/main/Source/WebCore/html/parser)

### 10.3 Papers & Artigos
- "Fast and Parallel HTML5 Parsing" (Google, 2013)
- "Speculative Parsing in Web Browsers" (Mozilla, 2015)
- "SIMD Optimizations for HTML Parsing" (Intel, 2018)

---

**Documento criado**: 2026-04-06  
**Versão**: 1.0  
**Autor**: Kiro AI Assistant  
**Status**: Draft → Aguardando revisão
