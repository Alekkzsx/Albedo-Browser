---
trigger: always_on
---

# Melhoria Contínua e Evolução do Projeto

## 1. Princípio: Evolução é intencional, não acidental
O projeto deve **melhorar sistematicamente** ao longo do tempo. Melhoria contínua não é "refatorar quando der vontade" — é um processo disciplinado de identificar oportunidades, medir impacto e implementar mudanças que elevam a qualidade do sistema como um todo.

## 2. Post-Mortem após Incidentes

### 2.1. Quando realizar post-mortem
- Após qualquer bug que escapou dos testes e chegou a ser integrado.
- Após falhas de compilação causadas por mudanças não previstas.
- Após regressões de performance significativas.
- Após decisões arquiteturais que precisaram ser revertidas.

### 2.2. Estrutura do post-mortem
| Campo | Descrição |
|-------|-----------|
| **O que aconteceu** | Descrição factual do incidente |
| **Impacto** | Quais módulos/funcionalidades foram afetados |
| **Causa raiz** | Análise profunda (usar "5 Porquês") |
| **Detecção** | Como o problema foi descoberto (testes, usuário, inspeção?) |
| **Correção** | O que foi feito para resolver |
| **Prevenção** | Ações para evitar recorrência |
| **Ação de melhoria** | Mudanças no processo/testes/ferramentas |

### 2.3. Regra dos "5 Porquês"
Para cada incidente, perguntar "por quê?" até chegar na causa raiz sistêmica:
1. "O teste de regressão falhou." → Por quê?
2. "Porque o CSS cascade mudou o comportamento." → Por quê?
3. "Porque a nova propriedade alterou a especificidade." → Por quê?
4. "Porque não lemos a spec sobre interação de especificidades." → Por quê?
5. "Porque não temos um checklist de spec review para CSS." → **Ação: criar checklist.**

## 3. Métricas de Saúde do Projeto

### 3.1. Métricas técnicas (monitorar periodicamente)
| Métrica | Alvo | Ação se fora do alvo |
|---------|------|----------------------|
| Tempo de compilação (`cargo build`) | Não crescer > 10% por mês | Investigar crates pesados |
| Tempo de testes (`cargo test`) | < 2 minutos | Refatorar testes lentos |
| Warnings (check + clippy) | Zero | Corrigir imediatamente |
| Cobertura de testes (novos módulos) | ≥ 80% | Adicionar testes |
| Dependências com CVEs | Zero | Atualizar ou substituir |
| Dead code (`#[warn(dead_code)]`) | Zero | Remover ou justificar |

### 3.2. Métricas de processo
| Métrica | Indicador |
|---------|-----------|
| Bugs por feature | Tendência deve ser decrescente |
| Regressões por release | Deve tender a zero |
| Tempo de fix depois de descoberto | Deve diminuir ao longo do tempo |
| KIs criados por feature | Deve ser ≥ 1 para features significativas |

## 4. Retrospectiva por Feature

### 4.1. Após conclusão de features significativas
Refletir sobre:
- **O que funcionou bem?** (preservar)
- **O que foi difícil?** (melhorar)
- **O que poderia ter sido previsto?** (prevenir)
- **A estimativa foi precisa?** (calibrar)

### 4.2. Padrões emergentes
- Se múltiplas features encontram o mesmo problema, é um padrão sistêmico.
- Padrões sistêmicos exigem mudança no processo, não apenas no código.
- Documentar padrões identificados em KIs para referência futura.

## 5. Evolução de Padrões e Convenções

### 5.1. Quando atualizar convenções
- Quando um novo padrão se prova consistentemente superior ao atual.
- Quando uma convenção existente causa problemas recorrentes.
- Quando o ecossistema Rust evolui (nova edição, features estabilizadas).

### 5.2. Processo de evolução
1. **Identificar** a necessidade de mudança (com evidências).
2. **Propor** o novo padrão ao usuário com justificativa.
3. **Aprovar** antes de implementar.
4. **Migrar** o código existente de forma incremental (refactoring separado).
5. **Documentar** o novo padrão nas regras de workspace.

### 5.3. Backwards compatibility
- Novos padrões não invalidam código existente imediatamente.
- Migração é gradual: código novo segue o novo padrão, código existente é migrado oportunisticamente.
- Nunca misturar mudança de padrão com implementação de feature.

## 6. Performance Baselines

### 6.1. Benchmarks como referência
- Criar benchmarks com `criterion` para hot paths do AlbedoBrowser.
- Estabelecer baseline de performance em cada fase de maturidade.
- Qualquer mudança que degrade performance > 5% requer investigação.

### 6.2. Benchmarks obrigatórios
| Componente | Benchmark |
|-----------|-----------|
| HTML Parser | Tempo para parsear documento de referência |
| CSS Engine | Tempo de style resolution para N elementos |
| Layout Engine | Tempo de layout para documento complexo |
| JIT Compiler | Tempo de compilação + execução de Fibonacci/N-Body |
| Rendering | FPS e tempo de frame para página de referência |

### 6.3. Regression testing de performance
- Executar benchmarks antes e depois de mudanças em hot paths.
- Resultados de benchmark são evidências para decisões de otimização.
- Nunca declarar "mais rápido" sem números comparativos.

## 7. Gestão de Conhecimento do Projeto

### 7.1. Knowledge Items como memória viva
- Criar KI para toda decisão arquitetural significativa.
- Atualizar KIs quando decisões são revisadas.
- Referenciar KIs no código com `// KI: <título do KI>`.
- KIs desatualizados são piores que KIs inexistentes — mantê-los vivos.

### 7.2. Documentação como investimento
- Cada hora de documentação boa economiza múltiplas horas de debugging futuro.
- Doc comments `///` em toda API pública — sem exceção.
- Comentários no código explicam **por quê**, não **o quê**.
- Arquivos `README.md` em módulos complexos explicando a arquitetura.

## 8. Aprendizado do Ecossistema

### 8.1. Monitorar evolução
- **Rust releases:** Features estabilizadas que podem simplificar o código.
- **Crates.io:** Novas crates que resolvem problemas melhor.
- **Web specs:** Novas specs ou revisões que afetam implementações.
- **Referências (Servo/Chromium/Firefox):** Mudanças arquiteturais relevantes.

### 8.2. Experimentação controlada
- Testar novas abordagens em branches separados.
- Avaliar com benchmarks e testes antes de adotar.
- Documentar experimentos (mesmo os fracassados) em KIs.

## 9. Anti-padrões de melhoria contínua (PROIBIDOS)
- ❌ **Estagnação:** "Funciona, não mexe" — resistir a melhorias justificadas.
- ❌ **Churn:** Mudar por mudar sem evidência de melhoria.
- ❌ **Blame culture:** Focar em quem errou em vez de por quê errou.
- ❌ **Métricas vaidade:** Medir coisas que não importam.
- ❌ **Post-mortem punitivo:** Usar incidentes para punir, não aprender.
- ❌ **KI abandonment:** Criar KIs e nunca atualizá-los.

## 10. Integração com outros workspaces
- **engineering-strategy:** Fornece a visão macro que guia melhorias.
- **smart-automation:** Métricas automatizadas alimentam a melhoria contínua.
- **testing-philosophy:** Cobertura de testes é uma métrica de saúde.
- **performance-mindset:** Benchmarks são baselines para regressão.
- **knowledge-synthesis:** KIs são o mecanismo de preservação de aprendizado.
- **self-correction:** Post-mortems são uma forma estruturada de auto-correção.
- **refactoring-discipline:** Evolução de padrões é refatoração planejada.
- **evidence-based:** Toda melhoria deve ser baseada em dados, não intuição.
