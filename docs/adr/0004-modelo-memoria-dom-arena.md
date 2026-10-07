# ADR-0004: Modelo de Memória do DOM baseado em Arena Geracional

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_core`, `ace_dom`

---

## 1. Contexto e Problema

O DOM é um grafo arbitrariamente conectado e cíclico (nós apontam para pais, filhos, irmãos e listas de nós). Em Rust puro, representar grafos cíclicos com ponteiros inteligentes de contagem de referência (`Rc<RefCell<Node>>`) causa lentidão acentuada pelo custo atômico/overhead de contadores e resulta em vazamentos de memória crônicos por ciclos de referência não coletados.

## 2. Decisão

Forjamos uma **Arena Geracional com Epoch-Based Reclamation (EBR)** dentro de `ace_core::arena` e indexamos todos os nós do DOM através de identificadores estáticos de 64 bits (`NodeId`), eliminando o uso de contadores de referência para a topologia da árvore.

## 3. Consequências

- **Positivas:** Alocação de nós contígua e veloz, acesso O(1), cache locality incomparável e eliminação completa de vazamentos cíclicos.
- **Negativas / Riscos:** Exige que operações na árvore passem pela referência da Arena em vez de mutação direta sobre referências de nós isolados.

