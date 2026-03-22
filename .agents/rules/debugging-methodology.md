---
trigger: always_on
---

# Metodologia Sistemática de Debugging

## 1. Princípio: Debugging é ciência, não adivinhação
Debugging não é tentativa e erro — é o **método científico** aplicado ao código. Formular hipótese, testar, analisar resultado, iterar. Debugging sem método é desperdiço de tempo e fonte de regressões.

## 2. Os 7 Passos do Debugging Sistemático

### Passo 1: REPRODUZIR
- **Antes de tudo**: Conseguir reproduzir o bug de forma **consistente**.
- Se não consegue reproduzir, não consegue confirmar a correção.
- Documentar os passos EXATOS para reproduzir.
- Criar um teste automatizado que demonstra o bug (se possível).

### Passo 2: ISOLAR
- Encontrar o **menor caso de teste** que reproduz o bug.
- Eliminar variáveis: simplificar input, remover componentes, desabilitar features.
- Quanto menor o caso de teste, mais fácil identificar a causa.

### Passo 3: FORMULAR HIPÓTESE
- Com base nas evidências, propor uma **causa provável**.
- A hipótese deve ser **falsificável**: "Se minha hipótese está correta, então X deve acontecer quando Y."
- Considerar múltiplas hipóteses quando a causa não é óbvia.

### Passo 4: VERIFICAR
- Testar a hipótese com **evidências concretas**:
  - Adicionar logs estratégicos (`tracing::debug!`).
  - Inserir `debug_assert!` para verificar suposições.
  - Usar `RUST_BACKTRACE=1` para stack traces completos.
  - Inspecionar variáveis em pontos-chave.
- Se a hipótese for refutada: voltar ao Passo 3 com novo conhecimento.

### Passo 5: CORRIGIR A RAIZ
- Corrigir a **causa raiz**, não o sintoma.
- Se o bug é "variável X está com valor errado", a correção é "por que X recebeu esse valor?", não "if X is wrong, set X to correct value".
- Verificar se a mesma causa raiz existe em outros locais do código.

### Passo 6: TESTAR
- Criar **teste de regressão** que falha sem o fix e passa com o fix.
- Executar a suite completa de testes para confirmar que não há regressões.
- Verificar o fix no contexto original do bug (não só no teste isolado).

### Passo 7: DOCUMENTAR
- Documentar no commit/PR:
  - **O que causou:** Explicação da causa raiz.
  - **Por que aconteceu:** Condições que levaram ao bug.
  - **Como foi corrigido:** Descrição da correção.
  - **Como prevenir:** O que fazer para evitar bugs similares no futuro.

## 3. Ferramentas de Debugging

### 3.1. Logging e Tracing
```rust
// Usar tracing para debugging estruturado
use tracing::{debug, error, info, trace, warn, instrument};

#[instrument(skip(large_data))]
fn process_data(id: u32, large_data: &[u8]) -> Result<Output> {
    debug!(id, data_len = large_data.len(), "Processing data");
    // ...
}
```

### 3.2. Variáveis de ambiente úteis
| Variável | Propósito |
|----------|-----------|
| `RUST_BACKTRACE=1` | Stack trace em panics |
| `RUST_BACKTRACE=full` | Stack trace completo com todas as frames |
| `RUST_LOG=debug` | Habilitar logs de debug |
| `RUST_LOG=module::path=trace` | Logs granulares por módulo |

### 3.3. Git bisect
Para bugs que "apareceram do nada":
```bash
git bisect start
git bisect bad       # commit atual está bugado
git bisect good <commit>  # último commit sabidamente bom
# Git navega binariamente no histórico
# Para cada commit: testar e marcar good/bad
```

### 3.4. Debug assertions
```rust
// Verificar invariantes em desenvolvimento
debug_assert!(index < vec.len(), "Index out of bounds: {} >= {}", index, vec.len());
debug_assert_eq!(expected, actual, "State mismatch after operation");
```

## 4. Categorias de Bug e Abordagens

### 4.1. Bugs determinísticos
- Mais fáceis: mesma entrada sempre produz mesmo bug.
- Abordagem: reproduzir → isolar → corrigir.

### 4.2. Bugs intermitentes (Heisenbugs)
- Mais difíceis: dependem de timing, concorrência ou estado global.
- Abordagem: adicionar logging extensivo → analisar padrões → usar ferramentas de concorrência (ThreadSanitizer, miri).

### 4.3. Bugs de performance
- Manifestam como lentidão, não como comportamento errado.
- Abordagem: profiling → identificar hot spot → benchmarks antes/depois.

### 4.4. Bugs de memória
- Vazamentos, use-after-free (em contexto FFI/unsafe).
- Abordagem: Valgrind, AddressSanitizer, ou `cargo +nightly miri`.

## 5. Anti-padrões de debugging (PROIBIDOS)
- ❌ **Tentativa e erro cega:** Mudar código aleatoriamente até funcionar.
- ❌ **Print debugging permanente:** Deixar `println!` no código final.
- ❌ **Corrigir o sintoma:** "O valor está errado, vou colocar um if para corrigir."
- ❌ **Debugging por diff:** Comparar com versão anterior sem entender a causa.
- ❌ **Ignorar o bug:** "Acontece raramente, podemos conviver com isso."
- ❌ **Deletar o teste que falha:** O teste está certo — o código está errado.

## 6. Integração com outros workspaces
- **self-correction:** Debugging é o mecanismo formal de auto-correção.
- **evidence-based:** Debugging é o processo mais intenso de coleta de evidências.
- **testing-philosophy:** Todo bug corrigido gera um teste de regressão.
- **failure-analysis:** Bugs encontrados alimentam a análise de modos de falha.
