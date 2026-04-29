# Plano de Melhoria: ACE-HTML Tree Construction

## Status Atual (29/04/2026)

### Testes: ✅ 100% PASSANDO

| Teste | Casos | Passou | Taxa |
|-------|-------|--------|------|
| Tree Construction (html5lib) | 1498 | 1498 | **100%** |
| Hard Cases | todos | todos | **100%** |
| Tokenizer Required | requisitos | requisitos | **100%** |

**Conclusão: O Tree Construction está FUNCIONANDO!**

---

## Áreas de Melhoria Identificadas

### 1. Throughput (Alta Prioridade)

**Estado atual**: ~9 MB/s (Rust) vs ~85 MB/s (Chrome)

**Causa raiz**:
- O `fast_path` (linha 985-1006) tem muitas restrições e raramente ativa
- html5ever faz muit's allocations中间
- Conversão html5ever → Albedo tem overhead

**Locais críticos**:
```
src/ace/html/mod.rs:985-1006  (is_fast_path_candidate)
src/ace/html/mod.rs:880-983   (try_fast_parse_document)
src/ace/html/html5ever_parser.rs:356 (parse_document_html5ever)
```

**Melhorias possívelis**:
1. Relaxar restrições do fast path (permitir mais tags)
2. Reduzir allocations no parsing
3. Usar interning eficiente (já tem `lasso`)

### 2. Memory (Média Prioridade)

**Estado atual**: ~36KB por caso

**Causa**:
- html5ever cria muitos nós intermediários
- Conversão faz clone desnecessário

**Locais críticos**:
```
src/ace/html/html5ever_parser.rs:658-706  (convert_node)
```

### 3. Código Morto (Baixa Prioridade)

Funções não usadas que aparecem nos warnings:

```
src/ace/html/mod.rs:1171  fn build_document_from_tokens
src/ace/html/mod.rs:1265  fn infer_namespace
src/ace/html/tree_builder.rs:53  (struct com campos não usados)
```

---

## Plano de Ataque

### Fase 1: Limpeza (Fácil)
- [ ] Remover funções não usadas
- [ ] Limpar warnings de código morto

### Fase 2: Fast Path (Média)
- [ ] Analisar restrições atuais do fast_path
- [ ] Relaxar gradualmente as restrições
- [ ] Adicionar mais casos onde fast_path é usado

### Fase 3: Performance (Avançada)
- [ ] Analisar allocations com profiling
- [ ] Implementar allocator personalizado
- [ ] Reduzir clone na conversão

---

## Prioridades Imediatas

1. **Manter 100%** - Não quebrar os testes
2. **Documentar estado** - Este documento
3. **Fast path** - Melhorar sem quebrar conformidade
4. **Memory** - Medir antes de otimizar

---

## Métricas-Alvo

| Métrica | Atual | Meta |
|--------|-------|------|
| Tree Construction | 100% | Manter ≥99% |
| Throughput | ~9 MB/s | ≥20 MB/s |
| Memory/case | ~36KB | ≤20KB |

---

## Ações Imediatas

### 1. Executar benchmarks
```bash
cargo run --release --bin ace_html_benchmark
cargo run --release --bin ace_html_memory_profile
```

### 2. Limpar código morto
```bash
cargo fix --lib -p albedo
```

### 3. Testar fast_path isolation
```bash
# Testar com HTML > 64KB que passa nas restrições
ACE_HTML_FAST_PATH_TEST=1 cargo test
```

---

*Documento criado em 29/04/2026*
*Status: Testes passando, área de melhoria: Performance*