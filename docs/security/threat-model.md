# 🛡️ Modelo Formal de Ameaças (Threat Model) — Albedo Browser

> **Documento de Arquitetura de Segurança de Processos**  
> **Status:** Ativo & Vigente  
> **Classificação:** Defesa em Profundidade, Isolamento Multi-Processo e Integridade de Memória  
> **Alinhamento:** [PLANO.md](../../PLANO.md) — §4.4, §4.5, ADR-0005, ADR-0006, ADR-0009, ADR-0015 e Fase 12

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Técnicas | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.2.0** | 2026-10-12 | Sincronização do Workspace | • Incorporação de proteções de fronteira IPC e contenção de falhas (`ace_ipc`).<br>• Modelagem de guard pages GWP-ASan contra *Use-After-Free* (`ace_core`).<br>• Isolamento de rede RFC 9111 particionado e prevenção de inanição RFC 9218 (`ace_net`). | • Expansão das seções de isolamento IPC, mitigação de inanição de transporte e matriz STRIDE abrangente. |
| **1.1.0** | 2026-10-11 | Fechamento da Fase 6 (`ace_style`) | • Modelagem de ameaças contra o motor de estilos (vetores de DoS via CSS).<br>• Mitigação de ataques *Billion Laughs CSS* e ciclos em `var()`. | • Inclusão formal de limites de recursão do parser CSS e DAG de resolução de variáveis na matriz STRIDE. |
| **1.0.0** | 2026-10-07 | Formalização Arquitetural v7.0 | • Adoção de isolamento por site (*Site Isolation*) em substituição ao modelo antigo por aba (ADR-0015).<br>• Definição da fronteira do Network Service isolado.<br>• Modo *Jitless* como mitigação permanente contra corrupção de memória (ADR-0019). | • Criação deste documento de modelagem formal de ameaças. |

---

## 1. Fronteiras de Confiança e Topologia de Processos

O modelo de segurança do Albedo baseia-se na presunção de que **todo código e dado vindo da web é potencialmente malicioso**. O motor divide a execução em processos isolados do sistema operacional, estabelecendo barreiras físicas intransponíveis mediadas por protocolos binários tipados:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       PROCESSO DO BROWSER (Confiável)                       │
│  • Acesso a Sistema de Arquivos (Histórico, Bookmarks, Configurações)       │
│  • Gerenciamento de Janelas e Entrada de Usuário (winit)                    │
│  • Orquestrador Central e Supervisor de Ciclo de Vida dos Processos         │
└───────────────────────┬─────────────────────────────┬───────────────────────┘
                        │ IPC Tipado (ace_ipc)        │ IPC Tipado (ace_ipc)
                        │ Ring-Buffers Bounded        │ Ring-Buffers Bounded
                        v                             v
┌───────────────────────────────────┐     ┌───────────────────────────────────┐
│     NETWORK SERVICE (Semi-Conf.)  │     │   RENDERER PROCESS (Hostil/Não-C.)│
│  • Sockets TCP/UDP e TLS (rustls) │     │  • Parser HTML5 & Tree Builder    │
│  • Cache HTTP RFC 9111 Particionado│    │  • Motor de Estilos & Layout      │
│  • Scheduler RFC 9218 Anti-Starve │     │  • Interpretador JS (ace_js)      │
│  • Isolamento por NetworkIsolKey  │     │  • SANDBOX NATIVO DO SO (Sem I/O) │
│  • Cookie Jar Particionado (CHIPS)│     │  • ISOLADO POR SITE (eTLD+1)      │
└───────────────────────────────────┘     │  • PÁGINAS SENTINELA GWP-ASAN     │
                                          └───────────────────────────────────┘
```

---

## 2. Princípios de Isolamento e Defesa em Profundidade

### 2.1 Isolamento por Site (*Site Isolation* — ADR-0015)
- Em navegadores legados (isolamento restrito à aba), páginas em `attacker.com` embutidas em iframes dentro de `bank.com` compartilhavam o mesmo espaço de memória do processo.
- **No Albedo:** Cada *Site* canônico (`scheme + eTLD+1`) executa em seu próprio processo independente do sistema operacional. Se uma navegação mudar para uma origem cross-site, ocorre uma substituição completa de processo (*cross-origin process swap*).

### 2.2 Sandboxing de Sistema Operacional (Fase 4b / 12)
Os processos de **Renderer** são confinados sob as políticas mais restritivas suportadas pela plataforma:
- **Linux:** *Namespaces* de usuário e PID combinados com filtros restritivos via `seccomp-bpf`, permitindo exclusivamente leitura/escrita em file descriptors de IPC pré-estabelecidos e alocação anônima de memória (`mmap`).
- **Windows:** *AppContainers* e *Restricted Tokens* com `Integrity Level: Low`, bloqueio de I/O em arquivos locais e veto total à criação de processos filhos.
- **macOS:** *App Sandbox* com entitlements mínimos e isolamento de mach ports.

### 2.3 Fronteiras e Proteções do Barramento IPC (`ace_ipc` — ADR-0005)
A comunicação entre o Processo do Browser e os Renderers opera sob garantias estritas:
1. **Limites de Capacidade em Ring-Buffers Bounded:**
   - Os canais utilizam `bounded_channel(capacity)` sobre estruturas circulares lock-free.
   - **Mitigação contra Flooding / Memory Exhaustion DoS:** Se um processo Renderer corrompido ou malicioso tentar disparar mensagens infinitas para inundar a memória do Browser Process, o canal atinge a capacidade máxima e aplica *backpressure* imediato (`IpcError::Full` ou timeout), impedindo o colapso por exaustão de RAM.
2. **Roteador Pub/Sub com Validação Estrita de Tópicos (`IpcRouter`):**
   - Segregação mandatória de mensagens pelos tópicos temáticos `Navigation`, `Network`, `Dom`, `Render` e `Input`.
   - Impede injeção espúria de mensagens entre renderers distintos (*Cross-Site Message Injection / Topic Spoofing*).
3. **Contenção de Falhas e Pânico (*Crash & Panic Containment*):**
   - O Processo do Browser monitora a desconexão e terminação dos canais IPC (`IpcError::Disconnected`).
   - Se um Renderer entrar em pânico ou sofrer um encerramento forçado pelo SO, o Browser Process isola a falha, descarta a aba associada e recicla os recursos sem contaminação do processo central ou de outras abas ativas.

### 2.4 Proteções de Memória no Motor (`ace_core`)
Mesmo com o rigor de Safe Rust em 99%+ do código, o subsistema fundacional `ace_core` implementa salvaguardas ativas:
1. **GWP-ASan em Produção (`ace_core::memory::gwp_asan`):**
   - Alocador com amostragem de segurança que posiciona alocações selecionadas adjacentes a páginas sentinela sem permissão de leitura/escrita (*guard pages*).
   - **Detecção Instantânea:** Qualquer tentativa de *Use-After-Free* ou *Heap Buffer Overflow* dispara imediatamente uma falha de proteção de página de hardware (SIGSEGV/Access Violation), neutralizando a exploração antes que ocorra corrupção silenciosa de dados.
2. **Invariantes de TaggedPointer (`ace_core::memory::tagged_ptr`):**
   - Compactação de metadados em bits menos significativos restrita a alinhamento de 8 bytes, com máscara limpa antes de conversões para `NonNull`, eliminando ataques de confusão de tipos (*Type Confusion*).
3. **Arenas Geracionais EBR (`ace_core::arena::slab`):**
   - Identificadores de 64 bits (`ArenaId`) combinam índice e chave geracional monotônica. Tentativas de acessar nós desalocados ou reutilizados falham deterministicamente em $O(1)$, prevenindo ataques ABA.

### 2.5 Isolamento de Rede RFC 9111 e Prevenção de Inanição RFC 9218 (`ace_net`)
O subsistema de rede implementa isolamento multidimensional:
1. **Particionamento Triplo de Estado de Rede (`NetworkIsolationKey` — ADR-0009):**
   - Chave canônica `(TopLevelSite, FrameSite, is_cross_site)` aplicada a:
     - **Cache HTTP RFC 9111 (L1 RAM + L2 Disco WAL):** Elimina completamente ataques de canal lateral de temporização (*Cache Timing Attacks*) onde scripts em iframes inferem histórico de navegação medindo tempo de resposta de recursos locais.
     - **Pool de Conexões:** Impede reutilização de sockets TCP/TLS entre origens primárias distintas.
     - **Cookie Jar:** Suporte normativo a cookies particionados CHIPS (*Cookies Having Independent Partitioned State*).
2. **Escalonamento Prioritário Anti-Inanição RFC 9218:**
   - O `CriticalResourceScheduler` gerencia requisições com base em níveis de prioridade (`Urgent`, `High`, `Medium`, `Low`, `Lowest`).
   - **Prevenção de Inanição (*Starvation Prevention*):** Impede que sub-recursos massivos de origens terceiras monopolizem conexões ativas e causem inanição no carregamento do documento HTML principal e folhas de estilo CSS críticas.
3. **Acesso a Redes Privadas (PNA) e DNS Seguro:**
   - Bloqueio de travessia não autorizada da Internet para intranets locais (*Private Network Access*).
   - Consultas DNS encapsuladas em HTTPS com TLS 1.3 (DoH RFC 8484) prevenindo sequestro e envenenamento de DNS.

---

## 3. Matriz Expandida de Análise de Ameaças (STRIDE)

| Categoria STRIDE | Vetor de Ataque na Web / Motor | Mitigação Arquitetural e Tecnológica no Albedo | Subsistema Responsável |
| :--- | :--- | :--- | :---: |
| **Spoofing (Falsificação)** | Falsificação de origem em mensagens IPC, injeção cross-origin ou spoofing de pacotes DNS. | • Resolução estrita de origens W3C (RFC 6454).<br>• Segregação de canais no `IpcRouter` com tópicos validados.<br>• Resolução DoH (RFC 8484) com TLS 1.3 contra falsificação de DNS. | `ace_ipc`<br>`ace_net` |
| **Tampering (Adulteração)** | Injeção de scripts maliciosos (XSS), adulteração de pacotes de rede ou corrupção de mensagens IPC. | • Sanitizer API nativa e Content Security Policy Level 3.<br>• Subresource Integrity (SRI com SHA-256/384).<br>• Protocolo binário com enums fortemente tipados no IPC.<br>• WAL atômico em disco no cache L2 (RFC 9111). | `ace_dom`<br>`ace_net`<br>`ace_ipc` |
| **Repudiation (Repúdio)** | Reenvio malicioso de requisições, manipulação de transações ou desvio de protocolo. | • TLS 1.3 obrigatório via `rustls` (aws-lc-rs) com verificação WebPKI.<br>• Cabeçalhos HSTS com pré-carregamento.<br>• Canalização síncrona/ordenada em buffers circulares de IPC. | `ace_net`<br>`ace_ipc` |
| **Information Disclosure (Vazamento de Informação)** | Ataques de canal lateral de temporização (*Cache Timing Attacks*) e ataques microarquiteturais (Spectre). | • Particionamento triplo do cache HTTP e sockets via `NetworkIsolationKey`.<br>• Quantização determinística de tempo (`now_highres`) truncando timers de alta resolução.<br>• Isolamento físico de processos por site (eTLD+1). | `ace_net`<br>`ace_core`<br>`ace_browser` |
| **Denial of Service (DoS)** | Inundação de mensagens IPC (flooding), inanição de rede por terceiros, ou explosão recursiva de variáveis CSS (*Billion Laughs CSS*). | • Ring-buffers de capacidade limitada (`bounded_channel`) com backpressure no IPC.<br>• Agendador RFC 9218 prevenindo inanição de recursos vitais.<br>• Resolução de `var()` em Grafo Acíclico Dirigido (DAG) no `ace_style`.<br>• Limites de profundidade de nós e cotas de alocação de memória por processo. | `ace_ipc`<br>`ace_net`<br>`ace_style`<br>`ace_core` |
| **Elevation of Privilege (Elevação de Privilégio)** | Exploração de falhas de memória (*Use-After-Free*, *Buffer Overflow*) para escapar do Renderer e executar código no SO. | • **Modo Jitless permanente** (zero páginas de memória RWX executáveis).<br>• Páginas sentinela **GWP-ASan** detectando UAF instantaneamente.<br>• Sandboxing restritivo do SO (`seccomp-bpf` / AppContainer) proibindo criação de processos e I/O de disco.<br>• Contenção de falhas com descarte limpo de processos corrompidos. | `ace_core`<br>`ace_js`<br>`ace_browser` |

---

## 4. Diretrizes para Auditorias de Segurança

Toda nova funcionalidade introduzida no Albedo Core Engine deve submeter-se aos seguintes critérios de aceite de segurança:
1. **Regra de Safe Rust:** Novos crates ou módulos de alto nível devem declarar `#![forbid(unsafe_code)]` ou manter 0 ocorrências de código inseguro.
2. **Rastreabilidade de Invariantes:** Qualquer alteração em `ace_core` que envolva código `unsafe` deve ser registrada no [**`unsafe-registry.md`**](./unsafe-registry.md) com invariante `// SAFETY:` e teste sob Miri.
3. **Respeito às Fronteiras IPC:** Mensagens de protocolo em `ace_ipc` devem ser adicionadas exclusivamente ao enum mestre `IpcMessage` com tipos estáticos serializáveis.
4. **Isolamento de Estado:** Todo novo subsistema de armazenamento ou cache persistente deve indexar entradas obrigatoriamente por `NetworkIsolationKey`.
