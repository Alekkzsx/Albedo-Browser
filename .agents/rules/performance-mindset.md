---
trigger: always_on
---

# Mentalidade de Performance

## 1. Princípio: Performance é uma feature, não uma otimização
Um browser que é lento é um browser que ninguém usa. Performance deve ser considerada **desde o design**, não adicionada depois. Mas: otimizar sem medir é otimizar errado.

## 2. Regra de Ouro: Medir Antes de Otimizar

### 2.1. NUNCA otimizar por intuição
- "Acho que isso é lento" NÃO é motivo para otimizar.
- **SEMPRE** medir com dados concretos antes de otimizar.
- Usar `criterion` para benchmarks, `perf` para profiling de sistema.
- Documentar: baseline → mudança → resultado.

### 2.2. Regra dos 80/20
- 80% do tempo de execução está em 20% do código.
- Identificar hot paths com profiling ANTES de otimizar.
- Não otimizar código que executa 1 vez por sessão — focar no que executa milhões.

## 3. Complexidade Algorítmica

### 3.1. Big-O consciente
- **SEMPRE** considerar a complexidade de tempo e espaço.
- Documentar a complexidade em funções que manipulam coleções grandes.
- Preferir algoritmos de menor complexidade quando a diferença é significativa.

### 3.2. Limites aceitáveis
| Operação | Complexidade Máxima |
|----------|-------------------|
| Lookup/busca | O(log n) ou O(1) |
| Iteração em lista | O(n) |
| Sorting | O(n log n) |
| Operações em DOM | Evitar O(n²) |
| String matching | O(n) amortizado |

## 4. Alocação de Memória

### 4.1. Stack vs Heap
- Preferir **stack allocation** para dados pequenos e de vida curta.
- Heap apenas quando necessário: dados grandes, tamanho dinâmico, lifetime longa.
- Usar `Box` com consciência — cada Box é uma alocação no heap.

### 4.2. Zero-copy
- Preferir `&str` sobre `String` quando possível.
- Preferir `&[u8]` sobre `Vec<u8>` quando possível.
- Preferir `Cow<'_, str>` quando pode ser referência ou owned.
- Usar `bytes::Bytes` para dados de rede (reference-counted, zero-copy slicing).

### 4.3. Pré-alocação
- `Vec::with_capacity()` quando o tamanho é conhecido ou estimável.
- `String::with_capacity()` para strings em construção.
- `HashMap::with_capacity_and_hasher()` para maps grandes.
- Reutilizar buffers com `clear()` em vez de alocar novos.

## 5. Concorrência e Paralelismo

### 5.1. I/O-bound vs CPU-bound
- **I/O-bound:** Usar `tokio` (async runtime). Exemplos: rede, filesystem, DNS.
- **CPU-bound:** Usar `rayon` ou `std::thread`. Exemplos: parsing, renderização, JIT.
- **NUNCA** misturar: não fazer CPU-bound work no tokio runtime.

### 5.2. Minimizar contenção
- Locks de menor granularidade possível.
- `RwLock` para leitura frequente, escrita rara.
- Dados imutáveis com `Arc<T>` — zero contenção em leitura.
- Lock-free data structures quando a contenção é alta.

## 6. Lazy Evaluation
- Não computar o que não será usado.
- Usar iteradores lazy (`iter().filter().map()`) em vez de eagerly coletados.
- `std::lazy::LazyCell` e `once_cell::Lazy` para inicialização sob demanda.
- Defer computações pesadas até que o resultado seja realmente necessário.

## 7. Cache-Friendly Design
- Structs com campos acessados juntos devem estar próximos em memória.
- Preferir Arrays of Structs (AoS) → Struct of Arrays (SoA) para hot paths.
- Evitar indireção excessiva (ponteiros que saltam na memória).
- Considerar cache lines (64 bytes) no layout de dados críticos.

## 8. Hot Paths do AlbedoBrowser
Os seguintes paths devem ter benchmarks obrigatórios:
- **HTML Parser:** Tokenização e construção da árvore DOM.
- **CSS Engine:** Parsing, cascading, computed styles.
- **Layout Engine:** Cálculo de posição e dimensões.
- **Rendering Pipeline:** Rasterização e composição.
- **JIT Compiler:** Compilação e execução de bytecode.
- **Networking:** Resolução DNS, conexão, download.

## 9. Anti-padrões de performance (PROIBIDOS)
- ❌ **Otimização prematura:** Otimizar sem profiling.
- ❌ **Micro-otimização:** Otimizar instruções individuais ignorando algoritmo.
- ❌ **Alocação em loop:** `String::new()` ou `Vec::new()` dentro de loop quente.
- ❌ **Clone em hot path:** `clone()` em dados grandes em código que executa muito.
- ❌ **Blocking no async:** Operação síncrona dentro de contexto async.
- ❌ **Busca linear em coleção grande:** `Vec::contains()` com milhares de itens.

## 10. Integração com outros workspaces
- **code-quality:** Performance é um aspecto da qualidade do código.
- **evidence-based:** Benchmarks são evidências — não otimizar sem eles.
- **failure-analysis:** Degradação de performance é um modo de falha.
- **smart-automation:** Benchmarks devem fazer parte do ciclo de verificação.
