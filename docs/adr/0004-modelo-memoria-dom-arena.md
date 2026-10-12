# ADR-0004: Modelo de Memória do DOM baseado em Arena Geracional

- **Status:** Aceito & Implementado
- **Data:** 2026-08-16 (Implementado nas Fases 2 e 5)
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

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado em produção com 252 testes em `ace_dom` e 530 testes em `ace_core`.

- **`ace_core` (`Albedo_Core_Engine/ace_core/src/arena/`):**
  - Módulo `src/arena/slab.rs`: Implementação de `GenerationalSlab` e `ArenaId<T>`. Cada entrada possui um contador geracional incrementado a cada reciclagem de slot, prevenindo referências pendentes (*dangling IDs*) e o problema ABA.
  - Módulo `src/arena/id.rs`: Manipulação otimizada de chaves de nó com `NonZeroU64`.
  - Módulo `src/arena/bump.rs`: Alocação contígua em blocos de alta localidade de cache.
- **`ace_dom` (`Albedo_Core_Engine/ace_dom/src/node/` e `src/tree/`):**
  - Struct `NodeId`: Identificador opaco de 64 bits (`NonZeroU64`) que indexa os nós diretamente na arena do documento (`Document`).
  - Topologia da árvore: Relações hierárquicas (`parent`, `first_child`, `last_child`, `previous_sibling`, `next_sibling`) são armazenadas exclusivamente como `Option<NodeId>`, eliminando completamente ponteiros cíclicos `Rc<RefCell<Node>>`.
  - Reivindicação de época: Suporte ao rastreamento de nós ativos integrado com o ciclo de vida do documento em `ace_dom/src/gc.rs`.

