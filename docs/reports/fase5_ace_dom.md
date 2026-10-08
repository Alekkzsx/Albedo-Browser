# 🧪 Relatório Técnico de Auditoria: Fase 5 (`ace_dom` & Parser HTML)

> **Subsistema Auditado:** `Albedo_Core_Engine/ace_dom`  
> **Data de Auditoria:** 2026-10-07  
> **Status da Fase 5:** 🟡 **Parcialmente Concluída** (Tokenizer FSM, Tree Builder com 16-AAA e Arena operacional com mais de 100 testes de integração aprovados; pendente vendorização da suíte oficial `html5lib-tests` conforme DV-11)  
> **Alinhamento:** [PLANO.md v7.0](../../PLANO.md) — §3 e §6 (Fase 5)

---

## 📜 Histórico de Revisões e Auditorias

| Versão | Data | Contexto / Marco | Decisões & Escolhas Técnicas | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.1.0** | 2026-10-07 | Auditoria e Refatoração Clean Code | • Desmembramento da God Function `process_token` (744 linhas) em métodos no padrão State.<br>• Reconhecimento formal da dívida DV-11 (ausência dos `.dat` oficiais do `html5lib`). | • Todos os testes de integração do `ace_dom` compilando e passando.<br>• Transição de status para 🟡 conforme a Regra de Evidência. |
| **1.0.0** | 2026-08-16 | Implementação Inicial do DOM | • Criação do Tokenizer WHATWG §12 com aceleração SIMD.<br>• Árvore de nós indexada em Arena SlotMap. | • Implementação base do parser e seletores CSS4. |

---

## 1. Resumo Executivo da Auditoria

O subsistema **`ace_dom`** atingiu maturidade técnica substancial no parseamento de HTML5 em streaming e na manipulação de grafos de nós complexos, cobrindo cenários adversários e mutações reentrantes.

| Métrica Verificada | Resultado Auditado | Status |
| :--- | :--- | :---: |
| **Binários de Teste de Integração (`tests/*.rs`)** | **35 arquivos de teste ativos** | ✅ Conforme |
| **Testes de Reentrância do Tokenizer** | **22 testes passando** (`tokenizer_reentrancy_test.rs`) | ✅ Conforme |
| **Testes Normativos de Texto e Range** | **19 testes passando** (`normative_text_operations_test.rs`) | ✅ Conforme |
| **Testes de Shadow DOM & Eventos** | **10 testes passando** (`shadow_dom_retargeting_test.rs`) | ✅ Conforme |
| **Suíte Oficial `html5lib-tests` (`.dat`)** | **0 arquivos `.dat` vendorizados no repositório** | 🟡 Dívida (DV-11) |
| **Localização do CSSOM** | **Residindo temporariamente em `ace_dom/src/cssom`** | 🟡 Dívida (DV-05) |

---

## 2. Mapa dos Subsistemas do DOM Validados

| Subsistema Validado | Suíte de Testes | Comportamento Verificado |
| :--- | :--- | :--- |
| **Tokenizer HTML5 & SIMD** | `simd_scanning_test.rs`, `streaming_tokenizer_test.rs` | Varredura de delimitadores acelerada por hardware e streaming de tokens compactos. |
| **Reentrância & `document.write`** | `tokenizer_reentrancy_test.rs` | Preservação de estado em inserções dinâmicas aninhadas com profundidade até 50. |
| **Tree Construction & AAA** | `tree_builder_test.rs`, `adoption_agency_test.rs` | Correção automática de tags não balanceadas e algoritmo da agência de adoção de 16 passos. |
| **Seletores CSS4 & Ancestor Filter** | `selector_sota_test.rs`, `bloom_dataset_events_test.rs` | Suporte a `:is()`, `:where()`, `:has()` e aceleração por Ancestor Bloom Filter de 64 buckets. |
| **MutationObserver & Live Ranges** | `mutation_observer_test.rs`, `live_range_test.rs` | Ajuste dinâmico automático de fronteiras de seleção e registro de mutações na árvore. |
| **Sanitizer API & Segurança** | `sanitizer_and_parser_test.rs`, `csp_security_test.rs` | Remoção ativa de vetores de injeção XSS e enforcement de diretivas de script inline. |

---

## 3. Dívidas Conhecidas para o Fechamento Total da Fase 5 (✅)

Para que a Fase 5 avance para **✅ Concluída**:

1. **Vendorização do `html5lib-tests` (DV-11):**
   - Baixar e incorporar os arquivos `.dat` oficiais do repositório oficial do consórcio W3C/WHATWG dentro de `ace_test_driver` ou `ace_dom/tests/data/`.
   - Executar o runner de conformidade e gerar um relatório percentual inquestionável de passagem nos subtestes de tokens e construção de árvore.
2. **Migração do CSSOM para `ace_style` (DV-05 / ADR-0012):**
   - Na Fase 6, desacoplar os arquivos em `ace_dom/src/cssom/` para o crate independente `ace_style`, mantendo o `ace_dom` focado exclusivamente na estrutura semântica da árvore.

