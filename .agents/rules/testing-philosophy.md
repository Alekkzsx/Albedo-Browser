---
trigger: always_on
---

# Filosofia de Testes: Quando, O Quê e Como

## 1. Princípio: Testes são cidadãos de primeira classe
Testes não são opcionais, não são um afterthought, e não são trabalho secundário. Código sem testes é código não verificado — e código não verificado é código com bugs desconhecidos. O nível de confiança em uma mudança é diretamente proporcional à qualidade dos testes.

## 2. Pirâmide de Testes

### 2.1. Distribuição ideal
```
        /  E2E  \        ← Poucos (caros, lentos)
       /Integration\     ← Alguns (fluxos entre módulos)
      / Unit Tests   \   ← Muitos (rápidos, isolados, focados)
```

### 2.2. Testes unitários (base da pirâmide)
- Testar **uma única unidade** de lógica (função, método, struct).
- Devem ser **rápidos** (< 100ms cada), **isolados** (sem deps externas) e **determinísticos**.
- Focar em: lógica de negócio, parsing, transformações, validações.

### 2.3. Testes de integração (meio)
- Testar **interação entre módulos**: DOM + CSS = Layout correto.
- Podem acessar filesystem, mas não rede/serviços externos.
- Focar em: fluxos end-to-end internos, compatibilidade entre componentes.

### 2.4. Testes E2E (topo)
- Testar o **sistema como um todo**: renderizar uma página HTML completa.
- Lentos e frágeis — usar com moderação, apenas para fluxos críticos.

## 3. Quando Escrever Testes

### 3.1. Obrigatório
- **Toda nova feature** deve ter testes unitários para a lógica central.
- **Todo bug fix** DEVE gerar um teste de regressão que falha sem o fix e passa com o fix.
- **Toda refatoração** deve ter testes existentes ANTES de refatorar (rede de segurança).
- **Toda mudança em API pública** deve ter testes de contrato.

### 3.2. Recomendado
- Property-based testing para funções puras com `proptest`.
- Fuzzing para parsers e código que lida com input não-confiável.
- Benchmarks para hot paths com `criterion`.

## 4. Como Escrever Bons Testes

### 4.1. Naming convention
```rust
#[test]
fn test_<behavior>_when_<condition>_then_<expected>() {
    // Arrange → Act → Assert
}

// Exemplos:
fn test_parse_url_when_valid_https_then_returns_url() { ... }
fn test_parse_url_when_missing_scheme_then_returns_error() { ... }
fn test_dom_insert_when_parent_not_found_then_returns_none() { ... }
```

### 4.2. Estrutura AAA (Arrange-Act-Assert)
```rust
#[test]
fn test_example() {
    // Arrange: preparar dados e dependências
    let input = "example.com";
    
    // Act: executar a operação sob teste
    let result = parse_url(input);
    
    // Assert: verificar o resultado
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::MissingScheme);
}
```

### 4.3. Um assert por conceito
- Cada teste verifica **um comportamento**.
- Múltiplos `assert!` são permitidos se verificam aspectos do mesmo comportamento.
- Se um teste falhar, deve ser imediatamente claro **o que está errado**.

### 4.4. Testes como documentação
- O nome do teste deve ser auto-explicativo — funciona como documentação.
- Se alguém lê os nomes dos testes, deve entender o que a função faz.

## 5. Infraestrutura de Testes

### 5.1. Fixtures e Builders
- Para structs complexas, criar **builder helpers** no módulo de teste.
- Reutilizar fixtures entre testes com funções helper.
- **NUNCA** copiar e colar setup entre testes — extrair para helper.

### 5.2. Mocking
- Usar traits para abstrair dependências externas.
- Mock apenas boundaries (I/O, rede, clock) — nunca lógica interna.
- Preferir fake implementations sobre frameworks de mock complexos.

## 6. Cobertura e Métricas

| Métrica | Meta |
|---------|------|
| Cobertura de novos módulos | ≥ 80% |
| Testes de regressão por bug | 100% (obrigatório) |
| Tempo de execução da suite | < 2 minutos |
| Testes flaky (intermitentes) | = 0 (eliminar imediatamente) |

## 7. Anti-padrões de teste (PROIBIDOS)
- ❌ **Test after code:** Escrever testes como obrigação após o código estar "pronto".
- ❌ **Assert-less tests:** Testes que apenas executam sem verificar nada.
- ❌ **Testes frágeis:** Testes que quebram com mudanças de implementação (não de comportamento).
- ❌ **God test:** Um teste que verifica 10 coisas diferentes.
- ❌ **Testes ignorados:** `#[ignore]` sem issue/justificativa associada.
- ❌ **Copy-paste test:** Duplicar testes com variações mínimas — usar parametrização.

## 8. Integração com outros workspaces
- **review-full:** Testes são o aspecto central da validação funcional.
- **self-correction:** Testes falhando é o trigger primário de auto-correção.
- **evidence-based:** Testes são a evidência mais forte de correção.
- **make-full:** Uma tarefa sem testes NÃO está completa.
