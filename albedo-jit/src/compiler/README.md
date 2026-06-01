# 📁 albedo-jit/src/compiler/

Esta pasta é o núcleo de compilação do **AlbedoJIT**, contendo a lógica de interpretação de bytecode AIR (Tier 0) e os compiladores de código de máquina nativo (Tiers 1 e 2) baseados na biblioteca **Cranelift**.

## 🎯 Objetivo & Função
Este diretório implementa a pipeline de otimização e geração de código nativo do compilador JIT. Ele traduz a representação intermediária baseada em registradores (AIR - Albedo Intermediate Representation) em assembly x86-64/ARM64, aplicando otimizações de nível de compilador (como análise de escape, substituição escalar de agregados, e otimizações de loops) com base no feedback de tipos coletado em tempo de execução.

## 📄 Arquivos e Suas Funções

| Arquivo | Função / Propósito |
| :--- | :--- |
| [air_interpreter.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/air_interpreter.rs) | **Interpretador AIR (Tier 0)**: Executa código frio em modo interpretado, conta iterações de laços e dispara a promoção de loops quentes para código JIT nativo via OSR. Também serve como o destino de *deoptimization*. |
| [baseline_compiler.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/baseline_compiler.rs) | **Compilador Baseline (Tier 1)**: Traduz o bytecode AIR de forma rápida e direta para funções nativas utilizando Cranelift, sem aplicar otimizações custosas. Ideal para código morno. |
| [tier2_compiler.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/tier2_compiler.rs) | **Compilador Otimizador AlbedoTurbo (Tier 2)**: Gera código nativo altamente otimizado por meio de especialização de tipos dinâmica e remoção de checagens redundantes de tags. |
| [loop_opts.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/loop_opts.rs) | **Otimizador de Loops**: Aplica transformações clássicas em laços de repetição (como hoisting de invariantes de loop e desenrolamento). |
| [escape_analysis.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/escape_analysis.rs) | **Análise de Escape**: Rastreia o ciclo de vida dos objetos JavaScript alocados na heap. Identifica objetos que nunca "escapam" da função atual para promovê-los à stack ou eliminá-los. |
| [scalar_replacement.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/scalar_replacement.rs) | **SROA (Scalar Replacement of Aggregates)**: Quebra objetos não-escapantes em variáveis locais individuais (escalares), eliminando completamente a alocação e reduzindo a pressão no Garbage Collector. |
| [stack_allocator.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/stack_allocator.rs) | **Alocador de Pilha**: Planeja a alocação física de objetos de ciclo de vida curto na pilha nativa da CPU para evitar chamadas de malloc/free de heap. |
| [deopt.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/deopt.rs) | **Deotimizador**: Trata o descarte seguro de quadros otimizados (Bailout) de volta para o interpretador de AIR caso alguma assunção dinâmica de tipo seja violada no código otimizado. |
| [code_cache.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/albedo-jit/src/compiler/code_cache.rs) | **Cache de Código**: Gerencia a persistência e reciclagem dos blobs de funções JIT compilados na memória de execução. |

## 🛠️ Regras de Design (O que DEVE e NÃO DEVE estar aqui)

### O que DEVE estar aqui:
- Lógica de geração de código nativo intermediada por APIs do Cranelift.
- Algoritmos tradicionais de análise de fluxo de controle (CFG) e otimizações SSA.
- Mecanismos de monitoramento de laços e transições dinâmicas de deopt/OSR.

### O que NÃO DEVE estar aqui:
- Lógica de parsing de strings JS e geração da representação AST original (ocorre no QuickJS).
- Definições da API DOM do browser ou manipulações da árvore do documento (devem ocorrer no motor ACE).
- Lógica de allocators brutos de memória de sistema (devem residir no módulo `infra/` do JIT).
