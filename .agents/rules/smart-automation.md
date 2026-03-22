---
trigger: always_on
---

# Automação Inteligente e Verificação Contínua

## 1. Princípio: Automatizar o que pode ser automatizado
O agente deve usar ferramentas automatizadas para verificar, validar e garantir qualidade continuamente. Automação não substitui raciocínio — complementa-o. O que pode ser verificado por máquina, deve ser verificado por máquina.

## 2. Ciclo de Verificação Obrigatório

### 2.1. Após TODA alteração de código
Executar na ordem estrita:
1. **`cargo fmt`** — Formatar código (consistência visual).
2. **`cargo check`** — Verificar compilação (zero warnings).
3. **`cargo clippy -- -D warnings`** — Lint (zero warnings como erros).
4. **`cargo test`** — Executar testes (todos devem passar).

### 2.2. Política de falhas
- Se **`cargo check`** falha: corrigir ANTES de continuar qualquer outra atividade.
- Se **`cargo clippy`** alerta: corrigir ANTES de considerar o código pronto.
- Se **`cargo test`** falha: investigar com `debugging-methodology`, não pular.
- **NUNCA** proceder com um passo falhado. O ciclo é sequencial e obrigatório.

### 2.3. Frequência
- **Após cada mudança funcional**: ciclo completo.
- **Após refatoração**: ciclo completo.
- **Antes de finalizar tarefa**: ciclo completo + revisão.

## 3. Verificações Periódicas

### 3.1. Auditoria de dependências
- `cargo audit` — verificar vulnerabilidades conhecidas em deps.
- Manter dependências atualizadas (sem major version bumps sem planejamento).
- Revisar `Cargo.lock` para deps transitivas inesperadas.

### 3.2. Monitoramento de compilação
- Observar tempos de compilação — alertar se crescerem significativamente.
- Identificar crates que adicionam tempo desproporcional.
- Considerar `cargo build --timings` para diagnóstico.

### 3.3. Detecção de dead code
- Verificar com `#[warn(dead_code)]` habilitado.
- Código não utilizado deve ser removido, não comentado.
- Funções públicas não referenciadas devem ser avaliadas.

## 4. Automação de Tarefas Repetitivas

### 4.1. Geração de boilerplate
- Quando um padrão se repete 3+ vezes, considerar criação de macro ou template.
- Documentar macros e templates para reuso.

### 4.2. Testes automatizados
- Gerar stubs de testes para novas funções públicas.
- Manter coverage reports atualizados.
- Integrar benchmarks em CI quando configurado.

## 5. Verificação de Integridade do Projeto

### 5.1. Checklist pré-entrega
- [ ] `cargo fmt --check` — Formatação correta.
- [ ] `cargo check` — Zero warnings.
- [ ] `cargo clippy -- -D warnings` — Lint limpo.
- [ ] `cargo test` — Todos os testes passam.
- [ ] `cargo doc --no-deps` — Documentação gera sem erros.
- [ ] Sem `TODO`, `FIXME` ou `todo!()` não documentados.

### 5.2. Checklist de saúde do projeto
- [ ] `Cargo.lock` atualizado e commitado.
- [ ] Sem dependências duplicadas (`cargo tree --duplicates`).
- [ ] Sem vulnerabilidades conhecidas (`cargo audit`).

## 6. Anti-padrões de automação (PROIBIDOS)
- ❌ **Pular verificação:** "Só mudei uma linha, não precisa testar."
- ❌ **Ignorar warnings:** "É só um warning, não é erro."
- ❌ **Desabilitar clippy lints:** Sem `#[allow()]` sem justificativa.
- ❌ **Testes manuais apenas:** Se pode ser automatizado, deve ser.
- ❌ **CI como desculpa:** "O CI vai pegar" — verificar ANTES de push.

## 7. Integração com outros workspaces
- **code-quality:** O ciclo de verificação sustenta os padrões de qualidade.
- **review-full:** A revisão utiliza as ferramentas automatizadas.
- **evidence-based:** Resultados automatizados são evidências objetivas.
- **testing-philosophy:** Testes automatizados são parte da filosofia de testes.
