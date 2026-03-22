---
trigger: always_on
---

# Disciplina de Refatoração

## 1. Princípio: Refatorar é transformar, não reescrever
Refatoração é a prática disciplinada de melhorar a estrutura interna do código **sem alterar seu comportamento externo**. É uma ferramenta de manutenção, não uma desculpa para reescrever tudo. Toda refatoração deve ser motivada, planejada e verificada.

## 2. Regras Fundamentais

### 2.1. Separação absoluta
- **NUNCA** misturar refatoração com adição de feature no mesmo commit/PR.
- **NUNCA** misturar refatoração com correção de bug.
- Cada refatoração é uma mudança isolada, verificável e reversível.

### 2.2. Rede de segurança obrigatória
- Testes devem existir **ANTES** de refatorar.
- Se não existem testes, criar testes primeiro (commit separado), depois refatorar.
- Executar todos os testes **antes** e **depois** de cada passo de refatoração.

### 2.3. Passos atômicos
- Cada passo de refatoração deve ser pequeno e verificável:
  1. Fazer UMA mudança estrutural.
  2. Verificar que compila (`cargo check`).
  3. Verificar que testes passam (`cargo test`).
  4. Repetir.
- Se um passo quebra algo, reverter e tentar menor.

## 3. Quando Refatorar

### 3.1. Triggers legítimos
- **Duplicação:** Código copiado 3+ vezes → extrair para função/módulo.
- **Função longa:** Função com 50+ linhas → decompor.
- **God struct:** Struct com 15+ campos → separar responsabilidades.
- **Feature envy:** Função que usa mais dados de outro módulo que do próprio.
- **Shotgun surgery:** Mudar uma feature requer tocar 5+ arquivos.
- **Naming ruim:** Nome que não comunica o propósito.
- **Antes de adicionar feature:** Se a estrutura atual torna a feature difícil, refatorar primeiro.

### 3.2. Triggers ilegítimos (NÃO refatorar por estes motivos)
- "Eu faria diferente" — respeitar decisões anteriores se funcionam.
- "Essa tech é mais nova" — não trocar tecnologia sem justificativa de negócio.
- "Está feio mas funciona" — estética sozinha não justifica risco de regressão.

## 4. Catálogo de Refatorações Comuns

| Refatoração | Quando usar |
|-------------|------------|
| Extract Function | Código duplicado ou bloco com responsabilidade própria |
| Extract Module | Arquivo com 500+ linhas ou múltiplas responsabilidades |
| Rename | Nome não comunicativo ou ambíguo |
| Move | Código no módulo errado (violação de boundary) |
| Inline | Abstração desnecessária (wrappers vazios) |
| Replace Conditional with Polymorphism | Chains de `if/else` ou `match` repetidos |
| Introduce Parameter Object | Funções com muitos parâmetros |
| Replace Magic Number with Named Constant | Valores numéricos sem contexto |

## 5. Boy Scout Rule (com medida)
- "Deixe o código melhor do que encontrou" — mas com limites.
- Refatorações oportunistas são permitidas APENAS se:
  - São pequenas (< 10 linhas).
  - Não mudam comportamento.
  - São no mesmo módulo que está sendo alterado.
  - Não poluem o diff da mudança principal.
- Se a refatoração é maior, criar tarefa separada.

## 6. Documentação de Refatoração
Para cada refatoração significativa, documentar:
- **O que mudou:** Descrição estrutural (ex.: "Extraí `parse_css_value` para módulo separado").
- **Por que:** Code smell ou necessidade que motivou.
- **Impacto:** Quais módulos foram afetados.
- **Testes:** Evidência de que o comportamento não mudou.

## 7. Anti-padrões de refatoração (PROIBIDOS)
- ❌ **Big bang refactor:** Mudar tudo de uma vez sem verificação incremental.
- ❌ **Refatorar sem testes:** Esperança de que "provavelmente está certo".
- ❌ **Refatoração escondida:** Misturar mudança estrutural com feature sem comunicar.
- ❌ **Over-abstraction:** Criar abstrações "para o futuro" sem necessidade presente.
- ❌ **Perfeccionismo paralisante:** Refatorar infinitamente em busca do código "perfeito".

## 8. Integração com outros workspaces
- **surgical-precision:** Refatoração é o exemplo máximo de mudança com precisão cirúrgica.
- **testing-philosophy:** Testes são pré-requisito e rede de segurança da refatoração.
- **divide-action:** Refatorações grandes devem ser divididas em passos atômicos.
- **pattern-consistency:** Refatoração deve manter ou melhorar a consistência de padrões.
