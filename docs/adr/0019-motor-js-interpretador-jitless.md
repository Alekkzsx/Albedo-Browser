# ADR-0019: Motor JavaScript: Interpretador de Bytecode no MVP e Modo Jitless Permanente

- **Status:** Aceito
- **Data:** 2026-10-07
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_js`, Fase 10

---

## 1. Contexto e Problema

Compiladores JIT (Just-In-Time) de JavaScript são responsáveis por mais de 50% de todas as vulnerabilidades de exploração zero-day (corrupção de memória e *type confusion*) em motores de navegadores modernos como V8 e JavaScriptCore. Além disso, escrever um compilador JIT confiável e seguro adiciona anos de esforço de engenharia impraticáveis para a versão 1.0.

## 2. Decisão

1. O motor `ace_js` no **MVP (v1.0)** será composto estritamente por um **interpretador de bytecode próprio**, sem compilador JIT.
2. O **Modo Jitless** permanecerá permanentemente disponível no navegador como um recurso de segurança reforçada (*Lockdown Mode*), mesmo quando o JIT for introduzido como otimização opcional pós-MVP na Fase 14.

## 3. Consequências

- **Positivas:** Redução monumental na superfície de ataque do navegador (zero páginas de memória RWX); entrega viável e mais rápida do motor JS próprio; menor consumo de memória RAM base.
- **Negativas / Riscos:** Desempenho computacional inferior em benchmarks sintéticos intensivos de CPU (Octane/Speedometer). Mitigado pelo fato de que páginas web comuns e SPAs gastam a maior parte do tempo em I/O de rede, mutações de DOM e layout, onde o interpretador é amplamente suficiente.

