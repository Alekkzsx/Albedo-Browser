---
trigger: always_on
---

# Precisão Cirúrgica nas Mudanças

## 1. Princípio: Menor footprint possível
Toda alteração no código deve ser **a menor mudança necessária** para atingir o objetivo. Mudanças cirúrgicas são mais fáceis de entender, revisar, testar e reverter. Quanto menor a superfície de mudança, menor o risco.

## 2. Regras de Precisão

### 2.1. Escopo mínimo
- Alterar **apenas** os arquivos, funções e linhas estritamente necessárias.
- Se uma mudança toca mais de 3 arquivos, questionar: "Há uma forma mais localizada?"
- Se uma mudança toca mais de 100 linhas, considerar dividir em commits separados.

### 2.2. Sem efeitos colaterais
- Toda alteração deve ser **isolada e previsível**.
- Antes de alterar: mapear todas as dependências diretas e indiretas.
- Depois de alterar: verificar que NADA além do esperado mudou.
- Testes de regressão obrigatórios para confirmar isolamento.

### 2.3. Diff legível
- O diff da mudança deve **contar uma história**: lê-lo deve ser suficiente para entender o que foi feito e por quê.
- Evitar mudanças cosméticas misturadas com mudanças funcionais.
- Evitar reorganizações de código que poluam o diff.

### 2.4. Uma responsabilidade por mudança
- **NUNCA misturar** refatoração com implementação de feature.
- **NUNCA misturar** correção de bug com melhoria de performance.
- Cada alteração tem um único propósito, documentado no commit.

### 2.5. Reversibilidade
- Preferir mudanças fáceis de reverter.
- Se uma mudança é irreversível (ex.: migração de dados, mudança de schema), requerer aprovação explícita do usuário.
- Feature flags quando possível para mudanças de alto risco.

### 2.6. Preservação de invariantes
- Antes de alterar: listar as invariantes do código afetado.
- Após alterar: verificar que todas as invariantes continuam válidas.
- Se uma invariante precisar mudar, documentar explicitamente a mudança e o motivo.

## 3. Checklist de Precisão (antes de cada mudança)
- [ ] A mudança é a menor possível para o objetivo?
- [ ] Nenhuma mudança cosmética misturada com funcional?
- [ ] Todas as dependências mapeadas?
- [ ] Invariantes preservadas ou explicitamente alteradas?
- [ ] Testes de regressão confirmam isolamento?
- [ ] Diff é legível e auto-explicativo?

## 4. Anti-padrões de precisão (PROIBIDOS)
- ❌ **Shotgun surgery:** Tocar muitos arquivos para uma mudança simples.
- ❌ **Refatoração oportunista:** "Já que estou aqui, vou melhorar isso também."
- ❌ **Gold plating:** Adicionar funcionalidades não solicitadas.
- ❌ **Big bang:** Fazer todas as mudanças de uma vez sem commits intermediários.

## 5. Integração com outros workspaces
- **divide-action:** A divisão em subtarefas permite mudanças cirúrgicas em cada uma.
- **review-full:** A revisão verifica se a mudança é realmente mínima e isolada.
- **self-correction:** Se uma mudança ficou maior que o necessário, admitir e refatorar.
