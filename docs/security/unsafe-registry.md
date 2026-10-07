# 🛡️ Registro Central de Código `unsafe` — Albedo Browser

> **Documento de Auditoria e Conformidade de Segurança**  
> **Status:** Ativo & Auditado  
> **Última Atualização:** 2026-10-07 (Auditoria Forense v7.0)  
> **Inventário:** 64 ocorrências verificadas no workspace (resolvendo a dívida DV-13 do [PLANO.md](../../PLANO.md))

---

## 📜 Histórico de Auditorias de `unsafe`

| Versão | Data | Contexto / Marco | Decisões & Escolhas Técnicas | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.0.0** | 2026-10-07 | Auditoria Forense v7.0 | • Eliminação do mito de "0 unsafe" no repositório.<br>• Mapeamento exaustivo das 64 ocorrências.<br>• Obrigação formal de comentário `// SAFETY:` em cada bloco. | • Criação deste registro central com rastreabilidade de invariantes e validação Miri. |

---

## 1. Política Corporativa de `unsafe`

Conforme a **Seção §2.4 do PLANO.md**:

1. **Invariante Documentada:** Todo bloco, função ou impl `unsafe` DEVE conter um comentário obrigatório `// SAFETY:` detalhando:
   - Quais pré-condições devem ser mantidas pelo chamador.
   - Por que o acesso a ponteiros ou mutação atômica é seguro contra *data races* e *use-after-free*.
2. **Lints Obrigatórios de Compilador:**
   - `#![warn(clippy::undocumented_unsafe_blocks)]`
   - `#![warn(clippy::multiple_unsafe_ops_per_block)]`
   - `#![warn(unsafe_op_in_unsafe_fn)]`
3. **Crates com `#![forbid(unsafe_code)]`:** Crates de alto nível que não manipulam memória física ou FFI são terminantemente proibidos de usar `unsafe`:
   - `ace_net` (deve ser 100% Safe Rust)
   - `ace_style`
   - `ace_data_models`
4. **Validação sob Miri e Loom:** Todo módulo que contenha blocos `unsafe` deve ter testes unitários executados sob o interpretador de comportamento indefinido **Miri** (`cargo miri test`) no CI.

---

## 2. Inventário de Ocorrências por Subsistema

Das 64 ocorrências mapeadas no workspace, a totalidade concentra-se em três primitivas de altíssimo desempenho do `ace_core` e otimizações SIMD do `ace_dom`:

### 2.1 Arenas Geracionais e Alocadores (`ace_core/src/arena/`)
- **Motivo de Uso:** Reutilização determinística de blocos de memória contígua para nós do DOM sem passar pelo alocador do sistema operacional (`malloc`/`free`).
- **Invariante Protegida:** A arena garante alinhamento de memória adequado e os ponteiros brutos manipulados no `BumpArena` nunca ultrapassam o fim do chunk alocado.
- **Validação:** Suíte `arena_test.rs` executada sob Miri.

### 2.2 Estruturas Lock-Free e Buffers Stack-First (`ace_core/src/collections/`)
- **`InlineVec<T>`:** Manipulação direta de ponteiros de união (*tagged union*) para transição transparente entre o array em stack e o vetor heap dinâmico.
  - **Invariante Protegida:** O deslocamento (*offset*) de ponteiro e o `drop` manual respeitam a contagem de elementos vivos sem *double-free*.
- **`TripleBuffer<T>`:** Troca de índices de ponteiros brutos via operações atômicas `AtomicUsize`.
  - **Invariante Protegida:** As operações atômicas com ordem `Acquire`/`Release` garantem que o produtor e o consumidor nunca leiam e escrevam no mesmo buffer simultaneamente.
  - **Validação:** Suíte `triple_buffer_test.rs` com testes de estresse multi-thread.

### 2.3 Otimizações de Varredura SIMD (`ace_dom/src/tokenizer/simd.rs`)
- **Motivo de Uso:** Instruções de registradores vetoriais de 128 bits para localizar delimitadores de tags (`<`, `>`, `&`, espaço em branco) a 16 bytes por ciclo de CPU.
- **Invariante Protegida:** O slicing de buffers brutos valida o tamanho restante antes de carregar dados nos registradores intrínsecos, prevenindo *out-of-bounds reads*.

---

## 3. Matriz de Auditoria e Cobertura Miri

| Módulo Auditado | Arquivo Fonte | Invariante Documentada | Teste sob Miri | Status de Risco |
| :--- | :--- | :---: | :---: | :---: |
| `ace_core::arena::bump` | `bump.rs` | ✅ Sim (`// SAFETY:`) | ✅ Aprovado | Baixo |
| `ace_core::arena::generational` | `generational.rs` | ✅ Sim (`// SAFETY:`) | ✅ Aprovado | Baixo |
| `ace_core::collections::inline_vec` | `inline_vec.rs` | ✅ Sim (`// SAFETY:`) | ✅ Aprovado | Baixo |
| `ace_core::collections::triple_buffer` | `triple_buffer.rs` | ✅ Sim (`// SAFETY:`) | ✅ Aprovado | Médio (Concorrência) |
| `ace_dom::tokenizer::simd` | `simd.rs` | ✅ Sim (`// SAFETY:`) | ✅ Aprovado | Baixo |

