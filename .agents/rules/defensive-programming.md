---
trigger: always_on
---

# Programação Defensiva e Robustez

## 1. Princípio: Antecipar o inesperado
O agente deve programar como se todo input fosse hostil, todo recurso pudesse falhar, e todo estado pudesse ser corrompido. Programação defensiva não é paranoia — é engenharia responsável. O Rust ajuda com ownership e lifetimes, mas não cobre tudo.

## 2. Validação de Entrada

### 2.1. Trust Boundaries
- **Input externo** (rede, arquivos, user input): NUNCA confiar. Validar TUDO.
- **Input entre módulos**: Validar em boundaries de API pública. Confiar em APIs privadas.
- **FFI boundaries**: Tratar como se fosse input de rede — zero confiança.

### 2.2. Regras de validação
- Validar **antes** de usar — fail fast.
- Erros de validação devem ser descritivos: o que está errado, o que era esperado.
- Separar validação de lógica de negócio (camadas distintas).
- Para dados que podem ser inválidos: parse → validate → use (Type-Driven Development).

## 3. Tipos Fortes e Type Safety

### 3.1. Newtypes contra "primitive obsession"
```rust
// ❌ PROIBIDO — qualquer usize é aceito
fn get_user(id: usize) -> User { ... }

// ✅ CORRETO — tipo semântico
struct UserId(usize);
fn get_user(id: UserId) -> User { ... }
```

### 3.2. Type-State Pattern
Usar o sistema de tipos para tornar estados inválidos **irrepresentáveis**:
```rust
// Estados impossíveis são erros de compilação, não runtime
struct Connection<S: State> { ... }
struct Connecting;
struct Connected;
struct Disconnected;

impl Connection<Connected> {
    fn send(&self, data: &[u8]) -> Result<()> { ... }
}
// Connection<Disconnected>::send() não compila!
```

### 3.3. Enums over Booleans
- Preferir `enum Direction { Forward, Backward }` sobre `bool is_forward`.
- Preferir `enum Mode { Read, Write, ReadWrite }` sobre flags de bits.

## 4. Invariantes e Contratos

### 4.1. Assertions em debug
- Usar `debug_assert!()` para verificar invariantes em desenvolvimento.
- Documentar invariantes com comentários `// INVARIANT:`.
- Se uma invariante é violada em debug, é bug — corrigir imediatamente.

### 4.2. Pré-condições e pós-condições
- **Pré-condições:** O que deve ser verdade ANTES de chamar a função.
- **Pós-condições:** O que é garantido DEPOIS que a função retorna.
- Documentar ambas em funções críticas com `///` doc comments.

## 5. Limites e Bounds

### 5.1. Sempre verificar
- **Tamanhos de array/vec:** Verificar bounds antes de indexar.
- **Capacidades:** Verificar espaço antes de inserir.
- **Overflow:** Usar `checked_add`, `checked_mul` para aritmética que pode transbordar.
- **Recursão:** Limitar profundidade com contador explícito.

### 5.2. Timeouts
- **TODA** operação que pode bloquear deve ter timeout.
- Timeouts devem ser configuráveis, não hardcoded.
- Documentar o timeout padrão e o motivo.

## 6. Concorrência Defensiva
- Todo acesso a estado compartilhado DEVE usar sincronização (`Mutex`, `RwLock`).
- Preferir message passing (`mpsc`, `crossbeam`) sobre shared state.
- `parking_lot` sobre `std::sync` para primitivos mais eficientes.
- Locks devem ser mantidos pelo menor tempo possível.
- Ordem de aquisição de locks DEVE ser documentada para prevenir deadlocks.

## 7. Anti-padrões defensivos (PROIBIDOS)
- ❌ **Trust by default:** Aceitar input sem validar.
- ❌ **Index sem bound check:** `array[i]` sem garantia que `i < array.len()`.
- ❌ **Magic defaults:** Usar valores padrão sem documentar o motivo.
- ❌ **Silent truncation:** Truncar dados sem avisar.
- ❌ **Unbounded growth:** Coleções que crescem sem limite.
- ❌ **Stringly-typed:** Usar strings para representar estados, IDs ou tipos.

## 8. Integração com outros workspaces
- **error-handling:** Programação defensiva produz erros que devem ser bem tratados.
- **security-first:** Defesa contra entradas é a base da segurança.
- **code-quality:** Tipos fortes e validação melhoram a qualidade geral.
- **failure-analysis:** FMEA identifica onde defesas são mais necessárias.
