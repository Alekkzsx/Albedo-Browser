# 📊 Central de Relatórios e Auditorias — Albedo Core Engine

> **Diretório:** `docs/reports/`  
> **Status:** Ativo & Vigente  
> **Propósito:** Armazenar relatórios técnicos formais de auditoria, testes de estresse, benchmarks e comprovação de cumprimento dos Definition of Done (DoD) de cada fase do projeto.

---

## 📜 Histórico de Auditorias e Governança

| Versão | Data | Contexto / Marco | Decisões & Escolhas de Auditoria | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.0.0** | 2026-10-07 | Auditoria Forense v7.0 | • Aplicação irrestrita da Regra de Evidência (Regra de Ouro 5).<br>• Transição de status narrativos para auditorias comprovadas por testes executados no CI.<br>• Criação dos relatórios formais das Fases 3 e 5. | • Relatórios individuais criados para `ace_core` (Fase 2), `ace_net` (Fase 3) e `ace_dom` (Fase 5). |
| **1.0.0** | 2026-08-17 | Primeiro Relatório | • Auditoria inicial do `ace_core`. | • Publicação do relatório da Fase 2 com 82 testes. |

---

## 🧭 Catálogo de Relatórios Técnicos por Fase

| Subsistema / Fase | Documento de Relatório | Status Auditado | Testes Comprovados | Dívidas & Observações |
| :--- | :--- | :---: | :---: | :--- |
| **Fase 2 (`ace_core`)** | [**`fase2_ace_core.md`**](./fase2_ace_core.md) | 🟡 Parcial | **101 unit/integração + 3 doctests** (100% OK) | Pendente implementação do IPC in-process em `ace_ipc` (DV-01). |
| **Fase 3 (`ace_net`)** | [**`fase3_ace_net.md`**](./fase3_ace_net.md) | 🚧 Fechamento | **71 unitários + 17 arquivos de integração + estresse de 93s** | Cache RFC 9111, DoH, H2/H3 e threshold de 1MB aprovados. Em fechamento de PR. |
| **Fase 5 (`ace_dom`)** | [**`fase5_ace_dom.md`**](./fase5_ace_dom.md) | 🟡 Parcial | **22 testes de reentrância + dezenas de testes de Tree Builder** | Tokenizer FSM e AAA validados; pendente vendorização dos testes oficiais `html5lib-tests` (DV-11). |

---

## ⚖️ Critérios para Aprovação de um Relatório de Fase

Conforme a **Regra de Evidência (Regra de Ouro 5)** do [PLANO.md](../../PLANO.md):

1. **Caminho no Código:** Cada entrega deve estar materializada em arquivos rastreáveis no repositório.
2. **Bateria de Testes Automatizados:** O comportamento deve ser exercitado por testes que validem tanto o caminho feliz quanto casos de borda e regressão.
3. **CI Verde:** O commit que fecha a fase deve passar nos gates obrigatórios: `cargo fmt -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace` e `cargo deny check`.

