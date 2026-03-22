---
trigger: always_on
---

## 1. Princípio: Entregar sempre 100% completo
Toda tarefa assumida deve ser executada **de forma integral**, atendendo a todos os requisitos, critérios de aceite e expectativas. Nada de soluções parciais, "esqueletos" ou entregas que deixem pendências para depois. O objetivo é que, ao final, a tarefa esteja pronta para ser considerada concluída sem necessidade de retrabalho imediato.

## 2. O que significa "100% completo"
Uma tarefa é considerada completa quando:

- **Requisitos atendidos:** Todos os objetivos funcionais e não funcionais definidos na tarefa (e nas subtarefas) foram implementados.
- **Testes realizados:** A tarefa passou pela validação completa do workspace **review-full** (quatro aspectos: funcional, qualidade técnica, impacto sistêmico, segurança).
- **Documentação atualizada:** Qualquer documentação afetada (código, README, comentários, API docs) foi ajustada para refletir as mudanças.
- **Sem pendências:** Não existem `TODO`, `FIXME`, comentários de código não finalizado, ou funcionalidades pela metade.
- **Integração verificada:** A alteração se integra corretamente ao restante do sistema, sem regressões.
- **Pronto para entrega:** A solução pode ser disponibilizada (commit, PR, deploy) sem necessidade de intervenção manual adicional.

## 3. Definition of Done (Checklist Obrigatório)
Antes de declarar uma tarefa como completa, verificar TODOS os itens:

### 3.1. Código
- [ ] Compila sem warnings (`cargo check`)
- [ ] Lint limpo (`cargo clippy -- -D warnings`)
- [ ] Formatação correta (`cargo fmt`)
- [ ] Sem `unwrap()` em código de produção
- [ ] Sem `todo!()` ou `unimplemented!()`
- [ ] Doc comments `///` em toda API pública nova

### 3.2. Testes
- [ ] Testes unitários para toda lógica nova
- [ ] Testes de regressão para bugfixes
- [ ] Todos os testes passam (`cargo test`)
- [ ] Edge cases cobertos (input vazio, inválido, gigante)

### 3.3. Segurança
- [ ] Input externo validado em boundaries
- [ ] Sem informações sensíveis em logs ou erros
- [ ] Sem `unsafe` não justificado

### 3.4. Documentação
- [ ] Comentários em código não-óbvio
- [ ] README atualizado se API pública mudou
- [ ] KI criado/atualizado para decisões arquiteturais

## 4. Processo para garantir completude

### 4.1. Antes de iniciar
- Revisar a tarefa e suas subtarefas (via **divide-action**) para confirmar que o escopo completo está coberto.
- Identificar critérios de aceite e condições de "pronto" (definition of done).

### 4.2. Durante a execução
- Trabalhar nas subtarefas de forma incremental, mas sempre as concluindo integralmente antes de avançar para a próxima.
- A cada subtarefa finalizada, verificar se os critérios de conclusão foram atingidos.
- Executar o ciclo de verificação do **smart-automation** após cada mudança.
- Nunca deixar "marcadores" de trabalho futuro sem que sejam explicitamente tratados como parte da tarefa atual.

### 4.3. Antes de finalizar
- Executar o processo **review-full** para validar os quatro aspectos.
- Passar pelo Definition of Done checklist completo.
- Verificar a lista de subtarefas: todas devem estar marcadas como concluídas.
- Realizar uma auto-inspeção para garantir que nada foi esquecido.

### 4.4. Comunicação da conclusão
- Informar ao usuário que a tarefa está 100% completa, anexando evidências (ex.: testes passando, documentação atualizada).
- Se houver qualquer impedimento que impeça a completude, comunicar imediatamente e propor alternativas.

## 5. Comportamento diante de impedimentos
- **Bloqueios externos:** Se uma dependência impedir a conclusão:
  - Interromper o trabalho.
  - Explicar claramente o que falta e o que é necessário para desbloquear.
  - Aguardar orientação antes de prosseguir.
- **Complexidade inesperada:** Se a tarefa se revelar maior que o previsto:
  - Revisar a divisão em subtarefas.
  - Comunicar o aumento de escopo e solicitar confirmação.
  - Nunca entregar uma solução incompleta sob o argumento de "já fiz a parte principal".

## 6. Anti-padrões de completude (PROIBIDOS)
- ❌ **Entrega esqueleto:** Código que compila mas não faz nada útil.
- ❌ **Testes ausentes:** "Funciona na minha máquina" não é evidência.
- ❌ **TODO permanente:** `TODO` que nunca será resolvido na tarefa atual.
- ❌ **Documentação atrasada:** "Documento depois" — documentar junto.
- ❌ **Declaração prematura:** Dizer "pronto" sem ter verificado o checklist.

## 7. Integração com outros workspaces
- **divide-action:** Fornece a decomposição que permite acompanhar a completude por partes.
- **review-full:** Garante que a validação de qualidade e impacto seja realizada antes de considerar a tarefa finalizada.
- **smart-automation:** O ciclo de verificação automatizado sustenta a definition of done.
- **testing-philosophy:** Testes são pré-requisito fundamental de completude.
- **understand-task / understand-first:** Fornecem o contexto necessário para saber o que é considerado "completo".
