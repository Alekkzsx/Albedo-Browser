# 🛡️ Central de Segurança — Albedo Browser & Engine

> **Diretório:** `docs/security/`  
> **Status:** Ativo & Vigente  
> **Propósito:** Centralizar a documentação de segurança ofensiva e defensiva do Albedo Browser, incluindo modelos formais de ameaças, inventário exaustivo de código `unsafe` e diretrizes de auditoria arquitetural.

---

## 📜 Histórico de Governança de Segurança

| Versão | Data | Contexto / Marco | Decisões & Escolhas de Segurança | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.2.0** | 2026-10-12 | Sincronização Forense do Workspace | • Retificação do inventário para exatamente **66 ocorrências reais de `unsafe`** em 11 arquivos de `ace_core`.<br>• Confirmação de 0 blocos `unsafe` em `ace_dom`, `ace_net`, `ace_style`, `ace_ipc` e `ace_browser`.<br>• Imposição formal de `#![forbid(unsafe_code)]` em `ace_ipc`, `ace_browser` e `ace_style`.<br>• Expansão do modelo STRIDE com limites IPC (`ace_ipc`), sentinelas GWP-ASan (`ace_core`) e isolamento RFC 9111/9218 (`ace_net`). | • Sincronização exaustiva do registro e modelo de ameaças com a verdade do código. |
| **1.1.0** | 2026-10-11 | Fechamento da Fase 6 (`ace_style`) | • Validação de 0 unsafe em `ace_style` com `#![forbid(unsafe_code)]`.<br>• Modelagem de mitigação contra vetores DoS de CSS (Billion Laughs / ciclos em `var()`). | • Atualização do registro central e modelo STRIDE. |
| **1.0.0** | 2026-10-07 | Auditoria e Formalização v7.0 | • Criação do Registro Central de Unsafe (`unsafe-registry.md`).<br>• Definição do Modelo Formal de Ameaças Multi-Processo (`threat-model.md`).<br>• Obrigação de testes sob Miri no CI para módulos com código de baixo nível. | • Centralização de segurança na pasta dedicada `docs/security/`. |

---

## 🧭 Documentos de Segurança

| Documento | Foco | O que Detalha |
| :--- | :--- | :--- |
| [**`unsafe-registry.md`**](./unsafe-registry.md) | **Registro Central de `unsafe`** | Inventário exaustivo das 66 ocorrências reais no workspace (100% confinadas em 11 arquivos de `ace_core`), detalhamento de invariantes `// SAFETY:`, comprovação de 0 unsafe nos demais crates e plano de cobertura sob Miri. |
| [**`threat-model.md`**](./threat-model.md) | **Modelo Formal de Ameaças** | Análise STRIDE expandida cobrindo as fronteiras de processos (*Site Isolation*), proteções de capacidade do barramento IPC com contenção de falhas (`ace_ipc`), detecção de UAF via páginas sentinela GWP-ASan (`ace_core`), e isolamento de rede RFC 9111 com prevenção de inanição RFC 9218 (`ace_net`). |

---

## 🔒 Princípios de Segurança Inegociáveis

1. **Princípio do Menor Privilégio & Sandboxing de SO:**
   - Todo código de renderização e execução de scripts da web executa em processos de Renderer sem privilégios de sistema operacional (sem acesso a arquivos locais, sem periféricos e sem criação de subprocessos, protegidos por `seccomp-bpf` no Linux e AppContainers no Windows).
2. **Defesa em Profundidade na Memória:**
   - Camada 1: *Type Safety* e garantias de propriedade do Rust em 99%+ do monorepo, com imposição de `#![forbid(unsafe_code)]` em `ace_ipc`, `ace_browser` e `ace_style`.
   - Camada 2: Rastreabilidade absoluta das 66 ocorrências de `unsafe` em `ace_core` com testes sob Miri.
   - Camada 3: Monitoramento em produção via **GWP-ASan** com páginas sentinela inacessíveis adjacentes a alocações selecionadas, provocando falha instantânea em caso de *Use-After-Free* ou *Heap Buffer Overflow*.
3. **Fronteiras Bounded e Contenção de Falhas no IPC (`ace_ipc`):**
   - Canais delimitados (`bounded_channel`) com capacidade finita contra exaustão de memória por flooding.
   - Pub/Sub multi-tópico com segregação temática prevenindo injeção de mensagens entre renderers.
   - Isolamento de falhas (*Crash Containment*): pânicos em um Renderer encerram a aba correspondente sem afetar o Browser Process ou outras abas.
4. **Isolamento Multidimensional de Rede (`ace_net`):**
   - Chave tripla `NetworkIsolationKey (TopLevelSite, FrameSite, is_cross_site)` particionando o Cache HTTP RFC 9111 (L1 RAM e L2 WAL em disco), pools de conexão TCP/TLS e o Cookie Jar (CHIPS).
   - Agendamento de requisições RFC 9218 com prevenção de inanição (*Starvation Prevention*) de recursos críticos do documento principal.
   - DNS criptografado DoH (RFC 8484) com transporte TLS 1.3 seguro via `rustls` e bloqueio de travessia de intranet via PNA.
5. **Modo *Jitless* Permanente:**
   - O motor JavaScript do Albedo opera em modo interpretado permanente (sem compilação JIT), eliminando radicalmente a existência de páginas de memória com permissão simultânea de escrita e execução (RWX), neutralizando a classe dominante de exploits zero-day modernos.
