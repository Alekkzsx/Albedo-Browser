---
trigger: always_on
---

# Tratamento de Erros Robusto e Semântico

## 1. Princípio: Erros são cidadãos de primeira classe
Tratamento de erros não é um afterthought — é uma parte **fundamental** do design. Cada caminho de erro deve ser tão bem pensado quanto o caminho feliz. Um sistema robusto é definido por como lida com falhas, não por como funciona quando tudo dá certo.

## 2. Hierarquia de Tipos de Erro

### 2.1. Camadas de erro
- **Erros de domínio:** Regras de negócio violadas (ex.: "URL inválida").
- **Erros de infraestrutura:** Falhas de I/O, rede, filesystem.
- **Erros de programação:** Bugs, invariantes violadas (devem ser `panic!` em debug).
- **Cada camada do sistema** deve ter seu próprio tipo de erro com `thiserror`.

### 2.2. `thiserror` vs `anyhow`
- **`thiserror`:** Para bibliotecas e APIs internas — erros tipados e compostos.
- **`anyhow`:** Apenas na camada de aplicação (main, CLI, handlers de entrada) — quando a causa específica não importa, apenas o contexto.
- **NUNCA** usar `anyhow` em módulos de biblioteca — o chamador perde informação.

## 3. Regras de Erro Obrigatórias

### 3.1. Propagação com contexto
```rust
// ❌ PROIBIDO — perde contexto
let data = fs::read(path)?;

// ✅ CORRETO — adiciona contexto
let data = fs::read(path)
    .map_err(|e| Error::FileRead { path: path.to_owned(), source: e })?;

// ✅ ALTERNATIVA com anyhow (apenas em camada de app)
let data = fs::read(path)
    .context(format!("Falha ao ler arquivo: {}", path.display()))?;
```

### 3.2. `unwrap()` e `expect()`
- **`unwrap()`** — PROIBIDO em produção. Apenas em testes.
- **`expect("reason")`** — Permitido APENAS para invariantes comprovadas. A mensagem deve explicar **por que é impossível falhar**.
- **`?`** — Operador padrão para propagação de erros.

### 3.3. `panic!` e `unreachable!`
- **`panic!`** — Reservado para bugs reais (invariantes violadas em debug).
- **`unreachable!`** — Apenas quando o compilador não consegue inferir que é inalcançável.
- **NUNCA** usar `panic!` para erros recuperáveis.

## 4. Padrões de Result e Option

### 4.1. Quando usar `Result<T, E>`
- Operações que podem falhar de formas previsíveis.
- I/O, parsing, validação, operações de rede.

### 4.2. Quando usar `Option<T>`
- Valores que podem legitimamente não existir (não é um erro).
- Buscas que podem não encontrar resultado.
- Documentar **quando e por que** o valor pode ser `None`.

### 4.3. Quando `panic!` é aceitável
- Violação de invariantes que indica bug no código.
- Estado corrupto irrecuperável.
- `debug_assert!` para verificações em modo de desenvolvimento.

## 5. Logging de Erros

### 5.1. Framework: `tracing`
- Usar `tracing` com spans e campos semânticos.
- Cada erro logado deve incluir:
  - **Nível:** `error`, `warn`, `info` conforme severidade.
  - **Contexto:** O que estava sendo feito quando falhou.
  - **Detalhes:** Informações suficientes para reproduzir/diagnosticar.

### 5.2. Níveis de log
| Nível | Quando usar |
|-------|------------|
| `error!` | Falha que afeta funcionalidade — requer atenção |
| `warn!` | Situação inesperada mas recuperável |
| `info!` | Eventos importantes do fluxo normal |
| `debug!` | Detalhes úteis para debugging |
| `trace!` | Dados granulares (desabilitado em produção) |

## 6. Graceful Degradation
- O sistema **NUNCA** deve crashar por falha recuperável.
- Se um recurso não está disponível, continuar com funcionalidade reduzida.
- Se um módulo falha, isolar a falha — não propagar para módulos saudáveis.
- Documentar o comportamento degradado esperado.

## 7. Anti-padrões de erro (PROIBIDOS)
- ❌ **Engolir erros:** `let _ = possibly_failing_function();`
- ❌ **String como tipo de erro:** `Result<T, String>`. Usar tipos tipados.
- ❌ **Erro genérico universal:** Um único `Error` para todo o projeto.
- ❌ **Panic em boundary:** Nunca `panic!` em código que recebe input externo.
- ❌ **Log e propaga:** Não logar o erro E propagar — escolher um.

## 8. Integração com outros workspaces
- **code-quality:** Error handling é um aspecto central da qualidade.
- **defensive-programming:** Defesa em profundidade complementa error handling.
- **failure-analysis:** FMEA identifica quais erros são possíveis.
- **security-first:** Erros não devem vazar informações sensíveis.
