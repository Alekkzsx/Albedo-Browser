---
trigger: always_on
---

# Análise de Modos de Falha (FMEA Simplificada)

## 1. Princípio: Prever falhas antes que aconteçam
O agente **deve** analisar sistematicamente como o código pode falhar antes de implementar mudanças significativas. É mais barato prevenir uma falha no design do que corrigi-la em produção. Todo sistema complexo falha — a questão é se você antecipou onde.

## 2. Quando Aplicar FMEA

### 2.1. Obrigatório
- Mudanças em código concorrente (threads, async, shared state).
- Adição de novos subsistemas ou módulos.
- Alterações em código de parsing de input externo.
- Mudanças que afetam a pipeline de rendering.
- Alterações em código de rede ou I/O.
- Qualquer mudança que um `panic!` poderia alcançar em produção.

### 2.2. Recomendado
- Refatorações que alteram módulos com muitas dependências.
- Mudanças de performance em hot paths.
- Alterações em APIs públicas.

## 3. Processo de FMEA Simplificada

### 3.1. Tabela de Análise de Falhas
Para cada componente/mudança, listar:

| Modo de Falha | Causa | Efeito | Probabilidade | Severidade | Mitigação |
|---------------|-------|--------|---------------|------------|-----------|
| O que pode falhar | Por que falharia | Consequência | Baixa/Média/Alta | Baixa/Média/Alta | Como prevenir |

### 3.2. Cenários "E se...?" obrigatórios
Para toda mudança significativa, responder:
- "E se o input for **vazio**?"
- "E se o input for **gigante** (100MB+)?"
- "E se o input for **malformado**?"
- "E se o recurso **não existir**?"
- "E se a operação **demorar demais**?"
- "E se a **memória acabar**?"
- "E se duas threads acessarem **simultaneamente**?"
- "E se o sistema for **interrompido** no meio da operação?"

### 3.3. Análise de invariantes
- Listar **TODAS** as invariantes do código afetado.
- Para cada invariante, verificar: a mudança pode violá-la?
- Se sim: é intencional? Documentar a nova invariante.
- Se não: inserir `debug_assert!` para verificar em desenvolvimento.

## 4. Impacto em Cascata

### 4.1. Propagação de falhas
- Se módulo A falha, quais módulos B, C, D são afetados?
- Existe isolamento suficiente entre os módulos?
- A falha é contida ou se propaga pelo sistema?

### 4.2. Plano de rollback
Para mudanças de alto risco:
- Como reverter a mudança se causar problemas?
- Os dados do usuário são preservados em caso de falha?
- Existe um estado "safe mode" para o sistema operar degradado?

## 5. Categorias de Falha por Domínio

### 5.1. Parser e Input
- Input vazio, trucado, ou com encoding inválido.
- HTML/CSS/JS malformado ou adversarial.
- Recursão infinita (tags aninhadas infinitamente).
- Entities e escape sequences maliciosas.

### 5.2. Concorrência
- Race conditions em acesso a estado compartilhado.
- Deadlocks por ordem de aquisição de locks.
- Starvation de threads por prioridade incorreta.
- Use-after-free em contexto FFI/async.

### 5.3. Memória e Recursos
- Vazamento de memória por referências circulares.
- Exhaustão de file descriptors.
- Stack overflow por recursão profunda.
- Alocação excessiva por input adversarial.

### 5.4. Rede e I/O
- Timeout em operações de rede.
- Certificado SSL inválido ou expirado.
- Respostas parciais ou corrompidas.
- DNS que não resolve.

## 6. Anti-padrões de análise de falha (PROIBIDOS)
- ❌ **Otimismo cego:** "Isso nunca vai falhar."
- ❌ **Análise superficial:** Considerar apenas o happy path.
- ❌ **Ignorar concorrência:** Assumir que threads não interferem.
- ❌ **Confiar no input:** Assumir que dados externos são bem-formados.
- ❌ **Deletar testes que falham:** Esconder o problema não o resolve.

## 7. Integração com outros workspaces
- **deep-reasoning:** FMEA é uma aplicação direta de raciocínio profundo.
- **defensive-programming:** A análise de falhas informa onde colocar defesas.
- **error-handling:** Modos de falha definem quais erros o código deve tratar.
- **testing-philosophy:** Cada modo de falha identificado deve gerar um teste.
