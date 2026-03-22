---
trigger: always_on
---

# Consistência Obsessiva com Padrões

## 1. Princípio: Código novo deve ser indistinguível do existente
O agente **deve** seguir os padrões, convenções e estilos já estabelecidos no projeto como se fossem lei. Consistência não é apenas estética — é uma decisão de engenharia que reduz carga cognitiva, facilita manutenção e previne bugs.

## 2. Dimensões da Consistência

### 2.1. Naming Conventions
- **Funções/métodos:** Seguir exatamente o padrão já utilizado (`snake_case` em Rust).
- **Structs/enums:** `PascalCase`, com nomes descritivos e concisos.
- **Constantes:** `SCREAMING_SNAKE_CASE`.
- **Módulos:** Minúsculas, nomes curtos e claros.
- **Variáveis:** Descritivas — não usar `x`, `tmp`, `data` sem contexto.
- **Prefixos/sufixos:** Se o projeto usa `_impl`, `_handler`, `_builder`, manter o padrão.

### 2.2. Organização de Código
- **Ordem de itens em módulos:** Seguir a ordem existente (ex.: `use` → `const` → `struct` → `impl` → `fn` → `tests`).
- **Ordem de campos em structs:** Seguir a lógica do projeto (ex.: campo principal primeiro).
- **Organização de imports:** Agrupar conforme convenção existente (`std` → `extern crate` → `crate`).
- **Estrutura de diretórios:** Respeitar a hierarquia estabelecida.

### 2.3. Padrões de API
- **Assinaturas de função:** Seguir o mesmo padrão de parâmetros (ex.: `&self` vs `self`, referências vs owned).
- **Retorno de erros:** Usar o mesmo tipo de erro na mesma camada.
- **Builder pattern:** Se o projeto usa builders, toda nova struct complexa deve ter um.
- **Trait implementation:** Seguir a mesma ordem e estilo de implementação.

### 2.4. Padrões de Controle de Fluxo
- **Error handling:** Usar o mesmo estilo (`match` vs `if let` vs `?`).
- **Iteradores:** Se o projeto prefere `iter().map().collect()` sobre `for`, manter.
- **Closures vs funções nomeadas:** Seguir o precedente.
- **Guard clauses:** Se o projeto usa early return, manter.

### 2.5. Documentação e Comentários
- **Estilo de doc comments:** `///` para items públicos, `//` para internos.
- **Formato:** Se usa seções como `# Examples`, `# Panics`, `# Errors`, manter.
- **Quando comentar:** Seguir o nível de comentários do projeto (não mais, não menos).

## 3. Processo de Garantia de Consistência

### 3.1. Antes de escrever código novo
1. **Estudar 2-3 exemplos existentes** de código similar no projeto.
2. **Extrair o padrão:** Naming, estrutura, estilo de erro, testes.
3. **Replicar o padrão:** Código novo segue exatamente o precedente.

### 3.2. Ao encontrar inconsistências existentes
- **NÃO corrigir junto com a feature:** Inconsistências existentes são tarefa separada.
- **Documentar:** Registrar a inconsistência para refatoração futura.
- **Escolher o padrão mais recente** como referência, se houver conflito.

### 3.3. Ao introduzir um novo padrão
- **Justificar explicitamente** por que o padrão existente não serve.
- **Obter aprovação do usuário** antes de introduzir padrão novo.
- **Documentar o novo padrão** para que seja seguido no futuro.

## 4. Regra de Ouro
> Em caso de dúvida sobre como escrever algo, **encontre um exemplo existente no projeto e copie o estilo**. Nunca invente uma convenção quando já existe precedente.

## 5. Integração com outros workspaces
- **understand-first:** A análise inicial mapeia convenções e padrões do projeto.
- **code-quality:** Os padrões de código são a base sobre a qual a consistência opera.
- **review-full:** A revisão de qualidade técnica verifica aderência a padrões.
