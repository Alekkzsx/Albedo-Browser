---
trigger: always_on
---

## 1. Princípio: Nada é entregue sem validação
Antes de finalizar e entregar qualquer tarefa, o agente **deve** testar o código produzido para garantir que não contém erros e atende aos critérios definidos. A validação será conduzida em **quatro aspectos complementares**. Somente após a aprovação em todos eles a tarefa será considerada concluída.

## 2. Os quatro aspectos da validação

### 2.1. Aspecto 1 – Correção funcional
- **Objetivo:** Verificar se o código implementa exatamente o que foi solicitado, sem desvios de escopo.
- **Atividades:**
  - Executar testes automatizados (unitários, integração) existentes e novos criados para a tarefa.
  - Realizar testes manuais ou exploratórios nos fluxos principais e bordas (edge cases).
  - Validar os critérios de aceite documentados na tarefa.
  - Para código de parsing: testar com input malformado, vazio e gigante.
- **Critério de aprovação:** Todos os testes passam; comportamento está de acordo com a especificação.

### 2.2. Aspecto 2 – Qualidade técnica
- **Objetivo:** Garantir que o código é limpo, seguro e mantível.
- **Atividades (ciclo smart-automation obrigatório):**
  1. `cargo fmt` — Formatação consistente.
  2. `cargo check` — Zero warnings.
  3. `cargo clippy -- -D warnings` — Lint com warnings como erros.
  4. `cargo doc --no-deps` — Documentação compila sem erros.
  - Verificar aderência às convenções de código (**pattern-consistency**).
  - Avaliar complexidade ciclomática (**code-quality**: máx. 10 por função).
  - Verificar error handling (**error-handling**: sem `unwrap()` em produção).
- **Critério de aprovação:** Zero alertas; código segue os padrões estabelecidos.

### 2.3. Aspecto 3 – Impacto sistêmico
- **Objetivo:** Assegurar que a alteração não introduz regressões e se integra corretamente ao restante do sistema.
- **Atividades:**
  - Executar `cargo test` (suíte completa) para confirmar zero regressões.
  - Verificar que as **boundaries arquiteturais** foram respeitadas (**architecture-guard**).
  - Avaliar impacto em performance (tempo de resposta, uso de recursos) e escalabilidade, quando aplicável.
  - Se houver dependências externas, confirmar que a integração permanece íntegra.
- **Critério de aprovação:** Nenhuma regressão identificada; sistema se comporta conforme esperado.

### 2.4. Aspecto 4 – Segurança
- **Objetivo:** Garantir que a mudança não introduz vulnerabilidades de segurança.
- **Atividades:**
  - Verificar validação de input em boundaries (rede, arquivos, user input).
  - Confirmar que dados sensíveis não vazam em logs ou mensagens de erro.
  - Verificar conformidade com Same-Origin Policy e CSP (se aplicável).
  - Verificar se blocos `unsafe` são justificados e documentados com `// SAFETY:`.
  - Executar `cargo audit` para vulnerabilidades em dependências.
- **Critério de aprovação:** Sem vulnerabilidades identificadas; input externo validado.

## 3. Processo de revisão

### 3.1. Execução dos testes
- O agente deve executar as verificações de cada aspecto de forma sistemática, documentando os resultados.
- Em caso de falha em qualquer aspecto, o agente deve:
  - Corrigir os problemas identificados usando **debugging-methodology**.
  - Reexecutar os testes (ciclo até aprovação).
  - Aplicar **self-correction** se a abordagem inteira precisar mudar.

### 3.2. Checklist de aprovação final
- [ ] **Funcional:** Todos os testes passam + edge cases cobertos.
- [ ] **Qualidade:** `cargo fmt` + `check` + `clippy` + `doc` = zero issues.
- [ ] **Sistêmico:** `cargo test` completo passa + nenhuma regressão.
- [ ] **Segurança:** Input validado + sem info leak + cargo audit limpo.

### 3.3. Documentação da revisão
Após a conclusão bem-sucedida, o agente deve gerar um relatório sucinto contendo:
- **Resultado por aspecto:** status (aprovado/reprovado) e evidências (ex.: testes passaram, linter ok).
- **Observações relevantes:** pontos de atenção futuros, melhorias sugeridas.
- **Confirmação de finalização:** tarefa marcada como concluída.

## 4. Anti-padrões de revisão (PROIBIDOS)
- ❌ **Revisão cosmética:** Olhar o código e dizer "parece bom" sem executar testes.
- ❌ **Pular aspecto:** Omitir a verificação de segurança "porque é interno".
- ❌ **Ignorar warnings:** "É só um warning, não é erro" — todo warning é um bug potencial.
- ❌ **Testes seletivos:** Rodar apenas os testes novos, ignorando regressão.
- ❌ **Aprovar com TODO:** Considerar aprovado com `todo!()` ou `FIXME` pendentes.

## 5. Exceções – quando não é possível testar integralmente
Se, por restrições técnicas ou contextuais, não for possível executar um ou mais aspectos (ex.: ambiente de teste indisponível, falta de testes automatizados, impossibilidade de testar regressão), o agente **deve**:

1. **Explicitar claramente ao usuário** quais aspectos não puderam ser validados e o motivo.
2. **Propor alternativas** (ex.: testes manuais guiados, análise de impacto teórica, verificação em ambiente de homologação posterior).
3. **Solicitar orientação** sobre como prosseguir, antes de considerar a tarefa finalizada.

Nesses casos, a tarefa não será marcada como "finalizada" sem a anuência explícita do usuário.

## 6. Integração com outros workspaces
- **smart-automation:** O ciclo de verificação automatizado é a base do Aspecto 2.
- **testing-philosophy:** Define o que testar, como testar e com que cobertura.
- **security-first:** Define os critérios de validação do Aspecto 4.
- **code-quality:** Define os padrões de qualidade para o Aspecto 2.
- **architecture-guard:** Verifica boundaries no Aspecto 3.
- **debugging-methodology:** Usado quando qualquer aspecto falha.
- **self-correction:** Ativado quando a revisão revela problemas fundamentais.
- **evidence-based:** Toda aprovação deve ser baseada em evidências concretas (testes, métricas).
- **make-full:** O review-full é pré-requisito da completude.