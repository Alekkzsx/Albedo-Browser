---
trigger: always_on
---

# Estratégias de Engenharia para Desenvolvimento de Browser

## 1. Princípio: Visão macro guia ação micro
O agente deve operar com uma **visão estratégica** do desenvolvimento do AlbedoBrowser, não apenas executando tarefas isoladas. Cada micro-decisão (tipo de dado, nome de função, algoritmo) deve ser guiada por uma compreensão macro (onde o projeto está, para onde vai, e quais são as prioridades).

## 2. Modelo de Maturidade do AlbedoBrowser

### 2.1. Fases de desenvolvimento
| Fase | Foco | Critério de Transição |
|------|------|-----------------------|
| **Foundation** | Arquitetura, tipos base, pipeline skeleton | Core types compilam e conectam |
| **MVP** | Funcionalidade mínima end-to-end | Renderiza HTML básico com CSS inline |
| **Alpha** | Features essenciais, standards parcial | Passa 30%+ dos web platform tests |
| **Beta** | Compatibilidade, performance, estabilidade | Passa 70%+ WPT, benchmarks estáveis |
| **Stable** | Polish, compliance completo, otimizações | Passa 95%+ WPT, performance competitiva |

### 2.2. Identificar a fase atual
- Antes de implementar qualquer feature, o agente deve identificar em qual fase o projeto se encontra.
- **Decisões são relativas à fase:** Na fase Foundation, priorizar tipos corretos sobre performance. Na fase Beta, priorizar performance sobre novas features.

## 3. Hierarquia de Prioridades

### 3.1. Ordem de prioridade (invariável)
1. **Correção** — O código faz o que a spec diz? (correctness > features)
2. **Segurança** — O código é seguro contra input malicioso?
3. **Estabilidade** — O código não crasha? (robustez > features)
4. **Manutenibilidade** — O código é legível e modificável?
5. **Performance** — O código é rápido o suficiente?
6. **Features** — O código tem todas as funcionalidades desejadas?

### 3.2. Trade-offs entre prioridades
- Performance NUNCA justifica sacrificar correção ou segurança.
- Features novas NUNCA justificam introduzir instabilidade.
- Exceções requerem aprovação explícita do usuário com documentação.

## 4. Pipeline de Desenvolvimento de Features

### 4.1. Fluxo padrão para features de browser
```
Spec Research → Prototype → Implement → Test → Optimize → Document
```

1. **Spec Research:** Ler a spec WHATWG/W3C. Entender edge cases e interações.
2. **Prototype:** Implementação mínima focada em correção, sem otimizações.
3. **Implement:** Implementação completa com error handling, edge cases, testes.
4. **Test:** Web Platform Tests, testes unitários, fuzzing de input.
5. **Optimize:** Profiling, benchmarks, otimizações guiadas por dados.
6. **Document:** Doc comments, KIs para decisões arquiteturais.

### 4.2. Nunca pular etapas
- Implementar sem ler a spec → bugs de conformidade.
- Otimizar sem testar → otimização de código incorreto.
- Documentar "depois" → dívida técnica acumulada.

## 5. Gestão de Dívida Técnica

### 5.1. Classificação de dívida
| Tipo | Severidade | Ação |
|------|-----------|------|
| **Deliberada-prudente** | Baixa | "Sabemos que é subótimo, planejamos melhorar na fase X" |
| **Deliberada-imprudente** | Alta | "Não temos tempo, fazemos direito depois" — PROIBIDO |
| **Acidental-prudente** | Média | "Aprendemos algo novo, o código anterior pode melhorar" |
| **Acidental-imprudente** | Crítica | "Não sabíamos que era ruim" — corrigir imediatamente |

### 5.2. Tracking de dívida
- Toda dívida técnica deliberada deve ser registrada com:
  - **O quê:** Descrição do compromisso.
  - **Por quê:** Justificativa para não fazer direito agora.
  - **Quando:** Prazo ou condição para resolver.
  - **Impacto:** O que piora enquanto a dívida existir.
- Usar comentários `// TECH_DEBT:` no código para rastreabilidade.

### 5.3. Pagamento de dívida
- Reservar tempo para pagar dívida técnica em cada ciclo de desenvolvimento.
- Priorizar dívida que bloqueia features futuras.
- Nunca acumular dívida sobre dívida (debt compounding).

## 6. Gestão de Dependências (Crates)

### 6.1. Critérios de seleção de crates
Antes de adicionar uma nova dependência, avaliar:
- **Manutenção:** Último commit < 6 meses? Issues respondidas?
- **Popularidade:** Downloads, estrelas, adoção pela comunidade.
- **Segurança:** Histórico de CVEs? Cargo audit limpo?
- **Tamanho:** Tempo de compilação adicionado é justificado?
- **Alternativas:** Existe solução mais leve ou implementação própria viável?
- **Licença:** Compatível com a licença do AlbedoBrowser?

### 6.2. Princípio de minimização
- Preferir implementação própria para funcionalidades simples.
- Preferir crates com zero/poucas dependências transitivas.
- Evitar crates que puxam metade do ecossistema (ex.: `reqwest` para uma requisição simples).

### 6.3. Atualização de dependências
- Verificar atualizações periodicamente (`cargo outdated`).
- Patch versions: atualizar livremente.
- Minor versions: testar antes de aceitar.
- Major versions: planejar migração com análise de impacto.

## 7. Feature Flags e Entrega Incremental

### 7.1. Quando usar feature flags
- Features experimentais ou em progresso.
- Mudanças de alto risco que precisam de rollback fácil.
- A/B testing de diferentes implementações.

### 7.2. Implementação em Rust
```rust
// Feature flags via Cargo.toml
[features]
experimental_jit = []
css_grid = ["dep:grid-layout"]

// No código
#[cfg(feature = "experimental_jit")]
mod jit_compiler;
```

### 7.3. Entrega incremental
- Implementar features em fatias verticais (end-to-end fina) em vez de camadas horizontais.
- Cada incremento deve ser funcional e testável independentemente.
- Preferir "feature completa com escopo reduzido" sobre "feature ampla parcialmente implementada".

## 8. Web Standards Compliance

### 8.1. Compliance como métrica
- Usar **Web Platform Tests (WPT)** como referência de conformidade.
- Medir percentual de testes passando por categoria (HTML, CSS, DOM, JS).
- Definir metas de compliance por fase de maturidade.

### 8.2. Quando desviar da spec
- **Nunca** desviar sem documentar.
- Razões aceitáveis: performance crítica, limitação temporária, comportamento de segurança.
- Documentar o desvio com `// SPEC_DEVIATION:` e link para a spec.

## 9. Anti-padrões estratégicos (PROIBIDOS)
- ❌ **Feature-first:** Adicionar features sem fundação sólida.
- ❌ **Big bang delivery:** Acumular meses de trabalho sem entrega intermediária.
- ❌ **Dependency hell:** Adicionar crates sem avaliar impacto.
- ❌ **Spec ignorance:** Implementar WebAPIs sem consultar a spec.
- ❌ **Debt denial:** Ignorar dívida técnica "porque ainda funciona".
- ❌ **Premature optimization:** Otimizar antes de ter implementação correta.
- ❌ **Horizontal slicing:** Implementar toda a camada de network antes de ter rendering.

## 10. Integração com outros workspaces
- **context-mastery:** Fornece as fontes de specs e referências para o pipeline de features.
- **architecture-guard:** Define as boundaries que a estratégia deve respeitar.
- **performance-mindset:** Guia a fase de otimização do pipeline.
- **security-first:** Define prioridade permanente de segurança sobre features.
- **plan-mode:** O planejamento tático é guiado pela estratégia macro.
- **failure-analysis:** Informa decisões de priorização de risco.
- **continuous-improvement:** Alimenta retrospectivas e ajustes estratégicos.
