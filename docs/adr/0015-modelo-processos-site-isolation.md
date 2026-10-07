# ADR-0015: Modelo de Processos com Isolamento por Site (Site Isolation)

- **Status:** Aceito
- **Data:** 2026-10-07
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** Arquitetura Global, `ace_ipc`, Fase 12

---

## 1. Contexto e Problema

O modelo legado de arquitetura de navegadores que isola processos apenas "por aba" permite que iframes de terceiros executem no mesmo espaço de memória do site primário, deixando dados confidenciais vulneráveis a vazamentos por vulnerabilidades de execução especulativa de CPU (Spectre).

## 2. Decisão

Adotamos formalmente o modelo moderno de **Isolamento por Site (*Site Isolation*)** como alvo da Fase 12 do Albedo. Cada *Site* canônico (`scheme + eTLD+1`) recebe um processo de Renderer isolado e restrito no sistema operacional. Navegações cross-site realizam *process swaps* obrigatórios.

## 3. Consequências

- **Positivas:** Padrão ouro de segurança da indústria, alinhado com Chrome e Firefox modernos; base obrigatória para suporte a `SharedArrayBuffer` com cabeçalhos COOP/COEP.
- **Negativas / Riscos:** Maior consumo de memória do SO por processo adicional e complexidade acrescida no orquestrador de janelas.

