# ADR-0007: GC Unificado JS ↔ DOM para Ciclos de Referência

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_dom`, `ace_js`, `ace_core`

---

## 1. Contexto e Problema

Em navegadores web modernos, o motor JavaScript (`ace_js`) e a árvore de nós do DOM (`ace_dom`) formam frequentemente ciclos de referências cruzadas:
- Um objeto JavaScript (ex: `let el = document.getElementById("btn")`) mantém uma referência para um nó na arena DOM.
- O elemento DOM, por meio de event listeners ou propriedades expostas (ex: `el.onclick = () => { ... }`), mantém uma referência forte para uma closure ou objeto JavaScript.

Se o Garbage Collector (GC) da máquina virtual JavaScript e o gerenciador de memória da Arena do DOM operarem de maneira isolada sem consciência mútua, esses ciclos cruzados nunca serão coletados, resultando em vazamentos de memória graves (*memory leaks*) que degradam o consumo de RAM em Single Page Applications (SPAs).

## 2. Decisão

Adotamos a arquitetura de **Rastreamento de Liveness Unificado com GC Tracing Bus** entre o `ace_js` e o `ace_dom`:

1. **Wrappers WebIDL com Identificadores Estáveis:** Todo nó do DOM referenciado por JavaScript possui um wrapper correspondente no heap do `ace_js`, contendo seu `NodeId` da arena geracional.
2. **Fase de Marcação Cruzada (Trace Phase):** Durante o ciclo de marcação (*Mark*) do GC do `ace_js`, qualquer nó do DOM referenciado direta ou indiretamente pela pilha ou heap JS é registrado em uma lista de raízes vivas (*Root Set*).
3. **Preservação de Subárvores:** A presença de um nó no *Root Set* do DOM mantém viva toda a sua linhagem de ancestrais e descendentes necessários para operações normativas de árvore.
4. **Coleta Sincronizada (Sweep Phase):** Quando nem o DOM nem o heap JS possuem referências para um nó desconectado da raiz, o nó é elegível para reciclagem na arena de nós via EBR (*Epoch-Based Reclamation*).

## 3. Consequências

- **Positivas:**
  - Eliminação de vazamentos de memória causados por ciclos bidirecionais entre DOM e JS.
  - Compatibilidade com o modelo de arenas geracionais sem necessidade de ponteiros `Rc` contados.
  - Suporte a coleta incremental de lixo sem paradas longas (*stop-the-world*) na thread principal.
- **Negativas / Riscos:**
  - Requer coordenação estrita e protocolo de handshake durante os ciclos de GC.
  - Pequeno overhead na fase de marcação para inspecionar nós ativos mantidos por closures de eventos.

