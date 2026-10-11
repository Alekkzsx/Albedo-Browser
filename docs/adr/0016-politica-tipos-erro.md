# ADR-0016: Política de Tipos de Erro no Workspace

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_core`, todos os crates de biblioteca e binários

---

## 1. Contexto e Problema

Em projetos de grande porte em Rust, o tratamento inconsistente de erros leva a problemas graves:
1. **Bibliotecas usando `anyhow::Result`:** Ocultam a identidade estruturada dos erros em ponteiros dinâmicos (`Box<dyn Error>`), impedindo que chamadores realizem *pattern matching* programático em falhas específicas (ex: distinguir erro de timeout de rede de erro de resolução DNS).
2. **Bibliotecas criando `enum Error` manuais sem derivações:** Resultam em código repetitivo, manutenção frágil e falta de compatibilidade ergonômica com `std::error::Error`.
3. **Binários e testes definindo enums desnecessários:** Aumentam a complexidade acidental onde apenas o contexto textual e backtraces são necessários para depuração e saída de console.

## 2. Decisão

Padronizamos formalmente a disciplina de tratamento de erros no workspace do Albedo Browser:

1. **Crates de Biblioteca (`ace_core`, `ace_dom`, `ace_net`, `ace_style`, etc.):**
   - Uso obrigatório de enums tipados fortemente derivados via [`thiserror`](https://docs.rs/thiserror).
   - Todo erro exportado publicamente deve implementar `std::error::Error` e prover descrições textuais claras via anotações `#[error("...")]`.
   - É **estritamente proibido** retornar `anyhow::Result` ou `Box<dyn Error>` em APIs públicas de bibliotecas.
2. **Crates Binários e Testes de Integração (`albedo_browser`, `tests/`):**
   - Uso recomendado de [`anyhow`](https://docs.rs/anyhow) para agregação de erros e anotação de contexto dinâmico via `.context("...")`.
3. **Pânico (`panic!`) Proibido no Pipeline:**
   - Erros operacionais (inputs malformados de HTML/CSS/JS, respostas corrompidas de rede, timeouts) devem retornar `Result::Err` ou degradar de forma resiliente, nunca induzir pânico na thread do motor.

## 3. Consequências

- **Positivas:**
  - Interfaces públicas de bibliotecas robustas, previsíveis e com checagem estrita de tipos pelo compilador.
  - Ergonomia de desenvolvimento uniforme em todos os crates do monorepo.
  - Impossibilidade de quebrar clientes de biblioteca por encapsulamento dinâmico opaco de erros.
- **Negativas / Riscos:**
  - Exige declaração explícita de variantes de erro e mapeamentos via `From`/`?` em transições entre submódulos.
