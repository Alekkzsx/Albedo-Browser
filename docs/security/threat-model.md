# 🛡️ Modelo Formal de Ameaças (Threat Model) — Albedo Browser

> **Documento de Arquitetura de Segurança de Processos**  
> **Status:** Ativo & Vigente  
> **Classificação:** Defesa em Profundidade e Isolamento Multi-Processo  
> **Alinhamento:** [PLANO.md v7.0](../../PLANO.md) — §4.4, §4.5 e Fase 12

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Técnicas | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.0.0** | 2026-10-07 | Formalização Arquitetural v7.0 | • Adoção de isolamento por site (*Site Isolation*) em substituição ao modelo antigo por aba (ADR-0015).<br>• Definição da fronteira do Network Service isolado.<br>• Modo *Jitless* como mitigação permanente contra corrupção de memória (ADR-0019). | • Criação deste documento de modelagem formal de ameaças. |

---

## 1. Fronteiras de Confiança e Topologia de Processos

O modelo de segurança do Albedo baseia-se na presunção de que **todo código vindo da web é potencialmente malicioso**. O motor divide a execução em processos isolados do sistema operacional, estabelecendo barreiras intransponíveis:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       PROCESSO DO BROWSER (Confiável)                       │
│  • Acesso a Sistema de Arquivos (Histórico, Bookmarks, Configurações)       │
│  • Gerenciamento de Janelas e Entrada de Usuário (winit)                    │
│  • Orquestrador de Processos e Despachante de IPC                           │
└───────────────────────┬─────────────────────────────┬───────────────────────┘
                        │ IPC Tipado Seguro           │ IPC Tipado Seguro
                        v                             v
┌───────────────────────────────────┐     ┌───────────────────────────────────┐
│     NETWORK SERVICE (Semi-Conf.)  │     │   RENDERER PROCESS (Hostil/Não-C.)│
│  • Sockets TCP/UDP e TLS (rustls) │     │  • Parser HTML5 & Tree Builder    │
│  • Cache HTTP RFC 9111 (L1/L2 WAL)│     │  • Motor de Estilos & Layout      │
│  • Cookie Jar & State Partitioning│     │  • Interpretador JS (ace_js)      │
│  • Isolamento por NetworkIsolKey  │     │  • SANDBOX NATIVO DO SO (Sem I/O) │
└───────────────────────────────────┘     │  • ISOLADO POR SITE (eTLD+1)      │
                                          └───────────────────────────────────┘
```

---

## 2. Princípios de Isolamento

### 2.1 Isolamento por Site (*Site Isolation* — ADR-0015)
- Em navegadores legados (isolamento apenas por aba), uma página em `attacker.com` aberta em um iframe dentro de `bank.com` compartilhava o mesmo espaço de endereçamento de memória do processo.
- **No Albedo:** Cada *Site* canônico (`scheme + eTLD+1`) recebe seu próprio processo de sistema operacional. Se uma aba navegar entre origens cross-site, ocorre uma troca completa de processo (*cross-origin process swap*).

### 2.2 Sandboxing de Sistema Operacional (Fase 4b / 12)
Os processos de **Renderer** são colocados sob as políticas mais restritivas que o sistema operacional suporta:
- **Windows:** *AppContainer* ou *Restricted Tokens* com `Integrity Level: Low`, proibição de leitura/escrita em qualquer arquivo do usuário e bloqueio de criação de novos processos filho.
- **Linux:** *Namespaces* de usuário e PID combinados com filtros de chamadas de sistema restritivos via `seccomp-bpf` (permitindo apenas leitura/escrita em file descriptors de IPC pré-abertos).
- **macOS:** *App Sandbox* com entitlements mínimos.

### 2.3 Particionamento de Estado de Rede (`NetworkIsolationKey` — ADR-0009)
O **Network Service** isola completamente pools de conexões e cache por uma chave composta `(TopLevelSite, FrameSite, is_cross_site)`:
- Impede ataques de canal lateral onde um atacante mede o tempo de resposta de um recurso para deduzir se o usuário já visitou determinado site.
- Suporte a cookies **CHIPS**, restringindo o envio de cookies de terceiros ao contexto primário onde foram gerados.

---

## 3. Análise de Ameaças STRIDE

| Categoria | Vetor de Ataque na Web | Mitigação Arquitetural no Albedo |
| :--- | :--- | :--- |
| **Spoofing (Falsificação)** | Falsificação de origem ou sequestro de sessão cross-origin. | Resolução estrita de origens RFC 6454 e tokens CSPRNG de 128 bits criptograficamente seguros (`UnguessableToken`). |
| **Tampering (Adulteração)** | Injeção de código XSS ou adulteração de recursos externos. | W3C HTML Sanitizer API nativa, Content Security Policy Level 3 e Subresource Integrity (SRI) com SHA-256/384. |
| **Repudiation (Repúdio)** | Injeção de transações ou desvio de protocolo. | TLS 1.3 obrigatório via `rustls` (aws-lc-rs) com verificação de certificados WebPKI e HSTS preloaded. |
| **Information Disclosure** | Ataques de canal lateral (Spectre) medindo nano-segundos de execução. | Quantização determinística de tempo (`now_highres`), truncando a resolução de temporizadores para faixas seguras. |
| **Denial of Service (DoS)** | Bomba de nós no DOM ou loops infinitos de microtarefas para congelar a máquina. | Limite estrito de profundidade de nós no parser, cotas de alocação de memória por processo e mitigação de starvation no Event Loop. |
| **Elevation of Privilege** | Exploração de corrupção de memória no motor JS para escapar para o SO. | **Modo Jitless permanente** (eliminando páginas de memória RWX) somado ao Sandbox de SO que bloqueia qualquer syscall não autorizada. |

