---
trigger: always_on
---

# Padrões Absolutos de Qualidade de Código (Rust)

## 1. Princípio: Código de produção é código de elite
Todo código produzido deve atender aos mais altos padrões de qualidade. Não existe "bom o suficiente" — existe "correto, seguro, legível e performático". Cada linha de código é uma decisão de engenharia que deve resistir ao escrutínio.

## 2. Padrões Zero-Tolerance

### 2.1. Zero `unsafe` sem justificativa
- `unsafe` é **PROIBIDO** sem documentação explícita e aprovação do usuário.
- Se necessário, criar um KI documentando:
  - **Por que** `unsafe` é necessário (sem alternativa safe).
  - **Quais** invariantes devem ser mantidas.
  - **Como** a segurança é garantida manualmente.
- Todo bloco `unsafe` deve ter um `// SAFETY:` comment explicando a garantia.

### 2.2. Zero `clone()` desnecessário
- Preferir referências (`&T`, `&mut T`) sobre owned values.
- Preferir `Cow<'_, T>` quando pode ser referência ou owned.
- `clone()` permitido apenas para:
  - Tipos `Copy` (integers, bools, etc.).
  - Strings curtas em contexto não-hot-path.
  - Quando o borrow checker exige e não há alternativa sem refactoring maior.
- Documentar todo `clone()` com `// CLONE: motivo`.

### 2.3. Zero blocking na main thread
- I/O, decodificação, parsing pesado: **SEMPRE** em task/thread separada.
- UI thread e render thread: apenas lógica instantânea.
- Usar `tokio::spawn`, `rayon`, ou `std::thread` conforme o caso:
  - **I/O-bound:** `tokio` (async).
  - **CPU-bound:** `rayon` ou `std::thread`.

## 3. Ferramentas Obrigatórias

### 3.1. Ciclo de verificação
Após TODA alteração, executar na ordem:
1. `cargo fmt` — Formatar código.
2. `cargo check` — Verificar compilação (zero warnings).
3. `cargo clippy -- -D warnings` — Lint com warnings como erros.
4. `cargo test` — Executar testes (todos devem passar).

### 3.2. Tratamento de warnings
- **ZERO warnings** em `cargo check`. Todo warning é um bug potencial.
- **ZERO warnings** em `cargo clippy`. Clippy conhece padrões que humanos perdem.
- Se um warning de clippy é falso positivo, usar `#[allow()]` com comentário explicativo.

## 4. Padrões de Código

### 4.1. Funções
- **Tamanho máximo:** ~50 linhas por função. Se maior, decompor.
- **Parâmetros:** No máximo 4-5 parâmetros. Se mais, criar struct de configuração.
- **Retorno:** Sempre explícito. Evitar efeitos colaterais ocultos.
- **Naming:** Verbos para ações (`parse_html`, `render_frame`), substantivos para getters (`width()`, `children()`).

### 4.2. Structs e Enums
- **Documentação:** `///` obrigatório em todo item público.
- **Derive:** Implementar apenas os derives necessários (`Debug` quase sempre, `Clone` só se necessário).
- **Encapsulamento:** Campos devem ser `pub(crate)` por padrão, `pub` apenas se é parte da API pública.
- **Invariantes:** Documentar invariantes que devem ser mantidas.

### 4.3. Pattern Matching
- **Exaustivo:** Nunca usar wildcard `_` catch-all sem motivo documentado.
- Se novas variantes de enum podem surgir, usar `_` com `// TODO: handle new variants`.
- Preferir `match` sobre `if let` quando há mais de 2 branches.

### 4.4. Imports
- Ordenar: `std` → crates externas → `crate` → `self`/`super`.
- Evitar glob imports (`use module::*`) exceto em preludes.
- Preferir imports específicos para clareza.

## 5. `unwrap()` e `expect()`
- **`unwrap()` PROIBIDO em código de produção.** Permitido apenas em testes.
- **`expect("mensagem")`:** Permitido APENAS quando a situação é logicamente impossível (invariante comprovada). A mensagem deve explicar por que é impossível falhar.
- **Padrão preferido:** `?` com contexto (`map_err`, `context()`).
- **`todo!()`:** Permitido durante desenvolvimento, PROIBIDO em merge/entrega.

## 6. Métricas e Limites

| Métrica | Limite |
|---------|--------|
| Linhas por função | ≤ 50 |
| Parâmetros por função | ≤ 5 |
| Aninhamento máximo | ≤ 4 níveis |
| Complexidade ciclomática | ≤ 10 |
| Warnings (check + clippy) | = 0 |
| Cobertura de testes (novos módulos) | ≥ 80% |

## 7. Anti-padrões de qualidade (PROIBIDOS)
- ❌ **God struct:** Struct com 20+ campos e múltiplas responsabilidades.
- ❌ **String-typed programming:** Usar `String` quando um enum/newtype seria mais seguro.
- ❌ **Magic numbers:** Constantes sem nome. Usar `const` nomeada.
- ❌ **Dead code:** Código comentado ou funções não usadas. Remover.
- ❌ **Implicit conversions:** Preferir `From`/`Into` explícitos sobre casting.

## 8. Integração com outros workspaces
- **pattern-consistency:** Os padrões de qualidade complementam os padrões de consistência.
- **error-handling:** O tratamento de erros é um aspecto específico da qualidade.
- **smart-automation:** As ferramentas automatizadas verificam aderência aos padrões.
- **review-full:** A revisão valida a qualidade técnica.
