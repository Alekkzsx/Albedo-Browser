# 🛡️ Registro Central de Código `unsafe` — Albedo Browser

> **Documento de Auditoria e Conformidade de Segurança**  
> **Status:** Ativo & Auditado  
> **Última Atualização:** 2026-10-12 (Auditoria Forense e Sincronização do Workspace)  
> **Inventário:** Exatamente 66 ocorrências reais no workspace, 100% confinadas em 11 arquivos de `ace_core` (resolvendo a dívida DV-13 do [PLANO.md](../../PLANO.md))

---

## 📜 Histórico de Auditorias de `unsafe`

| Versão | Data | Contexto / Marco | Decisões & Escolhas Técnicas | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.2.0** | 2026-10-12 | Sincronização Forense do Workspace | • Retificação formal do inventário de 64 para **66 ocorrências reais** exclusivas de `ace_core`.<br>• Expurgados falsos positivos (`bump.rs` tem 0 unsafe, `generational.rs` não existe no caminho antigo, `simd.rs` em `ace_dom` usa APIs seguras da crate `memchr`).<br>• Confirmação de 0 blocos `unsafe` em todos os outros crates. | • Mapeamento detalhado dos 11 arquivos em `ace_core` (`thin_vec.rs`, `inline_vec.rs`, `intrusive_list.rs`, `triple_buffer.rs`, `gwp_asan.rs`, `tagged_ptr.rs`, `weak_ptr.rs`, `slab.rs`, `id.rs`, `percent.rs`, `string_builder.rs`).<br>• Documentação das garantias de compilador `#![forbid(unsafe_code)]`. |
| **1.1.0** | 2026-10-11 | Auditoria da Fase 6 (`ace_style`) | • Auditoria formal de integridade do motor de estilo.<br>• Confirmação de conformidade estrita com `#![forbid(unsafe_code)]`. | • Verificação de 0 blocos `unsafe` em todas as 5.675 linhas do `ace_style`.<br>• Registro do crate como 100% Safe Rust. |
| **1.0.0** | 2026-10-07 | Auditoria Forense v7.0 | • Eliminação do mito de "0 unsafe" no repositório.<br>• Mapeamento de construções inseguras de baixo nível.<br>• Obrigação formal de comentário `// SAFETY:` em cada bloco. | • Criação deste registro central com rastreabilidade de invariantes e validação Miri. |

---

## 1. Política Corporativa de `unsafe`

Conforme a **Seção §2.4 do PLANO.md**:

1. **Invariante Documentada:** Todo bloco, função ou impl `unsafe` DEVE conter um comentário obrigatório `// SAFETY:` detalhando:
   - Quais pré-condições devem ser mantidas pelo chamador.
   - Por que o acesso a ponteiros brutos ou operações de memória é seguro contra *data races*, *out-of-bounds* e *use-after-free*.
2. **Lints Obrigatórios de Compilador:**
   - `#![warn(clippy::undocumented_unsafe_blocks)]`
   - `#![warn(clippy::multiple_unsafe_ops_per_block)]`
   - `#![warn(unsafe_op_in_unsafe_fn)]`
3. **Crates com Garantia de Compilador `#![forbid(unsafe_code)]`:**
   Crates de alto nível que não manipulam memória física ou alocadores manuais impõem a proibição irrevogável no compilador:
   - `ace_ipc` (`src/lib.rs:6`)
   - `ace_browser` (`src/lib.rs:7`)
   - `ace_style` (`src/lib.rs:15`)
4. **Crates com 100% Safe Rust:**
   - `ace_dom`: **0 ocorrências de `unsafe`** (as menções textuais à palavra "unsafe" no crate restringem-se a valores de string de cabeçalhos CSP: `'unsafe-inline'` e `'unsafe-eval'`). O arquivo `src/tokenizer/simd.rs` consome exclusivamente funções seguras da crate `memchr`.
   - `ace_net`: **0 ocorrências de `unsafe`** (a pilha de rede opera integralmente sobre Safe Rust com `tokio`, `hyper` e `rustls`).
   - `ace_layout`: **0 ocorrências de `unsafe`**.
5. **Desmistificação de Falsos Positivos de Busca Textual:**
   - Em `ace_core/src/security/referrer.rs`, as correspondências textuais são literais da política HTTP Referrer `"unsafe-url"`.
   - Em `ace_dom/src/security/csp.rs` e `tests/csp_security_test.rs`, as correspondências são strings de diretivas CSP.
6. **Validação sob Miri e Loom:** Módulos contendo construções `unsafe` possuem testes unitários e de estresse executados sob o interpretador de comportamento indefinido **Miri** (`cargo miri test`) no pipeline de integração.

---

## 2. Inventário Exaustivo das 66 Ocorrências em `ace_core`

Todas as 66 ocorrências de código `unsafe` no repositório residem **estritamente em 11 arquivos do crate `ace_core`**:

### 2.1 Arenas e Gerenciamento de Identificadores (3 ocorrências)
- **`ace_core/src/arena/id.rs` (1 ocorrência — Linha 65):**
  - `NonZeroU64::new_unchecked(key)`: Conversão direta de inteiro para identificador não-nulo.
  - **Invariante Protegida:** O gerador monotônico de chaves da arena é inicializado em 1 e sempre incrementado; o valor nunca é zero.
- **`ace_core/src/arena/slab.rs` (2 ocorrências — Linhas 112, 126):**
  - Métodos `pub unsafe fn get_unchecked(&self, id)` e `pub unsafe fn get_unchecked_mut(&mut self, id)`.
  - **Invariante Protegida:** Acesso $O(1)$ sem checagem redundante de limites quando o chamador já validou a geração e a existência do slot na arena.

### 2.2 Estruturas de Dados de Alta Performance e Lock-Free (50 ocorrências)
- **`ace_core/src/collections/inline_vec.rs` (19 ocorrências — Linhas 34, 127, 156, 161, 176, 192, 195, 213, 243, 248, 258, 277, 289, 302, 367, 408, 413, 547, 581):**
  - Manipulação de `MaybeUninit<T>` para alocação em stack com transição dinâmica transparente para heap quando a capacidade inline é excedida.
  - **Invariante Protegida:** Alinhamento de memória adequado, contagem precisa de elementos inicializados e descarte manual correto via `assume_init_drop()` prevenindo vazamentos e *double-free*.
- **`ace_core/src/collections/thin_vec.rs` (17 ocorrências — Linhas 27, 28, 69, 74, 82, 117, 124, 129, 142, 155, 168, 182, 198, 214, 225, 250, 252):**
  - Implementação `unsafe impl Send/Sync` e vetor com cabeçalho embutido de ponteiro único (Thin Pointer de 8 bytes).
  - Alocação e desalocação manual com `std::alloc::{alloc, realloc}`, acesso aritmético a cabeçalho e dados, e reconstrução de fatias com `std::slice::from_raw_parts`.
  - **Invariante Protegida:** Layout de alocação rigorosamente alinhado, capacidade e tamanho verificados antes de desreferenciamento de ponteiros.
- **`ace_core/src/collections/intrusive_list.rs` (7 ocorrências — Linhas 51, 52, 124, 152, 176, 184, 200):**
  - Implementação `unsafe impl Send/Sync` e manipulação de nós duplamente encadeados sem alocações dinâmicas secundárias.
  - **Invariante Protegida:** Ponteiros brutos para nós anteriores e posteriores são válidos, não-nulos e mantidos em integridade durante inserção e remoção.
- **`ace_core/src/collections/triple_buffer.rs` (7 ocorrências — Linhas 23, 24, 47, 48, 108, 114, 193):**
  - Implementação `unsafe impl Send/Sync` e buffer triplo lock-free para desacoplamento de produtor e consumidor sem bloqueio de threads.
  - Troca atômica de slots através de `UnsafeCell` com semântica de ordenação atômica Acquire/Release.
  - **Invariante Protegida:** Produtor e consumidor nunca acessam o mesmo slot simultaneamente.

### 2.3 Gestão e Proteção de Memória (11 ocorrências)
- **`ace_core/src/memory/tagged_ptr.rs` (5 ocorrências — Linhas 33, 34, 69, 108, 112):**
  - Implementação `unsafe impl Send/Sync` e compactação de bits de tag nos 3 bits menos significativos de ponteiros alinhados a 8 bytes.
  - Uso de `NonNull::new_unchecked` após limpeza de bits de máscara.
  - **Invariante Protegida:** Alinhamento mínimo de 8 bytes do dado apontado garantindo que os 3 bits baixos sejam sempre zero antes da aplicação da tag.
- **`ace_core/src/memory/weak_ptr.rs` (4 ocorrências — Linhas 32, 33, 75, 173):**
  - Implementação `unsafe impl Send/Sync` e desreferenciamento de ponteiro `NonNull` protegido por bloco de controle geracional.
  - **Invariante Protegida:** Contagem de referências atômica previne desalocação prematura do bloco de controle; acesso ao dado requer validação de liveness.
- **`ace_core/src/memory/gwp_asan.rs` (2 ocorrências — Linhas 59, 71):**
  - Alocador com amostragem de páginas de guarda (GWP-ASan) para detecção de corrupção de memória em produção.
  - Alocação direta via `std::alloc::alloc` e posicionamento de buffers imediatamente adjacentes a páginas sentinela sem permissão de acesso.
  - **Invariante Protegida:** Layout de páginas de memória e offsets validados para capturar *Use-After-Free* e *Heap Buffer Overflow* instantaneamente via falha de página do SO.

### 2.4 Utilitários de Conversão de Texto e Rede (2 ocorrências)
- **`ace_core/src/net/percent.rs` (1 ocorrência — Linha 164):**
  - `String::from_utf8_unchecked(output)`: Construção de string a partir de buffer decodificado.
  - **Invariante Protegida:** O algoritmo de decodificação percentual valida exaustivamente cada sequência percentual (`%XX`), garantindo que apenas bytes UTF-8 válidos componham o vetor de saída.
- **`ace_core/src/text/string_builder.rs` (1 ocorrência — Linha 69):**
  - `std::str::from_utf8_unchecked(&buf[..len])`: Fatiamento de string a partir de buffer de caracteres.
  - **Invariante Protegida:** O buffer interno aceita exclusivamente caracteres ASCII e pontos de código UTF-8 válidos gravados sequencialmente.

---

## 3. Matriz Canônica de Auditoria e Cobertura Miri

| # | Arquivo Fonte em `ace_core` | Ocorrências | Linhas Mapeadas | Invariante Documentada | Validação Miri / Testes | Risco Arquitetural |
| :-: | :--- | :-: | :--- | :---: | :---: | :---: |
| **1** | `src/arena/id.rs` | 1 | 65 | ✅ `// SAFETY:` | ✅ `arena_test.rs` | Baixo |
| **2** | `src/arena/slab.rs` | 2 | 112, 126 | ✅ `// SAFETY:` | ✅ `generational_slab_test.rs` | Baixo |
| **3** | `src/collections/inline_vec.rs` | 19 | 34, 127, 156, 161, 176, 192, 195, 213, 243, 248, 258, 277, 289, 302, 367, 408, 413, 547, 581 | ✅ `// SAFETY:` | ✅ `inline_vec_test.rs` | Médio (Memória) |
| **4** | `src/collections/intrusive_list.rs` | 7 | 51, 52, 124, 152, 176, 184, 200 | ✅ `// SAFETY:` | ✅ `intrusive_list_test.rs` | Médio (Ponteiros) |
| **5** | `src/collections/thin_vec.rs` | 17 | 27, 28, 69, 74, 82, 117, 124, 129, 142, 155, 168, 182, 198, 214, 225, 250, 252 | ✅ `// SAFETY:` | ✅ `thin_vec_test.rs` | Médio (Alocador) |
| **6** | `src/collections/triple_buffer.rs` | 7 | 23, 24, 47, 48, 108, 114, 193 | ✅ `// SAFETY:` | ✅ `triple_buffer_test.rs` | Médio (Concorrência) |
| **7** | `src/memory/gwp_asan.rs` | 2 | 59, 71 | ✅ `// SAFETY:` | ✅ `gwp_asan_test.rs` | Baixo (Guarda) |
| **8** | `src/memory/tagged_ptr.rs` | 5 | 33, 34, 69, 108, 112 | ✅ `// SAFETY:` | ✅ `tagged_ptr_test.rs` | Baixo (Máscara) |
| **9** | `src/memory/weak_ptr.rs` | 4 | 32, 33, 75, 173 | ✅ `// SAFETY:` | ✅ `weak_ptr_test.rs` | Baixo (Controle) |
| **10** | `src/net/percent.rs` | 1 | 164 | ✅ `// SAFETY:` | ✅ `percent_test.rs` | Baixo (UTF-8) |
| **11** | `src/text/string_builder.rs` | 1 | 69 | ✅ `// SAFETY:` | ✅ `string_builder_test.rs` | Baixo (ASCII) |
| **Total** | **11 Arquivos** | **66** | — | **100% Coberto** | **Suíte Completa Aprovada** | **Auditado** |

---

## 4. Auditoria de Conformidade dos Demais Crates

| Crate | Ocorrências Reais de `unsafe` | Diretiva do Compilador | Justificativa Arquitetural |
| :--- | :---: | :---: | :--- |
| **`ace_dom`** | **0** | Safe Rust | Tokenizer e parser consomem apenas APIs seguras da crate `memchr`. Sem manipulação manual de ponteiros. |
| **`ace_net`** | **0** | Safe Rust | Pilha assíncrona baseada em `tokio`, `hyper` e `rustls` sem blocos inseguros. |
| **`ace_style`** | **0** | `#![forbid(unsafe_code)]` | Motor de estilos com árvore de nós computados protegida por `Arc<ComputedStyle>`. |
| **`ace_ipc`** | **0** | `#![forbid(unsafe_code)]` | Canais lock-free delimitados e roteamento pub/sub seguros sobre `crossbeam-channel`. |
| **`ace_browser`** | **0** | `#![forbid(unsafe_code)]` | Pipeline de orquestração do navegador operando 100% sobre Safe Rust. |
| **`ace_layout`** | **0** | Safe Rust | Geometria de caixas e fragmentos delegada a tipos seguros de `euclid` e `LayoutUnit`. |
