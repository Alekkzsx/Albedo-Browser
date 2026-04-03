# Plano Realista: ACE-HTML vs Chrome/Blink

## ⚠️ Situação Atual Honesta

O ACE-HTML **NÃO** está em nível de produção comparável a Chrome/Blink ou Firefox/Gecko. Estamos abaixo em conformidade, robustez e maturidade operacional.

---

## 📉 Gaps Críticos Identificados

### 1. Estabilidade Básica
- [ ] **preload_scanner.rs** teve quebra recente — parser não está consolidado
- [ ] Falta ciclo fechado de guard-rails (build + conformidade + regressões)
- [ ] Panics em fuzzing ainda ocorrem

### 2. Conformidade WHATWG §13.2
Chrome/Firefox passam fluxos críticos de tokenizer + tree builder. ACE-HTML tem lacunas reais:

#### Tokenizer
- [ ] Entidades históricas em atributos
- [ ] Ambiguous ampersand handling
- [ ] Estados especiais incompletos

#### Tree Builder
- [ ] Adoption Agency Algorithm (AAA) — implementação incompleta
- [ ] Foster parenting — casos edge não cobertos
- [ ] Frameset insertion mode — não testado suficientemente
- [ ] Fragment parsing em contexto de tabela — falha em casos reais

### 3. Foreign Content (SVG/MathML)
- [ ] Integration points HTML→SVG imprecisos
- [ ] Integration points SVG→HTML com bugs
- [ ] MathML namespace transitions falham
- [ ] Casos de foreign content em tabelas não cobertos

### 4. `<template>` Ecosystem
- [ ] Elemento `<template>` existe, mas comportamento incompleto
- [ ] Fragment parsing com template contexts falha
- [ ] Casos tortos de template aninhado não testados
- [ ] Template em foreign content não implementado

### 5. Error Recovery
Blink/Gecko recuperam silenciosamente HTML malformado. ACE-HTML ainda:
- [ ] Emite `UnexpectedToken` desnecessário
- [ ] Emite `UnexpectedEof` onde engines maduros recuperam
- [ ] Não segue algoritmos de recovery corretamente
- [ ] Gera erros extras em páginas reais

### 6. Encoding Detection
- [ ] 52 codificações implementadas, mas...
- [ ] Sem compatibilidade histórica completa
- [ ] Detecção via HTTP headers incompleta
- [ ] BOM handling em contextos edge falha
- [ ] Meta tag parsing com cases reais falha

### 7. Preload Scanner
- [ ] Implementação existe, mas não é madura
- [ ] Não equivale ao pipeline de speculation de browsers grandes
- [ ] Integração com tokenizer instável
- [ ] Casos de streaming não cobertos

---

## 🧪 Suíte de Testes Pendente

### html5lib Tests
- [ ] **tree-construction** — amplo, centenas de casos falhando
- [ ] **tokenizer** — completo, gaps em estados especiais
- [ ] **fragment cases** — extensivos, especialmente em tables/templates

### Testes de Produção
- [ ] Regressões de páginas reais (top 1000 sites)
- [ ] Fuzzing pesado (1M+ inputs) sem panic
- [ ] Documentos gigantes (10MB+) performance
- [ ] Streaming parsing sob carga
- [ ] Inputs hostis (malicious HTML)

### Guard-Rails
- [ ] CI bloqueia regressão de conformidade
- [ ] CI bloqueia regressão de performance
- [ ] CI roda fuzzing contínuo
- [ ] CI testa páginas reais diariamente

---

## 🎯 Roadmap Realista

### Fase 0: Estabilização (IMEDIATO)
1. Fixar `preload_scanner.rs`
2. Rodar html5lib e catalogar falhas
3. Implementar logging detalhado de erros
4. Criar suite mínima de regressão

### Fase 1: Conformidade Básica (3-6 meses)
1. Fechar Adoption Agency Algorithm
2. Implementar foster parenting completo
3. Fixar entities em atributos
4. Resolver ambiguous ampersand
5. Completar frameset insertion mode

### Fase 2: Foreign Content (6-9 meses)
1. Integration points HTML↔SVG
2. Integration points HTML↔MathML
3. Foreign content em tables
4. Namespace transitions precisas

### Fase 3: Template Ecosystem (9-12 meses)
1. Fragment parsing com templates
2. Template nesting cases
3. Template em foreign content
4. Template serialization

### Fase 4: Error Recovery (12-15 meses)
1. Mapear todos os UnexpectedToken/Eof
2. Comparar comportamento com Blink/Gecko
3. Implementar recovery algorithms
4. Testar com páginas reais quebradas

### Fase 5: Encoding Completo (15-18 meses)
1. Compatibilidade histórica total
2. HTTP header detection robusta
3. Meta tag parsing perfeito
4. BOM em todos os contextos

### Fase 6: Preload Scanner Maduro (18-21 meses)
1. Pipeline de speculation completo
2. Integração estável com tokenizer
3. Streaming parsing robusto
4. Performance equivalente a browsers

### Fase 7: Produção Industrial (21-24 meses)
1. 100% html5lib verde
2. Zero panics em fuzzing (10M+ inputs)
3. Performance competitiva
4. Top 1000 sites renderizam sem errors
5. CI com guard-rails completos

---

## 📊 Métricas de Sucesso

| Métrica | Agora | Target |
|---------|-------|--------|
| html5lib tree-construction | <50% | 100% |
| html5lib tokenizer | ~91% | 100% |
| html5lib fragment | <30% | 100% |
| Panics em fuzzing | >0 | 0 |
| UnexpectedToken em páginas reais | Alto | ~0 |
| Top 1000 sites sem errors | <10% | >99% |
| Performance (tokens/sec) | ? | ≥Blink |

---

## 🔥 Prioridades Imediatas (Próximas 2 Semanas)

1. **Catalogar falhas html5lib** — rodar suite completa e listar TODOS os failures
2. **Fixar preload_scanner** — estabilizar módulo quebrado
3. **Implementar logging** — saber exatamente ONDE e POR QUE falhamos
4. **Criar regression tests** — mínimo de 10 casos críticos
5. **Documentar gaps** — issues no tracker com priorização clara

---

## 💬 Nota Final

Este plano é **honesto e realista**. Não estamos enganando ninguém sobre nosso estado. O ACE-HTML é ambicioso e tem partes importantes implementadas, mas ainda está **abaixo** de Chrome/Firefox em:

- ✅ Conformidade WHATWG
- ✅ Robustez (error recovery)
- ✅ Cobertura de testes
- ✅ Maturidade operacional
- ✅ Performance sob carga
- ✅ Compatibilidade histórica

**Objetivo**: Chegar a 100% de conformidade e robustez em 24 meses, com marcos claros e mensuráveis.

*Não há atalhos. Conformidade exige trabalho duro e teste exaustivo.*
